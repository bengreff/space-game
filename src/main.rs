use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    event::{ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    window::WindowBuilder,
};

use sunscatter::editor::{
    render_editor_ui, EditorAction, generate_grid_vertices, generate_part_vertices,
    generate_ghost_vertices, screen_to_world, part_at_screen_pos, BodyInfo,
};
use egui;
use sunscatter::bodies::G;
use sunscatter::game::{Game, GameMode};
use sunscatter::render::{RenderState, MainMenuAction, PauseAction, OrbitRenderData, ShipRenderData, ShipOrbitData, ShipPartRenderData, OrbitSegmentData, SelectedTarget, StagedPartInfo, TargetPopup, Vertex, BodyInfoData};
use sunscatter::ship::{AutopilotTarget, ShipState, VesselPhysicsData, SHIP_SIZE, MAX_THRUST_ACCELERATION, AMBIENT_TEMPERATURE, RAILS_WARP_THRESHOLD};
use sunscatter::parts::default_heat_tolerance;

// 1:1 Real-Scale Solar System Simulation
// All physics use real-world values: masses, radii, distances, orbital velocities
// Rendering scale: 1 world unit = 1 billion meters (1e9 m)
const SCALE: f64 = 1e-9;

// Time warp levels (simulation seconds per real second)
const WARP_LEVELS: &[f64] = &[1.0, 2.0, 3.0, 5.0, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0, 100000000.0, 1000000000.0, 10000000000.0, 100000000000.0, 1000000000000.0];

// Visual scale factor for bodies (1.0 = real proportions, no artificial enlargement)
const BODY_SCALE: f64 = 1.0;

// Galaxy view threshold: screen spans 0.1 light-years or more
const GALAXY_VIEW_THRESHOLD_M: f64 = 0.1 * 9.461e15; // 0.1 light-years in meters

fn is_galaxy_view(camera_zoom: f32, _screen_height: u32) -> bool {
    let screen_span_m = 2.0 / (camera_zoom as f64 * SCALE);
    screen_span_m >= GALAXY_VIEW_THRESHOLD_M
}

fn main() {
    env_logger::init();

    println!("Sunscatter starting...");
    println!("Controls:");
    println!("  Escape: Pause / Menu");
    println!("  Left Shift / Left Ctrl: Throttle up/down");
    println!("  Z/X: Full/cut throttle");
    println!("  Q/E: Rotate left/right");
    println!("  WASD: RCS translation (when RCS enabled)");
    println!("  R: Toggle RCS");
    println!("  ` (backtick): Focus on ship");
    println!("  Left mouse drag: Pan camera");
    println!("  Scroll wheel: Zoom in/out");
    println!("  Double-click planet: Focus on it");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Sunscatter")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)
            .unwrap(),
    );

    let mut game = Game::new();
    let body_names: Vec<String> = game.solar_system.bodies.iter().map(|b| b.name.clone()).collect();
    let mut render_state = pollster::block_on(RenderState::new(window.clone(), &body_names));
    let mut last_frame = Instant::now();

    // Double-click detection
    let mut last_click_time: Option<Instant> = None;
    let mut last_click_pos: [f32; 2] = [0.0, 0.0];

    // Initial camera: focus on Sun, zoomed out to see all planets
    {
        let sun_pos = game.solar_system.body_position(1);
        render_state.camera.position[0] = sun_pos[0] * SCALE * BODY_SCALE;
        render_state.camera.position[1] = sun_pos[1] * SCALE * BODY_SCALE;
        render_state.camera.body_center = render_state.camera.position;
        render_state.camera.ship_offset = [0.0, 0.0];
        render_state.camera.zoom = 0.002; // Zoomed out to see full solar system
    }

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent { ref event, .. } => {
                    // Pass event to egui first
                    let egui_consumed = render_state.handle_event(event);

                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        WindowEvent::Resized(physical_size) => {
                            render_state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            // Calculate delta time
                            let now = Instant::now();
                            let dt = now.duration_since(last_frame).as_secs_f32();
                            last_frame = now;

                            // Update FPS (exponential moving average)
                            if dt > 0.0 {
                                let instant_fps = 1.0 / dt;
                                render_state.fps = render_state.fps * 0.95 + instant_fps * 0.05;
                            }

                            match game.mode {
                                GameMode::MainMenu => {
                                    render_main_menu_frame(
                                        &mut game,
                                        &mut render_state,
                                        elwt,
                                    );
                                }
                                GameMode::Flight => {
                                    render_flight_frame(
                                        &mut game,
                                        &mut render_state,
                                    );
                                }
                                GameMode::Editor => {
                                    render_editor_frame(
                                        &mut game,
                                        &mut render_state,
                                        dt,
                                    );
                                }
                                GameMode::TrackingStation => {
                                    render_tracking_station_frame(
                                        &mut game,
                                        &mut render_state,
                                    );
                                }
                            }
                        }

                        WindowEvent::MouseInput { state, button, .. } => {
                            match game.mode {
                                GameMode::MainMenu => {
                                    // egui-only handling (buttons in menu)
                                }
                                GameMode::Flight => {
                                    handle_flight_mouse_input(
                                        &mut game,
                                        &mut render_state,
                                        *state,
                                        *button,
                                        egui_consumed,
                                        &mut last_click_time,
                                        &mut last_click_pos,
                                    );
                                }
                                GameMode::Editor => {
                                    handle_editor_mouse_input(
                                        &mut game,
                                        &mut render_state,
                                        *state,
                                        *button,
                                        egui_consumed,
                                    );
                                }
                                GameMode::TrackingStation => {
                                    handle_tracking_station_mouse_input(
                                        &mut game,
                                        &mut render_state,
                                        *state,
                                        *button,
                                        egui_consumed,
                                        &mut last_click_time,
                                        &mut last_click_pos,
                                    );
                                }
                            }
                        }

                        WindowEvent::CursorMoved { position, .. } => {
                            let x = position.x as f32;
                            let y = position.y as f32;

                            match game.mode {
                                GameMode::MainMenu => {
                                    render_state.camera.last_mouse_pos = [x, y];
                                }
                                GameMode::Flight => {
                                    handle_flight_cursor_moved(
                                        &mut render_state,
                                        x, y,
                                        egui_consumed,
                                    );
                                }
                                GameMode::Editor => {
                                    handle_editor_cursor_moved(
                                        &mut game,
                                        &mut render_state,
                                        x, y,
                                        egui_consumed,
                                    );
                                }
                                GameMode::TrackingStation => {
                                    handle_tracking_station_cursor_moved(
                                        &mut render_state,
                                        x, y,
                                        egui_consumed,
                                    );
                                }
                            }
                        }

                        WindowEvent::MouseWheel { delta, .. } => {
                            if !egui_consumed {
                                let scroll_amount = match delta {
                                    MouseScrollDelta::LineDelta(_, y) => *y,
                                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                                };
                                let zoom_factor = 1.0 + scroll_amount * 0.1;

                                match game.mode {
                                    GameMode::Flight | GameMode::TrackingStation => {
                                        render_state.camera.zoom_by(zoom_factor);
                                    }
                                    GameMode::MainMenu => {
                                        // Camera is locked at fixed zoom in main menu
                                    }
                                    GameMode::Editor => {
                                        game.editor.zoom_camera(zoom_factor);
                                    }
                                }
                            }
                        }

                        WindowEvent::KeyboardInput {
                            event: KeyEvent {
                                logical_key,
                                state,
                                ..
                            },
                            ..
                        } => {
                            let pressed = *state == ElementState::Pressed;

                            // Universal Escape: toggle pause in all modes
                            // In editor: deselect first, then pause on next press
                            let escape_pressed = pressed && matches!(logical_key, Key::Named(winit::keyboard::NamedKey::Escape));
                            if escape_pressed && game.mode == GameMode::Editor
                                && game.editor.fairing_build_mode.is_some()
                            {
                                game.editor.exit_fairing_build_mode();
                            } else if escape_pressed && game.mode == GameMode::Editor
                                && (game.editor.selected_part_def.is_some() || game.editor.selected_placed_part.is_some())
                            {
                                game.editor.deselect();
                            } else if escape_pressed {
                                game.toggle_pause();
                            } else if !game.paused {
                                match game.mode {
                                    GameMode::MainMenu | GameMode::TrackingStation => {
                                        // No keyboard shortcuts in these modes
                                    }
                                    GameMode::Flight => {
                                        handle_flight_keyboard(
                                            &mut game,
                                            &mut render_state,
                                            logical_key,
                                            pressed,
                                        );
                                    }
                                    GameMode::Editor => {
                                        handle_editor_keyboard(
                                            &mut game,
                                            logical_key,
                                            pressed,
                                            egui_consumed,
                                        );
                                    }
                                }
                            }
                        }

                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}

/// Render a flight mode frame
fn render_flight_frame(
    game: &mut Game,
    render_state: &mut RenderState,
) {
    let dt = 1.0 / 60.0; // Approximate for now, actual dt passed separately

    // Power system state (updated each frame, read when building render data)
    let mut power_generation = 0.0_f64;
    let mut power_consumption = 0.0_f64;

    // --- Simulation (skipped when paused) ---
    let vessel_physics = if !game.paused {
        // Force warp to 1x if ship is below landing altitude at on-rails warp speeds (>10x)
        // Only when flying — landed ships can warp at any speed (update_landed is analytical)
        if game.flight.ship.below_landing_altitude(&game.solar_system)
            && matches!(game.flight.ship.state, ShipState::Flying)
        {
            let warp = WARP_LEVELS[game.warp_index];
            if warp > RAILS_WARP_THRESHOLD {
                game.warp_index = 0;
            }
        }

        // Update simulation with current time warp
        let time_warp = WARP_LEVELS[game.warp_index];

        // Auto-disable RCS when entering on-rails warp, re-enable when returning to physics warp
        let on_rails_warp = time_warp > RAILS_WARP_THRESHOLD;
        if on_rails_warp && render_state.rcs_enabled {
            render_state.rcs_enabled = false;
            render_state.rcs_disabled_by_rails = true;
        } else if !on_rails_warp && render_state.rcs_disabled_by_rails {
            render_state.rcs_enabled = true;
            render_state.rcs_disabled_by_rails = false;
        }

        game.solar_system.update(dt * time_warp);
        game.simulation_time += dt * time_warp;

        // Determine autopilot state and desired direction (before gimbal update)
        let autopilot_target = render_state.get_autopilot_target();
        let autopilot_active = autopilot_target != AutopilotTarget::Off && !game.flight.ship.on_rails;
        let autopilot_target_angle = if autopilot_active {
            if autopilot_target == AutopilotTarget::Target {
                // Target angle is precomputed by the render loop
                render_state.selected_target_angle
            } else {
                let maneuver_node = render_state.get_selected_maneuver_node();
                game.flight.ship.autopilot_target_angle(autopilot_target, maneuver_node)
            }
        } else {
            None
        };

        // Update engine gimbal angles: driven by autopilot when SAS active, else by A/D input
        if let Some(ref mut vessel) = game.flight.vessel {
            let (rotate_left, rotate_right) = if let Some(target_angle) = autopilot_target_angle {
                // Autopilot commands gimbals based on desired rotation direction
                let dir = game.flight.ship.autopilot_desired_direction(target_angle, None);
                (dir > 0.0, dir < 0.0)
            } else {
                (game.flight.ship_input.rotate_left, game.flight.ship_input.rotate_right)
            };
            vessel.update_gimbal(rotate_left, rotate_right);
        }

        // Build VesselPhysicsData from flight vessel if available
        // Uses active_thrust which excludes engines without fuel
        let rcs_enabled = render_state.rcs_enabled;
        let vessel_physics = game.flight.vessel.as_ref().map(|v| VesselPhysicsData {
            total_mass: v.total_mass,
            max_thrust_vac: v.active_thrust_vac(),
            max_thrust_asl: v.active_thrust_asl(),
            vessel_height: v.bounding_half_height(),
            bottom_extent: v.bottom_extent(),
            moment_of_inertia: v.moment_of_inertia,
            rcs_torque: if rcs_enabled { v.compute_rcs_torque(&game.part_definitions) } else { 0.0 },
            gimbal_torque: v.compute_gimbal_torque(),
            vessel_half_width: v.bounding_half_width(),
            rcs_translation_force: if rcs_enabled { v.compute_rcs_translation_force(&game.part_definitions) } else { 0.0 },
        });

        // Compute RCS translation from input (only when RCS enabled)
        game.flight.ship.rcs_translate = if rcs_enabled {
            let fwd = if game.flight.ship_input.translate_forward { 1.0 } else { 0.0 }
                    - if game.flight.ship_input.translate_backward { 1.0 } else { 0.0 };
            let right = if game.flight.ship_input.translate_right { 1.0 } else { 0.0 }
                      - if game.flight.ship_input.translate_left { 1.0 } else { 0.0 };
            [fwd, right]
        } else {
            [0.0, 0.0]
        };

        // Update ship physics (gimbal torque always applied in update_flying)
        let has_flight_vessel = game.flight.vessel.is_some();
        game.flight.ship.update(dt * time_warp, time_warp, &game.flight.ship_input, &game.solar_system, vessel_physics.as_ref(), autopilot_active, has_flight_vessel);
        if let Some(target_angle) = autopilot_target_angle {
            game.flight.ship.autopilot_rotate(target_angle, dt * time_warp, vessel_physics.as_ref());
        }

        // Fuel consumption and vessel sync
        if let Some(ref mut vessel) = game.flight.vessel {
            vessel.throttle = game.flight.ship.throttle;

            // Compute atmospheric pressure fraction for engine ISP interpolation
            let atmo_pressure = {
                let soi = &game.solar_system.bodies[game.flight.ship.soi_body];
                if let Some(ref atmo) = soi.atmosphere {
                    let dist = (game.flight.ship.rel_position[0].powi(2) + game.flight.ship.rel_position[1].powi(2)).sqrt();
                    let alt = dist - soi.radius;
                    if alt >= 0.0 && alt < atmo.visible_height() {
                        (atmo.pressure_at_altitude(alt) / 101_325.0).clamp(0.0, 1.0) // normalized to 1 atm
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            };

            // Always update engine states and consume fuel (updates active flags even at 0 throttle)
            let effective_dt = dt * time_warp;
            if !render_state.debug_infinite_fuel {
                vessel.consume_fuel(effective_dt, atmo_pressure, &game.part_definitions);
            } else {
                // Still update engine states without draining fuel
                vessel.update_engine_states(&game.part_definitions);
            }

            // Consume RCS fuel when rotating (manual or autopilot), only if RCS enabled
            let rcs_direction = if rcs_enabled {
                if autopilot_active {
                    if let Some(target_angle) = autopilot_target_angle {
                        game.flight.ship.autopilot_desired_direction(target_angle, vessel_physics.as_ref())
                    } else {
                        0.0
                    }
                } else if game.flight.ship_input.rotate_left {
                    1.0
                } else if game.flight.ship_input.rotate_right {
                    -1.0
                } else {
                    0.0
                }
            } else {
                0.0
            };
            vessel.consume_rcs_fuel(effective_dt, rcs_direction, &game.part_definitions);

            // Consume RCS fuel for translation
            vessel.consume_rcs_translation_fuel(effective_dt, game.flight.ship.rcs_translate, &game.part_definitions);

            vessel.recalculate_mass(&game.part_definitions);

            // Power update: compute sun distance and update electricity
            {
                let ship_abs = game.flight.ship.absolute_position(&game.solar_system);
                let sun_pos = game.solar_system.body_position(1); // Sun is index 1
                let dx = ship_abs[0] - sun_pos[0];
                let dy = ship_abs[1] - sun_pos[1];
                let sun_distance_m = (dx * dx + dy * dy).sqrt();
                let (gen, cons) = vessel.update_power(effective_dt, sun_distance_m, &game.part_definitions);
                power_generation = gen;
                power_consumption = cons;
            }

            // Sync vessel state from ship
            vessel.rel_position = game.flight.ship.rel_position;
            vessel.rel_velocity = game.flight.ship.rel_velocity;
            vessel.rotation = game.flight.ship.rotation;

            // Per-part heat update
            if !game.flight.ship.on_rails {
                if let Some((density, airspeed, airspeed_dir_world)) = game.flight.ship.compute_aero_environment(&game.solar_system) {
                    // Convert airspeed direction from world to part-local coordinates.
                    // Ship physics uses X=forward (rotation=0 → nose along +X), but parts
                    // use Y=forward (editor convention: nose at +Y). We apply the inverse
                    // rotation with a -PI/2 offset to account for this.
                    let rot = game.flight.ship.rotation;
                    let cos_r = rot.cos();
                    let sin_r = rot.sin();
                    // First: physics-local (X=forward)
                    let phys_x = airspeed_dir_world[0] * cos_r + airspeed_dir_world[1] * sin_r;
                    let phys_y = -airspeed_dir_world[0] * sin_r + airspeed_dir_world[1] * cos_r;
                    // Rotate +90° to part-local (Y=forward): part_x = -phys_y, part_y = phys_x
                    let airspeed_dir_local = [-phys_y, phys_x];
                    vessel.update_part_temperatures(effective_dt, density, airspeed, airspeed_dir_local, &game.part_definitions);
                } else {
                    // No atmosphere — radiative cooling only
                    vessel.update_part_temperatures(effective_dt, 0.0, 0.0, [1.0, 0.0], &game.part_definitions);
                }
            } else {
                // On-rails: cool all parts toward ambient with exponential decay
                for part in &mut vessel.parts {
                    if part.destroyed || part.decoupled { continue; }
                    if part.temperature > 300.0 {
                        part.temperature += (300.0 - part.temperature) * (1.0 - (-0.01 * effective_dt).exp());
                        part.temperature = part.temperature.max(300.0);
                    }
                }
            }
            // Sync hottest part temperature back to ship for HUD
            let hottest = vessel.parts.iter()
                .filter(|p| !p.destroyed && !p.decoupled)
                .map(|p| p.temperature)
                .fold(300.0_f64, f64::max);
            game.flight.ship.temperature = hottest;

            // Per-part terrain collision check
            if matches!(game.flight.ship.state, ShipState::Flying) {
                let soi_body = &game.solar_system.bodies[game.flight.ship.soi_body];
                if let Some(surface_angle) = vessel.check_terrain_collision(
                    game.flight.ship.rel_position,
                    game.flight.ship.rotation,
                    soi_body.radius,
                    game.flight.ship.soi_body,
                ) {
                    // If landing on launchpad, account for its height
                    let launchpad_offset = if game.flight.ship.soi_body == sunscatter::game::LAUNCHPAD_BODY_INDEX {
                        let angle_diff = surface_angle - sunscatter::game::LAUNCHPAD_SURFACE_ANGLE;
                        let angle_diff = angle_diff - (angle_diff / std::f64::consts::TAU).round() * std::f64::consts::TAU;
                        let half_angle = (sunscatter::game::LAUNCHPAD_BOTTOM_WIDTH * 0.5)
                            / game.solar_system.bodies[game.flight.ship.soi_body].radius;
                        if angle_diff.abs() < half_angle {
                            sunscatter::game::LAUNCHPAD_HEIGHT
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    let surface_distance = soi_body.radius + launchpad_offset + vessel.bottom_extent();
                    game.flight.ship.rel_position = [
                        surface_distance * surface_angle.cos(),
                        surface_distance * surface_angle.sin(),
                    ];
                    game.flight.ship.rel_velocity = [0.0, 0.0];
                    game.flight.ship.throttle = 0.0;
                    game.flight.ship.rotation = surface_angle;
                    game.flight.ship.state = ShipState::Landed {
                        body_index: game.flight.ship.soi_body,
                        surface_angle,
                    };
                    game.flight.ship.on_rails = false;
                }
            }
        }

        // Apply burns to maneuver node delta-v (atmosphere-adjusted thrust)
        let current_accel = vessel_physics.as_ref()
            .map(|v| if v.total_mass > 0.0 {
                let soi = &game.solar_system.bodies[game.flight.ship.soi_body];
                let p = if let Some(ref atmo) = soi.atmosphere {
                    let dist = (game.flight.ship.rel_position[0].powi(2) + game.flight.ship.rel_position[1].powi(2)).sqrt();
                    let alt = dist - soi.radius;
                    if alt >= 0.0 && alt < atmo.visible_height() {
                        (atmo.pressure_at_altitude(alt) / 101_325.0).clamp(0.0, 1.0)
                    } else { 0.0 }
                } else { 0.0 };
                let thrust = v.max_thrust_vac * (1.0 - p) + v.max_thrust_asl * p;
                thrust / v.total_mass
            } else { 0.0 })
            .unwrap_or(MAX_THRUST_ACCELERATION);
        if game.flight.ship.throttle > 0.0 && render_state.get_selected_maneuver_node().is_some() {
            let delta_v_this_frame = game.flight.ship.throttle * current_accel * dt * time_warp;
            let burn_direction = [game.flight.ship.rotation.cos(), game.flight.ship.rotation.sin()];
            render_state.apply_burn_to_maneuver(burn_direction, delta_v_this_frame);
        }

        // Propagate inactive vessels on rails
        let dt_sim = dt * time_warp;
        for vessel in &mut game.flight.inactive_vessels {
            if !vessel.ship.on_rails {
                vessel.ship.enter_rails_mode(&game.solar_system);
            }
            vessel.ship.update_on_rails(dt_sim, &game.solar_system);
        }
        // Delete vessels that entered atmosphere, hit surface, or whose orbit dips into atmosphere
        // Keep vessels within 3km of the active vessel even in atmosphere
        let active_pos = game.flight.ship.rel_position;
        let active_soi = game.flight.ship.soi_body;
        game.flight.inactive_vessels.retain(|v| {
            let in_landing_zone = v.ship.in_atmosphere(&game.solar_system)
                || v.ship.below_landing_altitude(&game.solar_system);
            if v.ship.periapsis_below_surface(&game.solar_system) && in_landing_zone {
                // Check if within 3km of active vessel
                if v.ship.soi_body == active_soi {
                    let dx = v.ship.rel_position[0] - active_pos[0];
                    let dy = v.ship.rel_position[1] - active_pos[1];
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist < 3000.0 {
                        return true; // Keep — close to active vessel
                    }
                }
                false // Delete — in atmosphere, far from active vessel
            } else {
                true // Keep — not in danger zone
            }
        });

        // Collision detection: active vs inactive vessels during physics warp (1x-10x)
        // Uses oriented bounding box (OBB) collision via Separating Axis Theorem
        if time_warp <= RAILS_WARP_THRESHOLD {
            if let Some(ref vessel) = game.flight.vessel {
                let active_hw = vessel.bounding_half_width();
                let active_hh = vessel.bounding_half_height();
                let active_rot = game.flight.ship.rotation;

                for inactive in &mut game.flight.inactive_vessels {
                    if inactive.ship.soi_body != active_soi { continue; }

                    let (inactive_hw, inactive_hh) = inactive.vessel.as_ref()
                        .map(|v| (v.bounding_half_width(), v.bounding_half_height()))
                        .unwrap_or((0.5, 1.0));
                    let inactive_rot = inactive.ship.rotation;

                    // Quick broad-phase: circle check with circumscribed radius
                    let dx = active_pos[0] - inactive.ship.rel_position[0];
                    let dy = active_pos[1] - inactive.ship.rel_position[1];
                    let dist_sq = dx * dx + dy * dy;
                    let max_r = (active_hw * active_hw + active_hh * active_hh).sqrt()
                        + (inactive_hw * inactive_hw + inactive_hh * inactive_hh).sqrt();
                    if dist_sq > max_r * max_r { continue; }

                    // Narrow-phase: SAT test on two oriented bounding boxes
                    if obb_overlap(
                        active_pos, active_rot, active_hw, active_hh,
                        inactive.ship.rel_position, inactive_rot, inactive_hw, inactive_hh,
                    ) {
                        let dist = dist_sq.sqrt().max(0.001);
                        let nx = dx / dist;
                        let ny = dy / dist;

                        // Separate: push active vessel out along collision normal
                        // Use a conservative separation distance
                        let overlap = (active_hw + inactive_hw).min(active_hh + inactive_hh);
                        game.flight.ship.rel_position[0] += nx * overlap * 0.5;
                        game.flight.ship.rel_position[1] += ny * overlap * 0.5;

                        // Bounce: reflect relative velocity along collision normal
                        let rel_vx = game.flight.ship.rel_velocity[0] - inactive.ship.rel_velocity[0];
                        let rel_vy = game.flight.ship.rel_velocity[1] - inactive.ship.rel_velocity[1];
                        let rel_dot_n = rel_vx * nx + rel_vy * ny;

                        // Only bounce if moving toward the inactive vessel
                        if rel_dot_n < 0.0 {
                            let restitution = 0.3;
                            let impulse = -(1.0 + restitution) * rel_dot_n;
                            game.flight.ship.rel_velocity[0] += impulse * nx;
                            game.flight.ship.rel_velocity[1] += impulse * ny;
                        }
                        break;
                    }
                }
            }
        }

        vessel_physics
    } else {
        // Paused: still need vessel_physics for HUD display
        let rcs_enabled = render_state.rcs_enabled;
        game.flight.vessel.as_ref().map(|v| VesselPhysicsData {
            total_mass: v.total_mass,
            max_thrust_vac: v.active_thrust_vac(),
            max_thrust_asl: v.active_thrust_asl(),
            vessel_height: v.bounding_half_height(),
            bottom_extent: v.bottom_extent(),
            moment_of_inertia: v.moment_of_inertia,
            rcs_torque: if rcs_enabled { v.compute_rcs_torque(&game.part_definitions) } else { 0.0 },
            gimbal_torque: v.compute_gimbal_torque(),
            vessel_half_width: v.bounding_half_width(),
            rcs_translation_force: if rcs_enabled { v.compute_rcs_translation_force(&game.part_definitions) } else { 0.0 },
        })
    };

    // Compute RCS rotation direction for plume rendering (0 if RCS disabled)
    let rcs_direction_for_render = if render_state.rcs_enabled {
        let autopilot_target = render_state.get_autopilot_target();
        let autopilot_active = autopilot_target != AutopilotTarget::Off && !game.flight.ship.on_rails;
        if autopilot_active {
            let autopilot_target_angle = if autopilot_target == AutopilotTarget::Target {
                render_state.selected_target_angle
            } else {
                let maneuver_node = render_state.get_selected_maneuver_node();
                game.flight.ship.autopilot_target_angle(autopilot_target, maneuver_node)
            };
            if let Some(target_angle) = autopilot_target_angle {
                let vessel_physics = game.flight.vessel.as_ref().map(|v| VesselPhysicsData {
                    total_mass: v.total_mass,
                    max_thrust_vac: v.active_thrust_vac(),
                    max_thrust_asl: v.active_thrust_asl(),
                    vessel_height: v.bounding_half_height(),
                    bottom_extent: v.bottom_extent(),
                    moment_of_inertia: v.moment_of_inertia,
                    rcs_torque: v.compute_rcs_torque(&game.part_definitions),
                    gimbal_torque: v.compute_gimbal_torque(),
                    vessel_half_width: v.bounding_half_width(),
                    rcs_translation_force: v.compute_rcs_translation_force(&game.part_definitions),
                });
                game.flight.ship.autopilot_desired_direction(target_angle, vessel_physics.as_ref())
            } else {
                0.0
            }
        } else if game.flight.ship_input.rotate_left {
            1.0
        } else if game.flight.ship_input.rotate_right {
            -1.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Compute RCS translation for plume rendering (only when RCS enabled)
    let rcs_translate_for_render = if render_state.rcs_enabled {
        game.flight.ship.rcs_translate
    } else {
        [0.0, 0.0]
    };

    // --- Rendering (always runs) ---

    // Compute atmospheric pressure fraction for HUD thrust display
    let hud_atmo_pressure = {
        let soi = &game.solar_system.bodies[game.flight.ship.soi_body];
        if let Some(ref atmo) = soi.atmosphere {
            let dist = (game.flight.ship.rel_position[0].powi(2) + game.flight.ship.rel_position[1].powi(2)).sqrt();
            let alt = dist - soi.radius;
            if alt >= 0.0 && alt < atmo.visible_height() {
                (atmo.pressure_at_altitude(alt) / 101_325.0).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        }
    };

    // Compute current acceleration for HUD (atmosphere-adjusted)
    let current_accel = vessel_physics.as_ref()
        .map(|v| if v.total_mass > 0.0 {
            let thrust = v.max_thrust_vac * (1.0 - hud_atmo_pressure) + v.max_thrust_asl * hud_atmo_pressure;
            thrust / v.total_mass
        } else { 0.0 })
        .unwrap_or(MAX_THRUST_ACCELERATION);

    // Collect body data for rendering
    let mut scaled_positions: Vec<[f64; 2]> = Vec::with_capacity(game.solar_system.bodies.len());

    for i in 0..game.solar_system.bodies.len() {
        let pos = game.solar_system.body_position(i);
        let body = &game.solar_system.bodies[i];

        let scaled_pos = if let Some(parent_idx) = body.parent {
            let parent_scaled = scaled_positions[parent_idx];
            let parent_unscaled = game.solar_system.body_position(parent_idx);
            let rel_x = pos[0] - parent_unscaled[0];
            let rel_y = pos[1] - parent_unscaled[1];
            [
                parent_scaled[0] + rel_x * BODY_SCALE,
                parent_scaled[1] + rel_y * BODY_SCALE,
            ]
        } else {
            pos
        };
        scaled_positions.push(scaled_pos);
    }

    // Update camera tracking
    if game.flight.tracking_ship {
        // Decompose camera into body_center + ship_offset for precision at galaxy-scale distances.
        // body_center is subtracted from body positions in f64 (precise galaxy-scale subtraction).
        // ship_offset is subtracted in the GPU shader in f32 (precise because it's a small value).
        // camera.position is the full position (quantized) for backward compat with UI/hit-testing.
        let soi_pos = scaled_positions[game.flight.ship.soi_body];
        let rel = game.flight.ship.rel_position;
        let body_center = [soi_pos[0] * SCALE, soi_pos[1] * SCALE];
        let ship_offset = [rel[0] * SCALE * BODY_SCALE, rel[1] * SCALE * BODY_SCALE];
        render_state.camera.body_center = body_center;
        render_state.camera.ship_offset = ship_offset;
        render_state.camera.position[0] = body_center[0] + ship_offset[0];
        render_state.camera.position[1] = body_center[1] + ship_offset[1];
    } else {
        render_state.update_tracking(&scaled_positions, SCALE);
    }

    // Camera rotation: surface-down when below landing altitude and suborbital
    {
        let below_landing = game.flight.ship.below_landing_altitude(&game.solar_system);
        let is_suborbital = game.flight.ship.is_suborbital(&game.solar_system);

        let target_rotation = if below_landing && is_suborbital {
            // Rotate so surface is "down": ship's radial-in direction points screen-down
            let rx = game.flight.ship.rel_position[0];
            let ry = game.flight.ship.rel_position[1];
            // atan2(ry, rx) = angle of ship from body center
            // We want "up" on screen (positive Y) to align with radial-out
            // So rotation = PI/2 - angle_from_body
            (std::f64::consts::FRAC_PI_2 - ry.atan2(rx)) as f32
        } else {
            0.0
        };

        // Smooth interpolation with angle wrapping
        let mut diff = target_rotation - render_state.camera.rotation;
        while diff > std::f32::consts::PI {
            diff -= std::f32::consts::TAU;
        }
        while diff < -std::f32::consts::PI {
            diff += std::f32::consts::TAU;
        }

        let rate = 5.0 * dt as f32; // ~5 rad/s
        if diff.abs() < rate {
            render_state.camera.rotation = target_rotation;
        } else {
            render_state.camera.rotation += diff.signum() * rate;
        }

        // Normalize to [-PI, PI]
        while render_state.camera.rotation > std::f32::consts::PI {
            render_state.camera.rotation -= std::f32::consts::TAU;
        }
        while render_state.camera.rotation < -std::f32::consts::PI {
            render_state.camera.rotation += std::f32::consts::TAU;
        }
    }

    let bodies: Vec<_> = (0..game.solar_system.bodies.len())
        .map(|i| {
            let body = &game.solar_system.bodies[i];
            let pos = scaled_positions[i];
            let atmo_height = body.atmosphere.map(|a| a.visible_height()).unwrap_or(0.0);
            let atmo_color = body.atmosphere.map(|a| a.color).unwrap_or([0.0; 3]);
            (pos[0], pos[1], body.radius * BODY_SCALE, body.color, atmo_height, atmo_color, i)
        })
        .collect();

    let pixels_per_world_unit = render_state.camera.zoom * render_state.size.height as f32 / 2.0;

    let orbits: Vec<Option<OrbitRenderData>> = (0..game.solar_system.bodies.len())
        .map(|i| {
            let body = &game.solar_system.bodies[i];
            match (body.parent, &body.orbit) {
                (Some(parent_idx), Some(orbit)) => {
                    // Skip star orbits around galactic center (shown only in galaxy view)
                    let parent_body = &game.solar_system.bodies[parent_idx];
                    if parent_body.parent.is_none() {
                        return None;
                    }

                    let body_world_radius = (body.radius * BODY_SCALE * SCALE) as f32;
                    let body_pixels = body_world_radius * pixels_per_world_unit * 2.0;
                    let is_moon = parent_body.parent
                        .map_or(false, |gp| game.solar_system.bodies[gp].parent.is_some());
                    let pixel_threshold = if is_moon { 100.0 } else { 5.0 };

                    if body_pixels >= pixel_threshold {
                        return None;
                    }

                    let parent_pos = scaled_positions[parent_idx];
                    let orbit_color = [
                        body.color[0] * 0.4,
                        body.color[1] * 0.4,
                        body.color[2] * 0.4,
                        0.5,
                    ];
                    Some(OrbitRenderData {
                        parent_x: parent_pos[0] * SCALE,
                        parent_y: parent_pos[1] * SCALE,
                        semi_major_axis: orbit.semi_major_axis * SCALE * BODY_SCALE,
                        eccentricity: orbit.eccentricity,
                        argument_of_periapsis: orbit.argument_of_periapsis,
                        color: orbit_color,
                    })
                }
                _ => None,
            }
        })
        .collect();

    // Prepare ship render data
    let _ = game.flight.ship.calculate_orbit(&game.solar_system);
    let ship_orbit = game.flight.ship.get_orbital_info(&game.solar_system).map(|info| {
        let parent_pos = scaled_positions[info.parent_idx];
        let parent_body = &game.solar_system.bodies[info.parent_idx];
        ShipOrbitData {
            parent_x: parent_pos[0] * SCALE,
            parent_y: parent_pos[1] * SCALE,
            semi_major_axis: info.orbit.semi_major_axis * SCALE * BODY_SCALE,
            eccentricity: info.orbit.eccentricity,
            argument_of_periapsis: info.orbit.argument_of_periapsis,
            apoapsis: info.apoapsis,
            periapsis: info.periapsis,
            orbital_period: info.orbital_period,
            time_to_apoapsis: info.time_to_apoapsis,
            time_to_periapsis: info.time_to_periapsis,
            parent_body_radius: parent_body.radius,
            parent_name: parent_body.name.clone(),
            retrograde: info.retrograde,
        }
    });

    let velocity = (game.flight.ship.rel_velocity[0].powi(2) + game.flight.ship.rel_velocity[1].powi(2)).sqrt();
    let distance_from_soi = (game.flight.ship.rel_position[0].powi(2) + game.flight.ship.rel_position[1].powi(2)).sqrt();
    let soi_body = &game.solar_system.bodies[game.flight.ship.soi_body];
    let altitude = distance_from_soi - soi_body.radius;

    let patched_traj_raw = game.flight.ship.get_patched_trajectory(&game.solar_system);
    let time_to_intercept = patched_traj_raw.as_ref()
        .and_then(|traj| traj.segments.first())
        .and_then(|seg| seg.end_time);

    let patched_trajectory = patched_traj_raw.as_ref()
        .map(|traj| {
            traj.segments.iter().enumerate().map(|(i, seg)| {
                let parent_pos = scaled_positions[seg.parent_idx];
                let parent_soi = game.solar_system.bodies[seg.parent_idx].soi_radius;
                let parent_mass = game.solar_system.bodies[seg.parent_idx].effective_mass_at(seg.orbit.semi_major_axis);
                let alpha = if i == 0 { 0.7 } else { 0.4 };
                OrbitSegmentData {
                    parent_x: parent_pos[0] * SCALE,
                    parent_y: parent_pos[1] * SCALE,
                    semi_major_axis: seg.orbit.semi_major_axis * SCALE * BODY_SCALE,
                    eccentricity: seg.orbit.eccentricity,
                    argument_of_periapsis: seg.orbit.argument_of_periapsis,
                    start_true_anomaly: seg.start_true_anomaly,
                    end_true_anomaly: seg.end_true_anomaly,
                    color: [game.flight.ship.color[0] * 0.6, game.flight.ship.color[1] * 0.6, game.flight.ship.color[2] * 0.6, alpha],
                    is_first_segment: i == 0,
                    retrograde: seg.retrograde,
                    soi_radius: parent_soi * SCALE * BODY_SCALE,
                    parent_body_radius: game.solar_system.bodies[seg.parent_idx].radius,
                    parent_mass,
                    parent_idx: seg.parent_idx,
                    render_scale: SCALE * BODY_SCALE,
                    start_time: seg.start_time,
                    base_epoch: game.simulation_time,
                }
            }).collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let current_true_anomaly = patched_trajectory.first()
        .map(|seg| seg.start_true_anomaly)
        .unwrap_or(0.0);

    // Build part render data and vessel stats from flight vessel

    // Sun distance for solar panel output display
    let sun_distance_m = {
        let ship_abs = game.flight.ship.absolute_position(&game.solar_system);
        let sun_pos = game.solar_system.body_position(1);
        let dx = ship_abs[0] - sun_pos[0];
        let dy = ship_abs[1] - sun_pos[1];
        (dx * dx + dy * dy).sqrt()
    };

    let (part_render_data, vessel_mass, vessel_fuel_frac, vessel_thrust, vessel_delta_v, vessel_stage_delta_vs, vessel_size) =
        if let Some(ref vessel) = game.flight.vessel {
            let parts: Vec<ShipPartRenderData> = vessel.parts.iter()
                .enumerate()
                .filter(|(_, p)| !p.destroyed && !p.decoupled)
                .map(|(i, p)| {
                    let def = game.part_definitions.get(&p.definition_id);
                    let name = def.map(|d| d.name.clone()).unwrap_or_else(|| p.definition_id.clone());
                    let dry_mass = def.map(|d| d.mass).unwrap_or(0.0);

                    // Engine info
                    let is_engine = p.propellant_type.is_some();
                    let engine_thrust_vac = if is_engine { Some(p.engine_thrust_vac) } else { None };
                    let engine_thrust_asl = if is_engine { Some(p.engine_thrust_asl) } else { None };
                    let engine_isp_vac = if is_engine { Some(p.engine_isp_vac) } else { None };
                    let engine_isp_asl = if is_engine { Some(p.engine_isp_asl) } else { None };
                    let propellant_name = p.propellant_type.map(|pt| pt.display_name().to_string());

                    // Tank info: separate oxidizer and fuel
                    let has_tank = def.map(|d| d.tank.is_some()).unwrap_or(false);
                    let (fuel_type_name, fuel_current, fuel_max, ox_current, ox_max) = if has_tank {
                        let fuel_names = ["rp1", "methane", "hydrogen", "monopropellant", "xenon"];
                        let f_current: f64 = fuel_names.iter()
                            .filter_map(|n| p.resources.get(*n))
                            .sum();
                        let f_max: f64 = fuel_names.iter()
                            .filter_map(|n| p.max_resources.get(*n))
                            .sum();
                        let o_current = p.resources.get("oxygen").copied().unwrap_or(0.0);
                        let o_max = p.max_resources.get("oxygen").copied().unwrap_or(0.0);
                        // Determine fuel type name from what's loaded
                        let ft_name = if p.max_resources.contains_key("rp1") {
                            Some("LOX/RP-1".to_string())
                        } else if p.max_resources.contains_key("methane") {
                            Some("LOX/CH4".to_string())
                        } else if p.max_resources.contains_key("hydrogen") {
                            if o_max > 0.0 { Some("LOX/LH2".to_string()) } else { Some("LH2".to_string()) }
                        } else if p.max_resources.contains_key("monopropellant") {
                            Some("Monopropellant".to_string())
                        } else if p.max_resources.contains_key("xenon") {
                            Some("Xenon".to_string())
                        } else if o_max > 0.0 {
                            Some("LOX".to_string())
                        } else {
                            Some("Empty".to_string())
                        };
                        (ft_name, Some(f_current), Some(f_max), Some(o_current), Some(o_max))
                    } else {
                        (None, None, None, None, None)
                    };

                    // Pod info
                    let crew_capacity = def.and_then(|d| d.pod.as_ref().map(|pod| pod.crew_capacity));
                    let (monoprop_current, monoprop_max) = if crew_capacity.is_some() {
                        let cur = p.resources.get("monopropellant").copied();
                        let max = p.max_resources.get("monopropellant").copied();
                        (cur, max)
                    } else {
                        (None, None)
                    };

                    // Compute RCS nozzle activation state (combined rotation + translation)
                    let has_rcs_input = rcs_direction_for_render.abs() > 0.001
                        || rcs_translate_for_render[0].abs() > 0.001
                        || rcs_translate_for_render[1].abs() > 0.001;
                    let rcs_nozzle_state = if has_rcs_input && p.rcs_thrust > 0.0 {
                        if let Some(rcs_def) = def.and_then(|d| d.rcs.as_ref()) {
                            let dir = rcs_direction_for_render;
                            let rx = p.local_position[0]; // part x relative to COM
                            let ry = p.local_position[1]; // part y relative to COM
                            let is_mirrored = rcs_def.is_mirrored;
                            let trans_fwd = rcs_translate_for_render[0];   // +forward (vessel +Y)
                            let trans_right = rcs_translate_for_render[1]; // +right (vessel +X for non-mirrored)

                            // --- Rotation-driven nozzle activation ---
                            // Lateral nozzle exhausts away from mount side: for non-mirrored,
                            // it exhausts left (-X), producing force to the right at this part.
                            // Torque = r × F: with part above COM (ry > 0), rightward force
                            // gives CW (negative) torque, so torque_sign = -ry.
                            let lateral_torque_sign = if is_mirrored { ry } else { -ry };
                            let rot_lateral = dir.abs() > 0.001 && ((dir > 0.0 && lateral_torque_sign > 0.0) || (dir < 0.0 && lateral_torque_sign < 0.0));
                            // Mirrored lateral: opposite side nozzle (for bilateral pod RCS)
                            let lateral_torque_sign_m = if is_mirrored { -ry } else { ry };
                            let rot_lateral_m = dir.abs() > 0.001 && ((dir > 0.0 && lateral_torque_sign_m > 0.0) || (dir < 0.0 && lateral_torque_sign_m < 0.0));
                            let up_torque_sign = -rx;
                            let rot_up = dir.abs() > 0.001 && ((dir > 0.0 && up_torque_sign > 0.0) || (dir < 0.0 && up_torque_sign < 0.0));
                            let down_torque_sign = rx;
                            let rot_down = dir.abs() > 0.001 && ((dir > 0.0 && down_torque_sign > 0.0) || (dir < 0.0 && down_torque_sign < 0.0));

                            // --- Translation-driven nozzle activation ---
                            // Forward (vessel +Y): down nozzles fire (push ship forward=up)
                            let trans_down = trans_fwd > 0.001;
                            // Backward (vessel -Y): up nozzles fire (push ship backward=down)
                            let trans_up = trans_fwd < -0.001;
                            // Going right (+X): fire right-mount laterals (exhaust left, push right)
                            //   Non-mirrored = right-mount, mirrored = left-mount
                            // Going left (-X): fire left-mount laterals (exhaust right, push left)
                            //   Mirrored = left-mount (lateral nozzle exhausts right)
                            let trans_lateral = (trans_right > 0.001 && !is_mirrored) || (trans_right < -0.001 && is_mirrored);
                            let lateral = rot_lateral || trans_lateral;

                            // Pod bilateral nozzles (opposite side):
                            // Going left: fire right nozzle on non-mirrored pod (exhaust right, push left)
                            // Going right: fire left nozzle on mirrored pod (exhaust left, push right)
                            let trans_lateral_m = (trans_right < -0.001 && !is_mirrored) || (trans_right > 0.001 && is_mirrored);
                            let lateral_mirrored = rot_lateral_m || trans_lateral_m;
                            let up = rot_up || trans_up;
                            let down = rot_down || trans_down;
                            if lateral || lateral_mirrored || up || down {
                                Some(sunscatter::render::RcsNozzleState { lateral, lateral_mirrored, up, down })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    ShipPartRenderData {
                        definition_id: p.definition_id.clone(),
                        local_x: p.local_position[0],
                        local_y: p.local_position[1],
                        rotation: p.rotation,
                        engine_active: p.engine_active,
                        part_index: i,
                        name,
                        dry_mass,
                        hitbox_half_w: if is_part_rotation_swapped(p.rotation) { p.hitbox_half_extents[1] } else { p.hitbox_half_extents[0] },
                        hitbox_half_h: if is_part_rotation_swapped(p.rotation) { p.hitbox_half_extents[0] } else { p.hitbox_half_extents[1] },
                        click_local_y: p.local_position[1],
                        click_hitbox_half_h: if is_part_rotation_swapped(p.rotation) { p.hitbox_half_extents[0] } else { p.hitbox_half_extents[1] },
                        engine_thrust_vac,
                        engine_thrust_asl,
                        engine_isp_vac,
                        engine_isp_asl,
                        engine_enabled: p.engine_enabled,
                        propellant_name,
                        fuel_type_name,
                        fuel_current,
                        fuel_max,
                        ox_current,
                        ox_max,
                        crew_capacity,
                        monoprop_current,
                        monoprop_max,
                        battery_current: if p.max_electricity > 0.0 { Some(p.electricity) } else { None },
                        battery_max: if p.max_electricity > 0.0 { Some(p.max_electricity) } else { None },
                        solar_output: def.and_then(|d| d.solar_panel.as_ref().map(|sp| {
                            let au_m = 1.496e11_f64;
                            let ratio = au_m / sun_distance_m.max(1.0);
                            sp.output_1au * ratio * ratio
                        })),
                        rtg_output: def.and_then(|d| d.rtg.as_ref().map(|r| r.output_watts)),
                        is_decoupler: def.map(|d| d.decoupler.is_some()).unwrap_or(false),
                        crossfeed_enabled: p.crossfeed_enabled,
                        gimbal_angle: p.gimbal_angle,
                        rcs_thrust: if p.rcs_thrust > 0.0 { Some(p.rcs_thrust) } else { None },
                        rcs_nozzle_state,
                        heat_fraction: ((p.temperature - 300.0) / (p.max_heat_tolerance - 300.0)).clamp(0.0, 1.0) as f32,
                        temperature: p.temperature,
                        is_fairing: def.map(|d| d.fairing.is_some()).unwrap_or(false),
                        fairing_shape: p.fairing_shape.clone(),
                        fairing_half: p.fairing_half,
                    }
                })
                .collect();

            let (stage_fuel, stage_fuel_max) = vessel.get_stage_fuel(&game.part_definitions);
            let fuel_frac = if stage_fuel_max > 0.0 {
                stage_fuel / stage_fuel_max
            } else {
                0.0
            };

            let size = vessel.bounding_half_height() * 2.0;
            let stage_dvs = vessel.calculate_stage_delta_v(&game.part_definitions);
            let dv: f64 = stage_dvs.iter().sum();

            (Some(parts), Some(vessel.total_mass), Some(fuel_frac), Some(vessel.active_thrust_at_pressure(hud_atmo_pressure)), Some(dv), stage_dvs, size)
        } else {
            (None, None, None, None, None, Vec::new(), SHIP_SIZE)
        };

    // Use hottest part temperature when vessel exists, otherwise ship temperature
    let (effective_temp, effective_heat_fraction) = if let Some(ref vessel) = game.flight.vessel {
        let hottest = vessel.parts.iter()
            .filter(|p| !p.destroyed && !p.decoupled)
            .map(|p| p.temperature)
            .fold(300.0_f64, f64::max);
        let max_tol = vessel.parts.iter()
            .filter(|p| !p.destroyed && !p.decoupled)
            .filter(|p| p.temperature == hottest)
            .map(|p| p.max_heat_tolerance)
            .next()
            .unwrap_or(default_heat_tolerance());
        let frac = ((hottest - 300.0) / (max_tol - 300.0)).clamp(0.0, 1.0) as f32;
        (hottest, frac)
    } else {
        let frac = ((game.flight.ship.temperature - AMBIENT_TEMPERATURE)
            / (default_heat_tolerance() - AMBIENT_TEMPERATURE))
            .clamp(0.0, 1.0) as f32;
        (game.flight.ship.temperature, frac)
    };
    let heat_fraction = effective_heat_fraction;

    // Compute felt g-force (what crew experiences)
    const G0: f64 = 9.80665;
    let g_force = match &game.flight.ship.state {
        ShipState::Landed { .. } => {
            // On the ground, crew feels the normal force = surface gravity
            soi_body.surface_gravity() / G0
        }
        ShipState::Flying => {
            // In flight, crew feels thrust + drag as a vector sum (gravity is freefall, not felt)
            let rot = game.flight.ship.rotation;
            let thrust_mag = vessel_physics.as_ref()
                .map(|v| if v.total_mass > 0.0 {
                    let thrust = v.max_thrust_vac * (1.0 - hud_atmo_pressure) + v.max_thrust_asl * hud_atmo_pressure;
                    game.flight.ship.throttle * thrust / v.total_mass
                } else { 0.0 })
                .unwrap_or(0.0);
            let thrust = [rot.cos() * thrust_mag, rot.sin() * thrust_mag];

            let drag = {
                let soi = &game.solar_system.bodies[game.flight.ship.soi_body];
                game.flight.ship.compute_drag_accel(soi, vessel_physics.as_ref())
            };

            let net_x = thrust[0] + drag[0];
            let net_y = thrust[1] + drag[1];
            (net_x.powi(2) + net_y.powi(2)).sqrt() / G0
        }
    };

    // Compute drag force in kN for HUD display
    let drag_kn = {
        let soi = &game.solar_system.bodies[game.flight.ship.soi_body];
        let drag_accel = game.flight.ship.compute_drag_accel(soi, vessel_physics.as_ref());
        let drag_mag = (drag_accel[0].powi(2) + drag_accel[1].powi(2)).sqrt();
        let mass_kg = vessel_physics.as_ref().map(|v| v.total_mass * 1000.0).unwrap_or(1000.0);
        drag_mag * mass_kg / 1000.0 // N -> kN
    };

    // Use scaled_positions + rel_position to match body rendering precision
    let soi_pos_render = scaled_positions[game.flight.ship.soi_body];
    let rel_render = game.flight.ship.rel_position;
    let ship_render = ShipRenderData {
        x: soi_pos_render[0] * SCALE + rel_render[0] * SCALE * BODY_SCALE,
        y: soi_pos_render[1] * SCALE + rel_render[1] * SCALE * BODY_SCALE,
        rotation: game.flight.ship.rotation,
        size: vessel_size * SCALE * BODY_SCALE,
        color: game.flight.ship.color,
        orbit: ship_orbit,
        patched_trajectory,
        velocity,
        altitude,
        soi_body_name: soi_body.name.clone(),
        throttle: game.flight.ship.throttle,
        time_to_intercept,
        acceleration: current_accel,
        current_true_anomaly,
        parts: part_render_data,
        total_mass: vessel_mass,
        fuel_fraction: vessel_fuel_frac,
        power_generation: if game.flight.vessel.is_some() { Some(power_generation) } else { None },
        power_consumption: if game.flight.vessel.is_some() { Some(power_consumption) } else { None },
        electricity_fraction: game.flight.vessel.as_ref().and_then(|v| v.electricity_fraction()),
        electricity_stored: game.flight.vessel.as_ref().map(|v| v.total_electricity()),
        thrust_kn: vessel_thrust,
        drag_kn,
        delta_v: vessel_delta_v,
        soi_surface_gravity: game.solar_system.bodies[game.flight.ship.soi_body].surface_gravity(),
        g_force,
        current_stage: game.flight.vessel.as_ref().map(|v| v.current_stage),
        total_stages: game.flight.vessel.as_ref().map(|v| v.stages.len()),
        temperature: effective_temp,
        heat_fraction,
        heat_flux: game.flight.ship.heat_flux,
        rcs_direction: rcs_direction_for_render,
        rcs_translate: rcs_translate_for_render,
        below_landing_altitude: game.flight.ship.below_landing_altitude(&game.solar_system)
            && matches!(game.flight.ship.state, ShipState::Flying),
        velocity_direction: {
            let vx = game.flight.ship.rel_velocity[0];
            let vy = game.flight.ship.rel_velocity[1];
            let speed = (vx * vx + vy * vy).sqrt();
            if speed > 0.1 { [vx / speed, vy / speed] } else { [0.0, 0.0] }
        },
        stage_delta_vs: if vessel_stage_delta_vs.is_empty() { None } else { Some(vessel_stage_delta_vs) },
        stages: game.flight.vessel.as_ref().map(|v| {
            v.stages.iter().map(|stage| {
                stage.iter().map(|&part_idx| {
                    let name = if part_idx < v.parts.len() {
                        game.part_definitions.get(&v.parts[part_idx].definition_id)
                            .map(|d| d.name.clone())
                            .unwrap_or_else(|| format!("Part {}", part_idx))
                    } else {
                        format!("Part {}", part_idx)
                    };
                    StagedPartInfo { part_index: part_idx, name }
                }).collect()
            }).collect()
        }),
    };

    // Build background vessel data for inactive vessels
    let background_vessels: Vec<sunscatter::render::TrackingVesselData> = game.flight.inactive_vessels.iter()
        .map(|v| {
            // Use scaled_positions + rel_position for precision (same as active ship)
            let soi_pos = scaled_positions[v.ship.soi_body];
            let rel = v.ship.rel_position;
            let orbit_data = v.ship.get_render_orbit().map(|(orbit, parent_idx)| {
                let parent_pos = scaled_positions[parent_idx];
                OrbitRenderData {
                    parent_x: parent_pos[0] * SCALE,
                    parent_y: parent_pos[1] * SCALE,
                    semi_major_axis: orbit.semi_major_axis * SCALE * BODY_SCALE,
                    eccentricity: orbit.eccentricity,
                    argument_of_periapsis: orbit.argument_of_periapsis,
                    color: [0.5, 0.5, 0.5, 0.3], // Dimmed grey
                }
            });
            let parts = v.vessel.as_ref().map(|fv| {
                build_vessel_part_render_data(fv, &game.part_definitions)
            });
            sunscatter::render::TrackingVesselData {
                id: v.id,
                name: v.name.clone(),
                color: v.ship.color,
                x: soi_pos[0] * SCALE + rel[0] * SCALE * BODY_SCALE,
                y: soi_pos[1] * SCALE + rel[1] * SCALE * BODY_SCALE,
                body_center: [soi_pos[0] * SCALE, soi_pos[1] * SCALE],
                rel_offset: [rel[0] * SCALE * BODY_SCALE, rel[1] * SCALE * BODY_SCALE],
                soi_body: v.ship.soi_body,
                orbit: orbit_data,
                parts,
                rotation: v.ship.rotation,
            }
        })
        .collect();

    // Store decomposed ship position for precision-safe camera-relative rendering
    render_state.ship_body_center = [soi_pos_render[0] * SCALE, soi_pos_render[1] * SCALE];
    render_state.ship_rel_offset = [rel_render[0] * SCALE * BODY_SCALE, rel_render[1] * SCALE * BODY_SCALE];

    let accretion_discs = build_accretion_disc_data(game);
    let in_galaxy_view = is_galaxy_view(render_state.camera.zoom, render_state.size.height);
    render_state.update_bodies_orbits_ship_and_vessels(&bodies, &orbits, Some(&ship_render), SCALE, Some(&game.part_definitions), &background_vessels, &accretion_discs, in_galaxy_view);

    // Update simulation time for node epoch computation
    render_state.simulation_time = game.simulation_time;

    // Compute target angle for navigation target
    if let Some(target) = render_state.selected_target {
        let ship_abs = game.flight.ship.absolute_position(&game.solar_system);
        let target_abs = match target {
            SelectedTarget::Body(idx) => {
                game.solar_system.body_position(idx)
            }
            SelectedTarget::Vessel(id) => {
                if let Some(v) = game.flight.inactive_vessels.iter().find(|v| v.id == id) {
                    v.ship.absolute_position(&game.solar_system)
                } else {
                    // Target vessel no longer exists; clear target
                    render_state.selected_target = None;
                    render_state.selected_target_name.clear();
                    render_state.selected_target_angle = None;
                    [0.0, 0.0]
                }
            }
        };
        if render_state.selected_target.is_some() {
            let dx = target_abs[0] - ship_abs[0];
            let dy = target_abs[1] - ship_abs[1];
            render_state.selected_target_angle = Some(dy.atan2(dx));
        }
    } else {
        render_state.selected_target_angle = None;
    }

    // Compute closest approach marker to navigation target
    render_state.closest_approach_world_pos = None;
    render_state.closest_approach_marker = None;
    if let (Some(target), Some(ref traj)) = (render_state.selected_target, &patched_traj_raw) {
        // Determine which SOI the target is in
        let target_soi_parent = match target {
            SelectedTarget::Body(idx) => game.solar_system.bodies[idx].parent,
            SelectedTarget::Vessel(id) => {
                game.flight.inactive_vessels.iter()
                    .find(|v| v.id == id)
                    .map(|v| v.ship.soi_body)
            }
        };

        if let Some(target_parent) = target_soi_parent {
            // Find first trajectory segment in the same SOI as the target
            if let Some(seg) = traj.segments.iter().find(|s| s.parent_idx == target_parent) {
                let e = seg.orbit.eccentricity;
                let a = seg.orbit.semi_major_axis;
                let arg_peri = seg.orbit.argument_of_periapsis;
                let parent_mass = game.solar_system.bodies[seg.parent_idx].effective_mass_at(seg.orbit.semi_major_axis);
                let mu = G * parent_mass;

                // Semi-latus rectum
                let p = if e < 1.0 { a * (1.0 - e * e) } else { a.abs() * (e * e - 1.0) };

                // End true anomaly: use segment end or full orbit
                let end_ta = seg.end_true_anomaly.unwrap_or_else(|| {
                    if seg.retrograde {
                        seg.start_true_anomaly - std::f64::consts::TAU
                    } else {
                        seg.start_true_anomaly + std::f64::consts::TAU
                    }
                });

                let start_ma = game.flight.ship.true_to_mean_anomaly(&seg.orbit, seg.start_true_anomaly);
                let n = if e < 1.0 {
                    (mu / a.powi(3)).sqrt()
                } else {
                    (mu / a.abs().powi(3)).sqrt()
                };

                // Closure to compute ship pos and target pos at parameter t in [0,1]
                let compute_positions = |t: f64| -> Option<([f64; 2], [f64; 2])> {
                    let sample_ta = seg.start_true_anomaly + t * (end_ta - seg.start_true_anomaly);
                    let denom = 1.0 + e * sample_ta.cos();
                    if denom <= 0.001 { return None; }
                    let r = p / denom;
                    if r <= 0.0 || !r.is_finite() { return None; }

                    let angle = sample_ta + arg_peri;
                    let ship_pos = [r * angle.cos(), r * angle.sin()];

                    // Travel time from segment start
                    let sample_ma = game.flight.ship.true_to_mean_anomaly(&seg.orbit, sample_ta);
                    let delta_ma = if e < 1.0 {
                        if seg.retrograde {
                            let mut d = start_ma - sample_ma;
                            if d < 0.0 { d += std::f64::consts::TAU; }
                            d
                        } else {
                            let mut d = sample_ma - start_ma;
                            if d < 0.0 { d += std::f64::consts::TAU; }
                            d
                        }
                    } else {
                        (sample_ma - start_ma).abs()
                    };
                    let travel_time = if n > 0.0 { delta_ma / n } else { 0.0 };
                    let abs_time = game.simulation_time + seg.start_time + travel_time;

                    // Target position relative to shared parent at abs_time
                    let target_pos = match target {
                        SelectedTarget::Body(idx) => {
                            game.solar_system.bodies[idx].orbit.as_ref()
                                .map(|orb| orb.position_at(abs_time, parent_mass))
                                .unwrap_or([0.0, 0.0])
                        }
                        SelectedTarget::Vessel(id) => {
                            // Use vessel's current position (no orbit propagation)
                            game.flight.inactive_vessels.iter()
                                .find(|v| v.id == id)
                                .map(|v| v.ship.rel_position)
                                .unwrap_or([0.0, 0.0])
                        }
                    };

                    Some((ship_pos, target_pos))
                };

                let compute_dist = |t: f64| -> f64 {
                    compute_positions(t).map_or(f64::MAX, |(sp, tp)| {
                        let dx = sp[0] - tp[0];
                        let dy = sp[1] - tp[1];
                        (dx * dx + dy * dy).sqrt()
                    })
                };

                // Coarse sampling (64 points)
                let num_samples = 64usize;
                let mut best_t = 0.0f64;
                let mut best_dist = f64::MAX;
                for i in 0..num_samples {
                    let t = i as f64 / (num_samples - 1) as f64;
                    let dist = compute_dist(t);
                    if dist < best_dist {
                        best_dist = dist;
                        best_t = t;
                    }
                }

                // Golden-section refinement (12 iterations)
                if best_dist < f64::MAX {
                    let phi = (5.0_f64.sqrt() + 1.0) / 2.0;
                    let step = 1.0 / (num_samples - 1) as f64;
                    let mut lo = (best_t - step).max(0.0);
                    let mut hi = (best_t + step).min(1.0);
                    for _ in 0..12 {
                        let c = hi - (hi - lo) / phi;
                        let d = lo + (hi - lo) / phi;
                        if compute_dist(c) < compute_dist(d) {
                            hi = d;
                        } else {
                            lo = c;
                        }
                    }
                    best_t = (lo + hi) / 2.0;
                    best_dist = compute_dist(best_t);
                }

                // Convert best point to render coordinates
                if best_dist < f64::MAX {
                    let best_ta = seg.start_true_anomaly + best_t * (end_ta - seg.start_true_anomaly);
                    let denom = 1.0 + e * best_ta.cos();
                    if denom > 0.001 {
                        let r = p / denom;
                        if r > 0.0 && r.is_finite() {
                            let angle = best_ta + arg_peri;
                            let rel_x = r * angle.cos();
                            let rel_y = r * angle.sin();
                            let parent_scaled = scaled_positions[seg.parent_idx];
                            let world_x = parent_scaled[0] * SCALE + rel_x * SCALE * BODY_SCALE;
                            let world_y = parent_scaled[1] * SCALE + rel_y * SCALE * BODY_SCALE;
                            render_state.closest_approach_world_pos = Some(([world_x, world_y], best_dist));
                        }
                    }
                }
            }
        }
    }

    // Calculate predicted trajectories for maneuver nodes
    let mut predicted_trajectories: Vec<Vec<OrbitSegmentData>> = Vec::new();
    for node in render_state.get_maneuver_nodes() {
        if node.total_delta_v() < 0.001 {
            continue;
        }

        let scale = node.render_scale;
        let parent_idx = node.parent_idx;
        let current_parent_pos = scaled_positions[parent_idx];
        let current_parent_x = current_parent_pos[0] * SCALE;
        let current_parent_y = current_parent_pos[1] * SCALE;

        let world_pos = node.world_pos(current_parent_x, current_parent_y);
        let velocity = node.velocity();

        let rel_x = (world_pos[0] - current_parent_x) / scale;
        let rel_y = (world_pos[1] - current_parent_y) / scale;
        let pos = [rel_x, rel_y];

        let prograde = node.prograde_unit();
        let radial = node.radial_unit();

        let new_vel = [
            velocity[0] + node.delta_v.prograde * prograde[0] + node.delta_v.radial_out * radial[0],
            velocity[1] + node.delta_v.prograde * prograde[1] + node.delta_v.radial_out * radial[1],
        ];

        if let Some(pred_traj) = game.flight.ship.calculate_predicted_trajectory(
            pos, new_vel, parent_idx, &game.solar_system, node.epoch
        ) {
            let segments: Vec<OrbitSegmentData> = pred_traj.segments.iter().enumerate().map(|(i, seg)| {
                let parent_pos = scaled_positions[seg.parent_idx];
                let parent_soi = game.solar_system.bodies[seg.parent_idx].soi_radius;
                let parent_mass = game.solar_system.bodies[seg.parent_idx].effective_mass_at(seg.orbit.semi_major_axis);
                let alpha = if i == 0 { 0.7 } else { 0.5 };
                OrbitSegmentData {
                    parent_x: parent_pos[0] * SCALE,
                    parent_y: parent_pos[1] * SCALE,
                    semi_major_axis: seg.orbit.semi_major_axis * SCALE * BODY_SCALE,
                    eccentricity: seg.orbit.eccentricity,
                    argument_of_periapsis: seg.orbit.argument_of_periapsis,
                    start_true_anomaly: seg.start_true_anomaly,
                    end_true_anomaly: seg.end_true_anomaly,
                    color: [0.2, 0.8, 0.2, alpha],
                    is_first_segment: i == 0,
                    retrograde: seg.retrograde,
                    soi_radius: parent_soi * SCALE * BODY_SCALE,
                    parent_body_radius: game.solar_system.bodies[seg.parent_idx].radius,
                    parent_mass,
                    parent_idx: seg.parent_idx,
                    render_scale: SCALE * BODY_SCALE,
                    start_time: seg.start_time,
                    base_epoch: node.epoch,
                }
            }).collect();
            predicted_trajectories.push(segments);
        }
    }
    render_state.set_predicted_trajectories(predicted_trajectories);

    // --- Transfer planner computation ---
    if render_state.transfer_planner_open {
        use sunscatter::ship::transfer;

        // Update target lists
        let soi = game.flight.ship.soi_body;
        render_state.transfer_hohmann_targets = transfer::hohmann_targets(soi, &game.solar_system.bodies);
        render_state.transfer_interplanetary_targets = transfer::lambert_targets(soi, &game.solar_system.bodies);

        // Auto-select navigation target in planner if no target chosen yet
        if render_state.transfer_selected_target.is_none() {
            if let Some(SelectedTarget::Body(idx)) = render_state.selected_target {
                if render_state.transfer_hohmann_targets.iter().any(|(i, _)| *i == idx) {
                    render_state.transfer_selected_target = Some(idx);
                    render_state.transfer_planner_mode = 0;
                } else if render_state.transfer_interplanetary_targets.iter().any(|(i, _)| *i == idx) {
                    render_state.transfer_selected_target = Some(idx);
                    render_state.transfer_planner_mode = 1;
                }
            }
        }

        // Validate selected target
        let targets = if render_state.transfer_planner_mode == 0 {
            &render_state.transfer_hohmann_targets
        } else {
            &render_state.transfer_interplanetary_targets
        };
        if let Some(sel) = render_state.transfer_selected_target {
            if !targets.iter().any(|(i, _)| *i == sel) {
                render_state.transfer_selected_target = None;
            }
        }

        // Compute transfer if target selected and ship has an orbit
        render_state.transfer_display = if let (Some(target_idx), Some(ship_orbit)) =
            (render_state.transfer_selected_target, game.flight.ship.get_cached_orbit())
        {
            let parent_mass = game.solar_system.bodies[ship_orbit.parent_idx].effective_mass_at(ship_orbit.orbit.semi_major_axis);
            if render_state.transfer_planner_mode == 0 {
                // Hohmann mode
                if let Some(target_orbit) = game.solar_system.bodies[target_idx].orbit.as_ref() {
                    transfer::compute_hohmann(
                        &ship_orbit.orbit,
                        ship_orbit.retrograde,
                        ship_orbit.mean_anomaly,
                        target_orbit,
                        parent_mass,
                        game.solar_system.time,
                    ).map(|h| {
                        let target_name = game.solar_system.bodies[target_idx].name.clone();
                        transfer::TransferDisplay {
                            mode: 0,
                            target_name,
                            departure_dv: h.departure_delta_v.abs(),
                            arrival_dv: h.arrival_delta_v,
                            transfer_time: h.transfer_time,
                            current_phase_angle: h.current_phase_angle.to_degrees(),
                            required_phase_angle: h.required_phase_angle.to_degrees(),
                            time_to_window: h.time_to_window,
                            departure_position_angle: h.departure_position_angle,
                            prograde_dv: h.departure_delta_v,
                            radial_dv: 0.0,
                            valid: true,
                        }
                    })
                } else {
                    None
                }
            } else {
                // Lambert mode
                let defaults = transfer::hohmann_optimal_times(
                    soi, target_idx, game.solar_system.time, &game.solar_system.bodies,
                );
                if let Some((default_dep, default_arr)) = defaults {
                    let dep_time = default_dep + render_state.transfer_departure_offset;
                    let arr_time = default_arr + render_state.transfer_arrival_offset;
                    transfer::compute_interplanetary(
                        &ship_orbit.orbit,
                        ship_orbit.retrograde,
                        ship_orbit.mean_anomaly,
                        soi,
                        target_idx,
                        dep_time,
                        arr_time,
                        game.solar_system.time,
                        &game.solar_system.bodies,
                    ).map(|ip| {
                        let target_name = game.solar_system.bodies[target_idx].name.clone();
                        transfer::TransferDisplay {
                            mode: 1,
                            target_name,
                            departure_dv: ip.ejection_delta_v,
                            arrival_dv: ip.arrival_v_infinity,
                            transfer_time: ip.transfer_time,
                            current_phase_angle: ip.current_phase_angle.to_degrees(),
                            required_phase_angle: ip.required_phase_angle.to_degrees(),
                            time_to_window: ip.time_to_window,
                            departure_position_angle: ip.departure_position_angle,
                            prograde_dv: ip.ejection_delta_v,
                            radial_dv: 0.0,
                            valid: true,
                        }
                    })
                } else {
                    None
                }
            }
        } else {
            None
        };
    }

    // Compute time-to-node and burn time for the first maneuver node.
    // Computed live each frame from the ship's current orbit to avoid drift
    // over many orbits (numerical integration period != Keplerian period).
    render_state.time_to_node = None;
    render_state.burn_time = None;
    if let Some(first_node) = render_state.maneuver_nodes.first() {
        if let Some(ship_orbit) = game.flight.ship.get_cached_orbit() {
            if first_node.parent_idx == ship_orbit.parent_idx && ship_orbit.orbit.eccentricity < 1.0 {
                // Node's fixed inertial position angle
                let node_inertial = first_node.true_anomaly + first_node.argument_of_periapsis;
                // Project node onto ship's current orbit (same arg_peri for both → errors cancel)
                let node_ta = (node_inertial - ship_orbit.orbit.argument_of_periapsis)
                    .rem_euclid(std::f64::consts::TAU);
                let node_ma = game.flight.ship.true_to_mean_anomaly(&ship_orbit.orbit, node_ta);
                let ship_ma = ship_orbit.mean_anomaly;

                let delta_ma = if ship_orbit.retrograde {
                    (ship_ma - node_ma).rem_euclid(std::f64::consts::TAU)
                } else {
                    (node_ma - ship_ma).rem_euclid(std::f64::consts::TAU)
                };

                let parent_mass = game.solar_system.bodies[ship_orbit.parent_idx].effective_mass_at(ship_orbit.orbit.semi_major_axis);
                let mean_motion = ship_orbit.orbit.mean_motion(parent_mass);
                if mean_motion > 0.0 {
                    // Time to reach node on current orbit (0 to 1 period)
                    let time_one_pass = delta_ma / mean_motion;
                    let orbital_period = std::f64::consts::TAU / mean_motion;

                    // Use epoch to determine how many full orbits remain.
                    // The epoch gives the intended arrival time; we count how many
                    // complete orbits fit between now and the epoch, then add
                    // the within-orbit time for a drift-free countdown.
                    let epoch_remaining = first_node.epoch - game.simulation_time;
                    let full_orbits = if epoch_remaining > time_one_pass + orbital_period * 0.5 {
                        ((epoch_remaining - time_one_pass) / orbital_period).round() as u64
                    } else {
                        0
                    };

                    let ttn = time_one_pass + full_orbits as f64 * orbital_period;
                    if ttn > 0.0 {
                        render_state.time_to_node = Some(ttn);
                    }
                }
            }
        }

        // Compute burn time: dv / acceleration
        if let (Some(thrust_kn), Some(mass_tonnes)) = (render_state.vessel_thrust_kn, render_state.vessel_total_mass) {
            if thrust_kn > 0.0 && mass_tonnes > 0.0 {
                let accel = (thrust_kn * 1000.0) / (mass_tonnes * 1000.0); // m/s^2
                let remaining_dv = first_node.total_remaining_delta_v();
                render_state.burn_time = Some(remaining_dv / accel);
            }
        }
    }
    // Cancel warp-to-node if there are no nodes or no time computed
    if render_state.time_to_node.is_none() && render_state.warp_to_node {
        render_state.warp_to_node = false;
        game.warp_index = 0;
    }

    // Auto-warp-to-node logic
    if render_state.warp_to_node {
        if let Some(ttn) = render_state.time_to_node {
            let burn_half = render_state.burn_time.unwrap_or(0.0) / 2.0;
            let effective_time = ttn - burn_half;

            if effective_time <= 0.0 {
                // Past the burn start point — stop warping
                render_state.warp_to_node = false;
                game.warp_index = 0;
            } else if effective_time / WARP_LEVELS[5] < 1.0 {
                // Even minimum on-rails warp (100x) would arrive in < 1 real second
                // Drop to 1x and stop auto-warp
                render_state.warp_to_node = false;
                game.warp_index = 0;
            } else {
                // Find the highest warp level where we won't overshoot
                // (effective_time / warp_level >= 1.0 real second remaining)
                // Minimum auto-warp: index 5 (100x)
                let mut best_index = 5; // 100x minimum
                for i in (5..WARP_LEVELS.len()).rev() {
                    if effective_time / WARP_LEVELS[i] >= 1.0 {
                        best_index = i;
                        break;
                    }
                }
                game.warp_index = best_index;
            }
        }
    }

    let body_names: Vec<String> = game.solar_system.bodies.iter().map(|b| b.name.clone()).collect();
    let date_str = sunscatter::game::format_date(game.simulation_time);

    // Determine if the ship can safely exit flight (go to main menu)
    // Cannot exit if in atmosphere or in landing zone while suborbital (and not landed)
    let is_landed = matches!(game.flight.ship.state, ShipState::Landed { .. });
    let can_exit_flight = is_landed || (
        !game.flight.ship.in_atmosphere(&game.solar_system)
        && !(game.flight.ship.below_landing_altitude(&game.solar_system)
             && game.flight.ship.is_suborbital(&game.solar_system))
    );
    let can_recover = match game.flight.ship.state {
        ShipState::Landed { body_index, .. } => sunscatter::game::is_recoverable_body(body_index),
        _ => false,
    };

    let pre_render_warp_index = game.warp_index;
    match render_state.render(&body_names, WARP_LEVELS, game.warp_index, game.paused, &date_str, can_exit_flight, can_recover) {
        Ok((new_warp_index, pause_action)) => {
            game.warp_index = new_warp_index;
            // If user manually changed warp (clicked a button), cancel auto-warp
            if render_state.warp_to_node && new_warp_index != pre_render_warp_index {
                render_state.warp_to_node = false;
            }
            match pause_action {
                PauseAction::MainMenu => {
                    // Save active vessel to inactive list before leaving flight
                    let nodes = render_state.swap_maneuver_nodes(Vec::new());
                    game.flight.shelve_active_vessel(nodes, &game.solar_system);
                    game.enter_main_menu();
                }
                PauseAction::RecoverVessel => {
                    // Discard maneuver nodes (vessel is recovered, not shelved)
                    render_state.swap_maneuver_nodes(Vec::new());
                    game.flight.vessel = None;
                    log::info!("Recovered vessel: {} (id={})", game.flight.active_vessel_name, game.flight.active_vessel_id);
                    game.enter_main_menu();
                }
                PauseAction::Resume | PauseAction::None => {}
            }
        }
        Err(wgpu::SurfaceError::Lost) => {
            println!("Surface lost, resizing...");
            render_state.resize(render_state.size);
        }
        Err(wgpu::SurfaceError::OutOfMemory) => {
            eprintln!("Out of memory!");
            std::process::exit(1);
        }
        Err(e) => eprintln!("Render error: {:?}", e),
    }

    // Process transfer planner node creation request
    // The position_angle is an inertial angle; convert to true anomaly using
    // the trajectory segment's arg_peri (not the ship orbit's, which is ill-defined
    // for near-circular parking orbits).
    if let Some((position_angle, prograde, radial, time_to_window)) = render_state.transfer_node_request.take() {
        if let Some(segment) = render_state.current_trajectory.first() {
            let ta = sunscatter::ship::transfer::normalize_angle(position_angle - segment.argument_of_periapsis);
            let dv = sunscatter::render::ManeuverDeltaV { prograde, radial_out: radial };
            let epoch = game.simulation_time + time_to_window;
            let seg = segment.clone();
            render_state.create_maneuver_node_with_epoch(ta, &seg, dv, epoch);
        }
        render_state.transfer_planner_open = false;
    }

    // Process engine toggle request from part info popup
    if let Some((part_idx, enabled)) = render_state.engine_toggle_request.take() {
        if let Some(ref mut vessel) = game.flight.vessel {
            if part_idx < vessel.parts.len() {
                vessel.parts[part_idx].engine_enabled = enabled;
            }
        }
    }

    // Process crossfeed toggle request from part info popup
    if let Some((part_idx, enabled)) = render_state.crossfeed_toggle_request.take() {
        if let Some(ref mut vessel) = game.flight.vessel {
            if part_idx < vessel.parts.len() {
                vessel.parts[part_idx].crossfeed_enabled = enabled;
            }
        }
    }

    // Process manual decouple request from part info popup
    if let Some(part_idx) = render_state.decouple_request.take() {
        if let Some(ref mut vessel) = game.flight.vessel {
            if part_idx < vessel.parts.len() && !vessel.parts[part_idx].decoupled {
                let def = game.part_definitions.get(&vessel.parts[part_idx].definition_id);
                if let Some(def) = def {
                    if let Some(ref dec_data) = def.decoupler {
                        // Store ejection force for handle_post_decouple
                        vessel.last_decouple_force = dec_data.ejection_force;

                        let decoupler_bottom = vessel.parts[part_idx].local_position[1]
                            - def.hitbox_height() / 2.0;

                        // Mark the decoupler itself as decoupled
                        vessel.parts[part_idx].decoupled = true;

                        // Mark all parts whose top edge is at or below the decoupler bottom
                        for i in 0..vessel.parts.len() {
                            if i == part_idx || vessel.parts[i].decoupled {
                                continue;
                            }
                            let other_def = game.part_definitions.get(&vessel.parts[i].definition_id);
                            let other_top = if let Some(od) = other_def {
                                vessel.parts[i].local_position[1] + od.hitbox_height() / 2.0
                            } else {
                                vessel.parts[i].local_position[1] + vessel.parts[i].hitbox_half_extents[1]
                            };
                            if other_top <= decoupler_bottom + 0.01 {
                                vessel.parts[i].decoupled = true;
                            }
                        }
                    }
                }
            }
        }
        handle_post_decouple(game);
        render_state.selected_flight_part = None;
    }

    // Process fairing deploy request from part info popup
    if let Some(part_idx) = render_state.fairing_deploy_request.take() {
        if let Some(ref mut vessel) = game.flight.vessel {
            if part_idx < vessel.parts.len() && !vessel.parts[part_idx].decoupled {
                let def = game.part_definitions.get(&vessel.parts[part_idx].definition_id);
                if let Some(def) = def {
                    if let Some(ref fairing_data) = def.fairing {
                        vessel.last_decouple_force = fairing_data.ejection_force;
                        vessel.parts[part_idx].decoupled = true;
                    }
                }
            }
        }
        handle_post_decouple(game);
        render_state.selected_flight_part = None;
    }

    // Process debug teleport to LEO
    if render_state.debug_teleport_leo {
        render_state.debug_teleport_leo = false;
        let earth_idx = sunscatter::game::LAUNCHPAD_BODY_INDEX;
        let earth = &game.solar_system.bodies[earth_idx];
        let leo_alt = 4.0e5; // 400 km
        let r = earth.radius + leo_alt;
        let mu = sunscatter::bodies::G * earth.mass;
        let v_orb = (mu / r).sqrt();
        // Place at +Y, moving -X (counterclockwise, matching solar system convention)
        game.flight.ship.rel_position = [0.0, r];
        game.flight.ship.rel_velocity = [-v_orb, 0.0];
        game.flight.ship.soi_body = earth_idx;
        game.flight.ship.state = ShipState::Flying;
        game.flight.ship.on_rails = false;
        if let Some(ref mut vessel) = game.flight.vessel {
            vessel.rel_position = game.flight.ship.rel_position;
            vessel.rel_velocity = game.flight.ship.rel_velocity;
        }
    }

    // Process staging reorder request from flight staging panel
    if let Some(new_stages) = render_state.staging_reorder.take() {
        if let Some(ref mut vessel) = game.flight.vessel {
            vessel.stages = new_stages;
        }
    }

    // Per-part thermal destruction and vessel splitting
    let debris_list = if let Some(ref mut vessel) = game.flight.vessel {
        let destroyed_parts = vessel.destroy_overheated_parts();
        if !destroyed_parts.is_empty() {
            // Clean up staging lists — remove destroyed part indices
            for stage in &mut vessel.stages {
                stage.retain(|idx| !destroyed_parts.contains(idx));
            }

            // Check for vessel split — collect debris before creating ships
            let debris = vessel.check_and_split(&game.part_definitions);

            // Recenter on new COM
            let com_offset = vessel.recenter_on_com(&game.part_definitions);
            let rot = game.flight.ship.rotation;
            game.flight.ship.rel_position[0] += com_offset[0] * rot.cos() - com_offset[1] * rot.sin();
            game.flight.ship.rel_position[1] += com_offset[0] * rot.sin() + com_offset[1] * rot.cos();

            debris
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Create debris vessels outside the vessel borrow (thermal breakup — no ejection force)
    for (debris_vessel, com_offset) in debris_list {
        game.flight.create_debris_vessel(debris_vessel, com_offset, 0.0, &game.solar_system);
    }

    // Remove vessel if all parts destroyed
    if game.flight.vessel.as_ref().map(|v| !v.parts.iter().any(|p| !p.destroyed && !p.decoupled)).unwrap_or(false) {
        game.flight.vessel = None;
        game.flight.ship.temperature = AMBIENT_TEMPERATURE;
        game.flight.ship.heat_flux = 0.0;
        log::info!("Vessel completely destroyed by aerodynamic heating!");
    }
}

/// Render an editor mode frame
fn render_editor_frame(
    game: &mut Game,
    render_state: &mut RenderState,
    dt: f32,
) {
    // Update camera position based on held keys
    game.editor.update_camera(dt);

    let screen_width = render_state.size.width as f32;
    let screen_height = render_state.size.height as f32;

    // Set editor camera
    render_state.set_editor_camera(game.editor.camera_offset, game.editor.camera_zoom);

    // Generate editor geometry
    let mut vertices: Vec<Vertex> = Vec::new();

    // Grid
    vertices.extend(generate_grid_vertices(&game.editor, screen_width, screen_height));

    // Placed parts
    vertices.extend(generate_part_vertices(&game.editor, &game.part_definitions, Some(&render_state.sprite_atlas)));

    // Ghost preview
    vertices.extend(generate_ghost_vertices(&game.editor, &game.part_definitions, Some(&render_state.sprite_atlas)));

    // Get blueprint names for load dialog
    let blueprint_names: Vec<&str> = game.blueprints.names();

    // Clone what we need for the closure
    let part_defs = game.part_definitions.clone();

    // Calculate ship stats
    let stats = game.editor.calculate_stats(&part_defs);

    // Build body info list for TWR dropdown
    let bodies: Vec<BodyInfo> = game.solar_system.bodies.iter()
        .map(|b| BodyInfo {
            name: b.name.clone(),
            surface_gravity: b.surface_gravity(),
        })
        .collect();

    // Render with editor UI
    let mut action = EditorAction::None;
    let mut editor_pause_action = PauseAction::None;
    let paused = game.paused;

    let stage_delta_vs = game.editor.calculate_stage_delta_v(&part_defs);

    let result = render_state.render_editor(&vertices, |ctx| {
        action = render_editor_ui(
            ctx,
            &mut game.editor,
            &part_defs,
            &blueprint_names,
            &stats,
            &bodies,
            &stage_delta_vs,
        );

        // Pause overlay on top of editor
        if paused {
            egui::Area::new(egui::Id::new("pause_overlay"))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                        .inner_margin(egui::Margin::same(40.0))
                        .rounding(egui::Rounding::same(8.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading(egui::RichText::new("Paused").size(32.0).color(egui::Color32::WHITE));
                                ui.add_space(20.0);
                                if ui.button(egui::RichText::new("Main Menu").size(18.0)).clicked() {
                                    editor_pause_action = PauseAction::MainMenu;
                                }
                            });
                        });
                });
        }
    });

    // Handle editor actions
    match action {
        EditorAction::Launch => {
            game.flight.recover_vessels_on_launchpad(&game.solar_system);
            match game.launch_from_editor() {
                Ok(()) => {
                    // Zoom camera to see the vessel on the surface
                    if let Some(ref vessel) = game.flight.vessel {
                        let vessel_world_size = vessel.bounding_half_height() * 2.0 * SCALE * BODY_SCALE;
                        // We want the vessel to take up ~1/4 of the screen height
                        // pixels = vessel_world_size * zoom * screen_height / 2
                        // We want pixels ≈ screen_height / 4
                        // So zoom = screen_height/4 / (vessel_world_size * screen_height/2)
                        //         = 1 / (2 * vessel_world_size)
                        let target_fraction = 0.25;
                        let zoom = target_fraction / vessel_world_size as f32;
                        render_state.camera.zoom = zoom;
                    }
                    log::info!("Launched vessel");
                }
                Err(e) => log::error!("Failed to launch: {}", e),
            }
        }
        EditorAction::SaveBlueprint(name) => {
            match game.save_blueprint(name) {
                Ok(()) => log::info!("Blueprint saved"),
                Err(e) => log::error!("Failed to save: {}", e),
            }
        }
        EditorAction::LoadBlueprint(name) => {
            match game.load_blueprint(&name) {
                Ok(()) => log::info!("Blueprint loaded"),
                Err(e) => log::error!("Failed to load: {}", e),
            }
        }
        EditorAction::DeleteBlueprint(name) => {
            match game.blueprints.delete(&name) {
                Ok(()) => log::info!("Blueprint deleted: {}", name),
                Err(e) => log::error!("Failed to delete blueprint: {}", e),
            }
        }
        EditorAction::NewVessel => {
            game.new_vessel();
        }
        EditorAction::ExitToFlight => {
            println!("Exiting to flight mode...");
            game.enter_flight();
            println!("Now in flight mode: {:?}", game.mode);
        }
        EditorAction::None => {}
    }

    // Handle pause action
    match editor_pause_action {
        PauseAction::MainMenu => game.enter_main_menu(),
        PauseAction::Resume | PauseAction::None | PauseAction::RecoverVessel => {}
    }

    // Process any pending part deletions
    game.editor.process_pending_delete();

    if let Err(e) = result {
        match e {
            wgpu::SurfaceError::Lost => render_state.resize(render_state.size),
            wgpu::SurfaceError::OutOfMemory => std::process::exit(1),
            e => eprintln!("Editor render error: {:?}", e),
        }
    }
}

/// Compute scaled body positions for rendering (shared across flight, main menu, tracking station)
fn compute_scaled_positions(game: &Game) -> Vec<[f64; 2]> {
    let mut scaled: Vec<[f64; 2]> = Vec::with_capacity(game.solar_system.bodies.len());
    for i in 0..game.solar_system.bodies.len() {
        let pos = game.solar_system.body_position(i);
        let body = &game.solar_system.bodies[i];
        let scaled_pos = if let Some(parent_idx) = body.parent {
            let parent_scaled = scaled[parent_idx];
            let parent_unscaled = game.solar_system.body_position(parent_idx);
            let rel_x = pos[0] - parent_unscaled[0];
            let rel_y = pos[1] - parent_unscaled[1];
            [
                parent_scaled[0] + rel_x * BODY_SCALE,
                parent_scaled[1] + rel_y * BODY_SCALE,
            ]
        } else {
            pos
        };
        scaled.push(scaled_pos);
    }
    scaled
}

/// Walk up the body hierarchy to find the nearest star (child of the root body).
/// Returns the body index itself if it's already a star or the root.
fn find_star_ancestor(game: &Game, mut idx: usize) -> usize {
    loop {
        let body = &game.solar_system.bodies[idx];
        match body.parent {
            None => return idx,                                    // Root body
            Some(p) if game.solar_system.bodies[p].parent.is_none() => return idx, // Star (parent is root)
            Some(p) => idx = p,                                    // Walk up
        }
    }
}

/// Build body render tuples from scaled positions
fn build_body_data(game: &Game, scaled_positions: &[[f64; 2]], in_galaxy_view: bool) -> Vec<(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)> {
    (0..game.solar_system.bodies.len())
        .map(|i| {
            let body = &game.solar_system.bodies[i];
            let pos = scaled_positions[i];
            // In galaxy view, only show root + its direct children (stars)
            let visible = if in_galaxy_view {
                body.parent.is_none() || body.parent.map_or(false, |p|
                    game.solar_system.bodies[p].parent.is_none())
            } else {
                true
            };
            let radius = if visible { body.radius * BODY_SCALE } else { 0.0 };
            let atmo_height = if visible { body.atmosphere.map(|a| a.visible_height()).unwrap_or(0.0) } else { 0.0 };
            let atmo_color = body.atmosphere.map(|a| a.color).unwrap_or([0.0; 3]);
            (pos[0], pos[1], radius, body.color, atmo_height, atmo_color, i)
        })
        .collect()
}

/// Build accretion disc render data from game bodies
fn build_accretion_disc_data(game: &Game) -> Vec<Option<sunscatter::bodies::AccretionDisc>> {
    game.solar_system.bodies.iter()
        .map(|b| b.accretion_disc)
        .collect()
}

/// Build orbit render data from scaled positions
fn build_orbit_data(game: &Game, scaled_positions: &[[f64; 2]], render_state: &RenderState) -> Vec<Option<OrbitRenderData>> {
    let pixels_per_world_unit = render_state.camera.zoom * render_state.size.height as f32 / 2.0;
    let in_galaxy_view = is_galaxy_view(render_state.camera.zoom, render_state.size.height);
    (0..game.solar_system.bodies.len())
        .map(|i| {
            let body = &game.solar_system.bodies[i];
            match (body.parent, &body.orbit) {
                (Some(parent_idx), Some(orbit)) => {
                    let parent_body = &game.solar_system.bodies[parent_idx];
                    if parent_body.parent.is_none() {
                        // Star orbit: only show in galaxy view when this star is tracked
                        if in_galaxy_view && render_state.tracked_body == Some(i) {
                            let parent_pos = scaled_positions[parent_idx];
                            let orbit_color = [
                                body.color[0] * 0.4,
                                body.color[1] * 0.4,
                                body.color[2] * 0.4,
                                0.5,
                            ];
                            return Some(OrbitRenderData {
                                parent_x: parent_pos[0] * SCALE,
                                parent_y: parent_pos[1] * SCALE,
                                semi_major_axis: orbit.semi_major_axis * SCALE * BODY_SCALE,
                                eccentricity: orbit.eccentricity,
                                argument_of_periapsis: orbit.argument_of_periapsis,
                                color: orbit_color,
                            });
                        }
                        return None;
                    }

                    // In galaxy view, skip all non-star orbits
                    if in_galaxy_view {
                        return None;
                    }

                    let body_world_radius = (body.radius * BODY_SCALE * SCALE) as f32;
                    let body_pixels = body_world_radius * pixels_per_world_unit * 2.0;
                    let is_moon = parent_body.parent
                        .map_or(false, |gp| game.solar_system.bodies[gp].parent.is_some());
                    let pixel_threshold = if is_moon { 100.0 } else { 5.0 };
                    if body_pixels >= pixel_threshold {
                        return None;
                    }
                    let parent_pos = scaled_positions[parent_idx];
                    let orbit_color = [
                        body.color[0] * 0.4,
                        body.color[1] * 0.4,
                        body.color[2] * 0.4,
                        0.5,
                    ];
                    Some(OrbitRenderData {
                        parent_x: parent_pos[0] * SCALE,
                        parent_y: parent_pos[1] * SCALE,
                        semi_major_axis: orbit.semi_major_axis * SCALE * BODY_SCALE,
                        eccentricity: orbit.eccentricity,
                        argument_of_periapsis: orbit.argument_of_periapsis,
                        color: orbit_color,
                    })
                }
                _ => None,
            }
        })
        .collect()
}

/// Build part render data from a FlightVessel for rendering inactive vessels
fn build_vessel_part_render_data(
    vessel: &sunscatter::parts::FlightVessel,
    part_defs: &sunscatter::parts::PartDefinitions,
) -> Vec<ShipPartRenderData> {
    vessel.parts.iter()
        .enumerate()
        .filter(|(_, p)| !p.destroyed && !p.decoupled)
        .map(|(i, p)| {
            let def = part_defs.get(&p.definition_id);
            let name = def.map(|d| d.name.clone()).unwrap_or_else(|| p.definition_id.clone());
            let dry_mass = def.map(|d| d.mass).unwrap_or(0.0);
            let is_engine = p.propellant_type.is_some();
            ShipPartRenderData {
                definition_id: p.definition_id.clone(),
                local_x: p.local_position[0],
                local_y: p.local_position[1],
                rotation: p.rotation,
                engine_active: false, // Inactive vessels don't fire engines
                part_index: i,
                name,
                dry_mass,
                hitbox_half_w: if is_part_rotation_swapped(p.rotation) { p.hitbox_half_extents[1] } else { p.hitbox_half_extents[0] },
                hitbox_half_h: if is_part_rotation_swapped(p.rotation) { p.hitbox_half_extents[0] } else { p.hitbox_half_extents[1] },
                click_local_y: p.local_position[1],
                click_hitbox_half_h: if is_part_rotation_swapped(p.rotation) { p.hitbox_half_extents[0] } else { p.hitbox_half_extents[1] },
                engine_thrust_vac: if is_engine { Some(p.engine_thrust_vac) } else { None },
                engine_thrust_asl: if is_engine { Some(p.engine_thrust_asl) } else { None },
                engine_isp_vac: if is_engine { Some(p.engine_isp_vac) } else { None },
                engine_isp_asl: if is_engine { Some(p.engine_isp_asl) } else { None },
                engine_enabled: p.engine_enabled,
                propellant_name: p.propellant_type.map(|pt| pt.display_name().to_string()),
                fuel_type_name: None,
                fuel_current: None,
                fuel_max: None,
                ox_current: None,
                ox_max: None,
                crew_capacity: def.and_then(|d| d.pod.as_ref().map(|pod| pod.crew_capacity)),
                monoprop_current: None,
                monoprop_max: None,
                battery_current: if p.max_electricity > 0.0 { Some(p.electricity) } else { None },
                battery_max: if p.max_electricity > 0.0 { Some(p.max_electricity) } else { None },
                solar_output: None, // Inactive vessels don't compute solar output
                rtg_output: def.and_then(|d| d.rtg.as_ref().map(|r| r.output_watts)),
                is_decoupler: def.map(|d| d.decoupler.is_some()).unwrap_or(false),
                crossfeed_enabled: p.crossfeed_enabled,
                gimbal_angle: 0.0,
                rcs_thrust: if p.rcs_thrust > 0.0 { Some(p.rcs_thrust) } else { None },
                rcs_nozzle_state: None,
                heat_fraction: ((p.temperature - 300.0) / (p.max_heat_tolerance - 300.0)).clamp(0.0, 1.0) as f32,
                temperature: p.temperature,
                is_fairing: def.map(|d| d.fairing.is_some()).unwrap_or(false),
                fairing_shape: p.fairing_shape.clone(),
                fairing_half: p.fairing_half,
            }
        })
        .collect()
}

/// Build tracking vessel data for all vessels (active + inactive)
fn build_tracking_vessel_data(
    game: &Game,
    scaled_positions: &[[f64; 2]],
) -> Vec<sunscatter::render::TrackingVesselData> {
    use sunscatter::render::TrackingVesselData;

    let mut vessels = Vec::new();

    // All vessels are in inactive_vessels when not in flight
    for v in &game.flight.inactive_vessels {
        // Use scaled_positions + rel_position for precision at galaxy-scale distances
        let soi_pos = scaled_positions[v.ship.soi_body];
        let rel = v.ship.rel_position;
        let orbit_data = v.ship.get_render_orbit().map(|(orbit, parent_idx)| {
            let parent_pos = scaled_positions[parent_idx];
            sunscatter::render::OrbitRenderData {
                parent_x: parent_pos[0] * SCALE,
                parent_y: parent_pos[1] * SCALE,
                semi_major_axis: orbit.semi_major_axis * SCALE * BODY_SCALE,
                eccentricity: orbit.eccentricity,
                argument_of_periapsis: orbit.argument_of_periapsis,
                color: [0.6, 0.6, 0.6, 0.4], // Grey for all vessels in tracking station
            }
        });
        let parts = v.vessel.as_ref().map(|fv| {
            build_vessel_part_render_data(fv, &game.part_definitions)
        });
        vessels.push(TrackingVesselData {
            id: v.id,
            name: v.name.clone(),
            color: v.ship.color,
            x: soi_pos[0] * SCALE + rel[0] * SCALE * BODY_SCALE,
            y: soi_pos[1] * SCALE + rel[1] * SCALE * BODY_SCALE,
            body_center: [soi_pos[0] * SCALE, soi_pos[1] * SCALE],
            rel_offset: [rel[0] * SCALE * BODY_SCALE, rel[1] * SCALE * BODY_SCALE],
            soi_body: v.ship.soi_body,
            orbit: orbit_data,
            parts,
            rotation: v.ship.rotation,
        });
    }

    vessels
}

/// Render a main menu frame
fn render_main_menu_frame(
    game: &mut Game,
    render_state: &mut RenderState,
    _elwt: &winit::event_loop::EventLoopWindowTarget<()>,
) {
    // Always keep camera focused on the Sun at a fixed zoom
    let sun_pos = game.solar_system.body_position(1);
    render_state.camera.position[0] = sun_pos[0] * SCALE * BODY_SCALE;
    render_state.camera.position[1] = sun_pos[1] * SCALE * BODY_SCALE;
    render_state.camera.body_center = render_state.camera.position;
    render_state.camera.ship_offset = [0.0, 0.0];
    render_state.camera.zoom = 0.002;

    if !game.paused {
        let dt = 1.0 / 60.0;
        let time_warp = WARP_LEVELS[game.warp_index];
        game.solar_system.update(dt * time_warp);
        game.simulation_time += dt * time_warp;

        // Propagate all vessels on rails (no active vessel while not in flight)
        let dt_sim = dt * time_warp;
        for vessel in &mut game.flight.inactive_vessels {
            if !vessel.ship.on_rails {
                vessel.ship.enter_rails_mode(&game.solar_system);
            }
            vessel.ship.update_on_rails(dt_sim, &game.solar_system);
        }
        game.flight.inactive_vessels.retain(|v| {
            let in_landing_zone = v.ship.in_atmosphere(&game.solar_system)
                || v.ship.below_landing_altitude(&game.solar_system);
            !(v.ship.periapsis_below_surface(&game.solar_system) && in_landing_zone)
        });
    }

    let scaled_positions = compute_scaled_positions(game);
    let bodies = build_body_data(game, &scaled_positions, false);
    let orbits = build_orbit_data(game, &scaled_positions, render_state);

    // Update camera tracking (body focus)
    render_state.update_tracking(&scaled_positions, SCALE);

    // Update body/orbit geometry (no ship)
    render_state.update_bodies_orbits_and_ship(&bodies, &orbits, None, SCALE, None);

    let paused = game.paused;
    let date_str = sunscatter::game::format_date(game.simulation_time);

    match render_state.render_main_menu(WARP_LEVELS, game.warp_index, &date_str, |ctx| {
        let mut action = MainMenuAction::None;

        if paused {
            // Pause overlay
            egui::Area::new(egui::Id::new("pause_overlay"))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                        .inner_margin(egui::Margin::same(40.0))
                        .rounding(egui::Rounding::same(8.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading(egui::RichText::new("Paused").size(32.0).color(egui::Color32::WHITE));
                                ui.add_space(20.0);
                                if ui.button(egui::RichText::new("Exit Game").size(18.0)).clicked() {
                                    std::process::exit(0);
                                }
                            });
                        });
                });
        } else {
            // Main menu centered
            egui::Area::new(egui::Id::new("main_menu"))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                        .inner_margin(egui::Margin::same(40.0))
                        .rounding(egui::Rounding::same(8.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading(egui::RichText::new("Sunscatter").size(36.0).color(egui::Color32::WHITE));
                                ui.add_space(30.0);
                                if ui.button(egui::RichText::new("Editor").size(20.0)).clicked() {
                                    action = MainMenuAction::Editor;
                                }
                                ui.add_space(10.0);
                                if ui.button(egui::RichText::new("Tracking Station").size(20.0)).clicked() {
                                    action = MainMenuAction::TrackingStation;
                                }
                            });
                        });
                });
        }

        action
    }) {
        Ok((new_warp_index, menu_action)) => {
            game.warp_index = new_warp_index;
            match menu_action {
                MainMenuAction::Editor => game.enter_editor(),
                MainMenuAction::TrackingStation => {
                    game.enter_tracking_station();
                    render_state.focus_on_body(sunscatter::game::LAUNCHPAD_BODY_INDEX);
                    // Zoom so Earth fills ~half the screen
                    let earth_radius_world = game.solar_system.bodies[sunscatter::game::LAUNCHPAD_BODY_INDEX].radius * SCALE * BODY_SCALE;
                    render_state.camera.zoom = (0.25 / earth_radius_world) as f32;
                },
                MainMenuAction::None => {}
            }
        }
        Err(wgpu::SurfaceError::Lost) => render_state.resize(render_state.size),
        Err(wgpu::SurfaceError::OutOfMemory) => std::process::exit(1),
        Err(e) => eprintln!("Main menu render error: {:?}", e),
    }
}

/// Render a tracking station frame
fn render_tracking_station_frame(
    game: &mut Game,
    render_state: &mut RenderState,
) {
    if !game.paused {
        let dt = 1.0 / 60.0;
        let time_warp = WARP_LEVELS[game.warp_index];
        game.solar_system.update(dt * time_warp);
        game.simulation_time += dt * time_warp;

        // Propagate all vessels on rails (no active vessel while not in flight)
        let dt_sim = dt * time_warp;
        for vessel in &mut game.flight.inactive_vessels {
            if !vessel.ship.on_rails {
                vessel.ship.enter_rails_mode(&game.solar_system);
            }
            vessel.ship.update_on_rails(dt_sim, &game.solar_system);
        }
        game.flight.inactive_vessels.retain(|v| {
            let in_landing_zone = v.ship.in_atmosphere(&game.solar_system)
                || v.ship.below_landing_altitude(&game.solar_system);
            !(v.ship.periapsis_below_surface(&game.solar_system) && in_landing_zone)
        });
    }

    let scaled_positions = compute_scaled_positions(game);
    let in_galaxy_view = is_galaxy_view(render_state.camera.zoom, render_state.size.height);

    // In galaxy view, redirect tracked planet/moon to its parent star
    if in_galaxy_view {
        if let Some(tracked_idx) = render_state.tracked_body {
            let star_idx = find_star_ancestor(game, tracked_idx);
            if star_idx != tracked_idx {
                render_state.tracked_body = Some(star_idx);
            }
        }
    }

    let bodies = build_body_data(game, &scaled_positions, in_galaxy_view);
    let orbits = build_orbit_data(game, &scaled_positions, render_state);

    // Update camera tracking (body or vessel focus)
    render_state.update_tracking(&scaled_positions, SCALE);

    // Build vessel tracking data
    let tracking_vessels = build_tracking_vessel_data(game, &scaled_positions);

    // Update camera tracking for focused vessel
    if let Some(vessel_id) = render_state.tracked_vessel {
        if let Some(vessel_data) = tracking_vessels.iter().find(|v| v.id == vessel_id) {
            // Use SOI body center + vessel offset for precision
            let soi_pos = scaled_positions[vessel_data.soi_body];
            render_state.camera.body_center = [soi_pos[0] * SCALE, soi_pos[1] * SCALE];
            // vessel_data.x/y already include the offset; recover it by subtraction
            render_state.camera.ship_offset = [
                vessel_data.x - render_state.camera.body_center[0],
                vessel_data.y - render_state.camera.body_center[1],
            ];
            render_state.camera.position[0] = vessel_data.x;
            render_state.camera.position[1] = vessel_data.y;
        } else {
            // Tracked vessel was destroyed, focus on Earth
            render_state.tracked_vessel = None;
            render_state.focus_on_body(sunscatter::game::LAUNCHPAD_BODY_INDEX);
        }
    }

    let accretion_discs = build_accretion_disc_data(game);
    render_state.update_bodies_orbits_ship_and_vessels(&bodies, &orbits, None, SCALE, Some(&game.part_definitions), &tracking_vessels, &accretion_discs, in_galaxy_view);

    let body_names: Vec<String> = game.solar_system.bodies.iter().map(|b| b.name.clone()).collect();
    let date_str = sunscatter::game::format_date(game.simulation_time);
    let active_id = game.flight.active_vessel_id;

    // Build body info data for the info panel
    let body_info: Vec<BodyInfoData> = game.solar_system.bodies.iter().map(|body| {
        let orbit_period_s = body.orbit.as_ref().and_then(|orbit| {
            body.parent.map(|pi| {
                let parent_mass = game.solar_system.bodies[pi].effective_mass_at(orbit.semi_major_axis);
                let mu = G * parent_mass;
                std::f64::consts::TAU * (orbit.semi_major_axis.powi(3) / mu).sqrt()
            })
        });
        BodyInfoData {
            name: body.name.clone(),
            description: body.description.clone(),
            radius_m: body.radius,
            surface_gravity_ms2: body.surface_gravity(),
            mass_kg: body.mass,
            atmosphere_pressure_pa: body.atmosphere.as_ref().map(|a| a.surface_pressure),
            atmosphere_height_m: body.atmosphere.as_ref().map(|a| a.visible_height()),
            orbit_semi_major_axis_m: body.orbit.as_ref().map(|o| o.semi_major_axis),
            orbit_eccentricity: body.orbit.as_ref().map(|o| o.eccentricity),
            orbit_period_s,
        }
    }).collect();

    match render_state.render_tracking_station(&body_names, WARP_LEVELS, game.warp_index, game.paused, &date_str, &tracking_vessels, active_id, &body_info) {
        Ok((new_warp_index, pause_action, ts_action)) => {
            game.warp_index = new_warp_index;
            match pause_action {
                PauseAction::MainMenu => game.enter_main_menu(),
                PauseAction::Resume | PauseAction::None | PauseAction::RecoverVessel => {}
            }
            // Handle tracking station actions
            match ts_action {
                sunscatter::render::TrackingStationAction::FlyVessel(id) => {
                    // Pull vessel from inactive list and enter flight
                    match game.flight.activate_vessel(id, &game.solar_system) {
                        Ok(new_nodes) => {
                            render_state.swap_maneuver_nodes(new_nodes);
                            game.warp_index = 0;
                        }
                        Err(e) => log::error!("Failed to activate vessel: {}", e),
                    }
                    game.enter_flight();
                }
                sunscatter::render::TrackingStationAction::FocusVessel(id) => {
                    // Focus camera on vessel and track it continuously
                    if let Some(vessel_data) = tracking_vessels.iter().find(|v| v.id == id) {
                        let soi_pos = scaled_positions[vessel_data.soi_body];
                        render_state.camera.body_center = [soi_pos[0] * SCALE, soi_pos[1] * SCALE];
                        render_state.camera.ship_offset = [
                            vessel_data.x - render_state.camera.body_center[0],
                            vessel_data.y - render_state.camera.body_center[1],
                        ];
                        render_state.camera.position[0] = vessel_data.x;
                        render_state.camera.position[1] = vessel_data.y;
                        render_state.tracked_body = None;
                        render_state.tracked_vessel = Some(id);
                    }
                }
                sunscatter::render::TrackingStationAction::DeleteVessel(id) => {
                    game.flight.inactive_vessels.retain(|v| v.id != id);
                    // If we were tracking the deleted vessel, stop tracking
                    if render_state.tracked_vessel == Some(id) {
                        render_state.tracked_vessel = None;
                        render_state.focus_on_body(sunscatter::game::LAUNCHPAD_BODY_INDEX);
                    }
                }
                sunscatter::render::TrackingStationAction::None => {}
            }
        }
        Err(wgpu::SurfaceError::Lost) => render_state.resize(render_state.size),
        Err(wgpu::SurfaceError::OutOfMemory) => std::process::exit(1),
        Err(e) => eprintln!("Tracking station render error: {:?}", e),
    }
}

/// Handle flight mode mouse input
fn handle_flight_mouse_input(
    game: &mut Game,
    render_state: &mut RenderState,
    state: ElementState,
    button: MouseButton,
    egui_consumed: bool,
    last_click_time: &mut Option<Instant>,
    last_click_pos: &mut [f32; 2],
) {
    const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(300);
    const DOUBLE_CLICK_DIST: f32 = 10.0;

    // Part selection runs unconditionally so clicking on a part always works,
    // even when the part info window overlaps the ship and egui consumes the click.
    let part_clicked = if button == MouseButton::Left && state == ElementState::Pressed {
        let mouse_pos = render_state.camera.last_mouse_pos;
        if let Some(cache_idx) = render_state.flight_part_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
            render_state.selected_flight_part = Some(cache_idx);
            true
        } else {
            false
        }
    } else {
        false
    };

    if !egui_consumed && button == MouseButton::Left {
        if state == ElementState::Pressed {
            let now = Instant::now();
            let mouse_pos = render_state.camera.last_mouse_pos;
            let dx = mouse_pos[0] - last_click_pos[0];
            let dy = mouse_pos[1] - last_click_pos[1];
            let dist = (dx * dx + dy * dy).sqrt();

            if !part_clicked {
                // Check if clicking on a maneuver node
                if let Some(node_id) = render_state.maneuver_node_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                    render_state.start_dragging_node(node_id);
                    render_state.selected_maneuver_node = Some(node_id);
                    render_state.pending_orbit_click = None;
                    render_state.selected_flight_part = None;
                } else {
                    render_state.selected_flight_part = None;
                }
            }

            // Helper: single-click target/orbit detection
            let single_click = |game: &Game, render_state: &mut RenderState, mouse_pos: [f32; 2]| {
                // Check body click → show target popup
                if let Some(body_idx) = render_state.body_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                    let name = game.solar_system.bodies[body_idx].name.clone();
                    render_state.target_popup = Some(TargetPopup {
                        target: SelectedTarget::Body(body_idx),
                        name,
                    });
                    render_state.pending_orbit_click = None;
                }
                // Check background vessel click → show target popup
                else if let Some(vessel_id) = render_state.background_vessel_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                    // Find vessel name from tracking data
                    let name = game.flight.inactive_vessels.iter()
                        .find(|v| v.id == vessel_id)
                        .map(|v| v.name.clone())
                        .unwrap_or_else(|| format!("Vessel {}", vessel_id));
                    render_state.target_popup = Some(TargetPopup {
                        target: SelectedTarget::Vessel(vessel_id),
                        name,
                    });
                    render_state.pending_orbit_click = None;
                }
                // Check orbit click → pending maneuver node
                else if render_state.dragging_maneuver_node.is_none() {
                    render_state.target_popup = None;
                    if let Some(orbit_pos) = render_state.orbit_click_position(mouse_pos[0], mouse_pos[1]) {
                        render_state.pending_orbit_click = Some(orbit_pos);
                        render_state.selected_maneuver_node = None;
                    } else {
                        render_state.pending_orbit_click = None;
                    }
                }
            };

            // Check for double-click on body or vessel
            if let Some(last_time) = *last_click_time {
                if now.duration_since(last_time) < DOUBLE_CLICK_TIME && dist < DOUBLE_CLICK_DIST {
                    // Double-click: focus camera on body or switch to vessel
                    render_state.target_popup = None;
                    if let Some(vessel_id) = render_state.background_vessel_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                        switch_to_next_vessel_by_id(game, render_state, vessel_id);
                    } else if let Some(body_idx) = render_state.body_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                        render_state.focus_on_body(body_idx);
                        game.flight.tracking_ship = false;
                        println!("Focused on: {}", game.solar_system.bodies[body_idx].name);
                    }
                    *last_click_time = None;
                } else {
                    single_click(game, render_state, mouse_pos);
                    *last_click_time = Some(now);
                    *last_click_pos = mouse_pos;
                }
            } else {
                single_click(game, render_state, mouse_pos);
                *last_click_time = Some(now);
                *last_click_pos = mouse_pos;
            }
        } else {
            render_state.stop_dragging_node();
        }
    }
}

/// Handle editor mode mouse input
fn handle_editor_mouse_input(
    game: &mut Game,
    render_state: &mut RenderState,
    state: ElementState,
    button: MouseButton,
    egui_consumed: bool,
) {
    if egui_consumed {
        return;
    }

    let mouse_pos = render_state.camera.last_mouse_pos;
    let screen_width = render_state.size.width as f32;
    let screen_height = render_state.size.height as f32;

    // Fairing build mode intercepts all mouse input
    if game.editor.fairing_build_mode.is_some() {
        if button == MouseButton::Left && state == ElementState::Pressed {
            game.editor.add_fairing_vertex(&game.part_definitions);
        } else if button == MouseButton::Right && state == ElementState::Pressed {
            game.editor.undo_fairing_vertex();
        }
        return;
    }

    if button == MouseButton::Left {
        if state == ElementState::Pressed {
            // Check if clicking on a placed part - start dragging it
            if let Some(part_id) = part_at_screen_pos(
                mouse_pos[0], mouse_pos[1],
                screen_width, screen_height,
                &game.editor, &game.part_definitions
            ) {
                let mouse_world = screen_to_world(mouse_pos[0], mouse_pos[1], screen_width, screen_height, &game.editor);
                game.editor.start_drag(part_id, mouse_world);
            } else if game.editor.selected_part_def.is_some() {
                // Try to place a part
                game.editor.place_part(&game.part_definitions);
            }
        } else {
            // Mouse released - finish dragging if active
            if game.editor.is_dragging() {
                game.editor.finish_drag(&game.part_definitions);
            }
        }
    } else if button == MouseButton::Right && state == ElementState::Pressed {
        // Right-click to cancel drag, deselect, or delete
        if game.editor.is_dragging() {
            game.editor.cancel_drag();
        } else if game.editor.selected_part_def.is_some() {
            game.editor.deselect();
        } else if let Some(part_id) = game.editor.selected_placed_part {
            game.editor.delete_part(part_id);
            game.editor.selected_placed_part = None;
        }
    } else if button == MouseButton::Middle {
        // Middle mouse for camera drag
        if state == ElementState::Pressed {
            render_state.camera.is_dragging = true;
        } else {
            render_state.camera.is_dragging = false;
        }
    }
}

/// Handle flight mode cursor movement
fn handle_flight_cursor_moved(
    render_state: &mut RenderState,
    x: f32,
    y: f32,
    egui_consumed: bool,
) {
    if render_state.dragging_maneuver_node.is_some() {
        render_state.update_dragged_node(x, y);
    }

    render_state.camera.last_mouse_pos = [x, y];

    if !egui_consumed {
        render_state.update_hover(x, y);
    }
}

/// Handle editor mode cursor movement
fn handle_editor_cursor_moved(
    game: &mut Game,
    render_state: &mut RenderState,
    x: f32,
    y: f32,
    egui_consumed: bool,
) {
    let screen_width = render_state.size.width as f32;
    let screen_height = render_state.size.height as f32;

    if render_state.camera.is_dragging && !egui_consumed {
        let dx = x - render_state.camera.last_mouse_pos[0];
        let dy = y - render_state.camera.last_mouse_pos[1];
        game.editor.pan_camera(-dx as f64, dy as f64);
    }

    render_state.camera.last_mouse_pos = [x, y];

    if !egui_consumed {
        let [world_x, world_y] = screen_to_world(x, y, screen_width, screen_height, &game.editor);

        // Fairing build mode: update ghost point
        if game.editor.fairing_build_mode.is_some() {
            game.editor.update_fairing_ghost(world_x, world_y);
            return;
        }

        // Update drag position if dragging a part
        if game.editor.is_dragging() {
            game.editor.update_drag(world_x, world_y, &game.part_definitions);
        } else {
            // Update ghost position (only when not dragging)
            game.editor.update_ghost(world_x, world_y, &game.part_definitions);

            // Update hovered part
            game.editor.hovered_part = part_at_screen_pos(
                x, y, screen_width, screen_height,
                &game.editor, &game.part_definitions
            );
        }
    }
}

/// After decoupling, extract decoupled parts into a debris vessel and recenter the active vessel.
fn handle_post_decouple(game: &mut Game) {
    // Read ejection force before extracting (set by activate_next_stage or manual decouple)
    let ejection_force = game.flight.vessel.as_ref()
        .map(|v| v.last_decouple_force).unwrap_or(0.0);

    // Extract fairing halves BEFORE extracting decoupled parts
    let fairing_halves = game.flight.vessel.as_mut()
        .map(|v| v.extract_fairing_halves(&game.part_definitions))
        .unwrap_or_default();

    for (debris_vessel, com_offset, half) in fairing_halves {
        let sign = match half {
            sunscatter::parts::FairingHalf::Left => -1.0,
            sunscatter::parts::FairingHalf::Right => 1.0,
        };
        game.flight.create_fairing_debris(debris_vessel, com_offset, 5.0, sign, &game.solar_system);
    }

    // Extract decoupled parts into debris (separate step to avoid borrow conflict)
    let extracted = game.flight.vessel.as_mut()
        .and_then(|v| {
            let result = v.extract_decoupled_parts(&game.part_definitions);
            v.last_decouple_force = 0.0; // Reset after reading
            result
        });

    if let Some((debris_vessel, com_offset)) = extracted {
        game.flight.create_debris_vessel(debris_vessel, com_offset, ejection_force, &game.solar_system);
    }

    // Recenter parts on new COM and shift ship position to match
    if let Some(ref mut vessel) = game.flight.vessel {
        let com_offset = vessel.recenter_on_com(&game.part_definitions);
        let rot = game.flight.ship.rotation - std::f64::consts::FRAC_PI_2;
        game.flight.ship.rel_position[0] += com_offset[0] * rot.cos() - com_offset[1] * rot.sin();
        game.flight.ship.rel_position[1] += com_offset[0] * rot.sin() + com_offset[1] * rot.cos();
    }
}

/// Handle flight mode keyboard input
fn handle_flight_keyboard(
    game: &mut Game,
    render_state: &mut RenderState,
    logical_key: &Key,
    pressed: bool,
) {
    if let Key::Named(named_key) = logical_key {
        match named_key {
            winit::keyboard::NamedKey::Space => {
                if pressed {
                    if let Some(ref mut vessel) = game.flight.vessel {
                        vessel.activate_next_stage(&game.part_definitions);
                    }
                    handle_post_decouple(game);
                }
            }
            winit::keyboard::NamedKey::Shift => game.flight.ship_input.throttle_up = pressed,
            winit::keyboard::NamedKey::Control => game.flight.ship_input.throttle_down = pressed,
            _ => {}
        }
    }

    if let Key::Character(c) = logical_key {
        match c.as_str() {
            "w" | "W" => game.flight.ship_input.translate_forward = pressed,
            "s" | "S" => game.flight.ship_input.translate_backward = pressed,
            "a" | "A" => game.flight.ship_input.translate_left = pressed,
            "d" | "D" => game.flight.ship_input.translate_right = pressed,
            "q" | "Q" => game.flight.ship_input.rotate_left = pressed,
            "e" | "E" => game.flight.ship_input.rotate_right = pressed,
            "z" | "Z" => game.flight.ship_input.throttle_full = pressed,
            "x" | "X" => game.flight.ship_input.throttle_zero = pressed,
            "r" | "R" => {
                if pressed {
                    render_state.rcs_enabled = !render_state.rcs_enabled;
                }
            }
            "`" => {
                if pressed {
                    game.flight.tracking_ship = true;
                    render_state.tracked_body = None;
                    println!("Focused on: Ship");
                }
            }
            "[" | "]" => {
                if pressed {
                    switch_to_next_vessel(game, render_state, c.as_str() == "]");
                }
            }
            _ => {}
        }
    }
}

/// Switch to the next/previous vessel in the sorted vessel list.
fn switch_to_next_vessel(game: &mut Game, render_state: &mut RenderState, forward: bool) {
    if game.flight.inactive_vessels.is_empty() {
        return;
    }

    let ids = game.flight.all_vessel_ids();
    let current_pos = ids.iter().position(|&id| id == game.flight.active_vessel_id).unwrap_or(0);
    let next_pos = if forward {
        (current_pos + 1) % ids.len()
    } else {
        (current_pos + ids.len() - 1) % ids.len()
    };

    let target_id = ids[next_pos];
    if target_id == game.flight.active_vessel_id {
        return;
    }

    switch_to_next_vessel_by_id(game, render_state, target_id);
}

/// Switch directly to a specific vessel by ID
fn switch_to_next_vessel_by_id(game: &mut Game, render_state: &mut RenderState, target_id: u64) {
    if target_id == game.flight.active_vessel_id {
        return;
    }

    let old_nodes = render_state.swap_maneuver_nodes(Vec::new());
    match game.flight.switch_to_vessel(target_id, old_nodes, &game.solar_system) {
        Ok(new_nodes) => {
            render_state.swap_maneuver_nodes(new_nodes);
            render_state.tracked_body = None;
            game.warp_index = 0;
            log::info!("Switched to vessel: {} (id={})", game.flight.active_vessel_name, target_id);
        }
        Err(e) => log::error!("Failed to switch vessel: {}", e),
    }
}

/// Returns true if rotation is approximately 90° or 270° (dimensions should be swapped)
fn is_part_rotation_swapped(rotation: f64) -> bool {
    let norm = rotation.rem_euclid(std::f64::consts::TAU);
    let quarter = std::f64::consts::FRAC_PI_2;
    (norm - quarter).abs() < 0.01 || (norm - 3.0 * quarter).abs() < 0.01
}

/// Handle editor mode keyboard input
fn handle_editor_keyboard(
    game: &mut Game,
    logical_key: &Key,
    pressed: bool,
    egui_consumed: bool,
) {
    // Arrow keys track held state for smooth camera movement
    if let Key::Named(key) = logical_key {
        match key {
            winit::keyboard::NamedKey::ArrowUp => {
                game.editor.keys_held.up = pressed;
            }
            winit::keyboard::NamedKey::ArrowDown => {
                game.editor.keys_held.down = pressed;
            }
            winit::keyboard::NamedKey::ArrowLeft => {
                game.editor.keys_held.left = pressed;
            }
            winit::keyboard::NamedKey::ArrowRight => {
                game.editor.keys_held.right = pressed;
            }
            winit::keyboard::NamedKey::Delete | winit::keyboard::NamedKey::Backspace if pressed => {
                if let Some(part_id) = game.editor.selected_placed_part {
                    game.editor.delete_part(part_id);
                    game.editor.selected_placed_part = None;
                }
            }
            _ => {}
        }
    }

    // Character keys only on press, and only when egui doesn't have focus
    if pressed && !egui_consumed {
        if let Key::Character(c) = logical_key {
            match c.as_str() {
                "r" | "R" => {
                    // Rotate ghost or placed part by 90° clockwise
                    if game.editor.selected_part_def.is_some() {
                        // Ghost mode: rotate ghost preview
                        game.editor.ghost_rotation = (game.editor.ghost_rotation - std::f64::consts::FRAC_PI_2)
                            .rem_euclid(std::f64::consts::TAU);
                    } else if let Some(part_id) = game.editor.selected_placed_part {
                        // Placed part selected: rotate it in place (with overlap check)
                        if let Some(part) = game.editor.parts.get(&part_id) {
                            let new_rot = (part.rotation - std::f64::consts::FRAC_PI_2)
                                .rem_euclid(std::f64::consts::TAU);
                            let def_id = part.definition_id.clone();
                            let pos = part.position;
                            let mirror_id = part.mirror_partner;
                            if let Some(def) = game.part_definitions.get(&def_id) {
                                // Check if rotated hitbox would overlap
                                let new_bounds = sunscatter::editor::EditorState::calc_bounds_pub(
                                    pos,
                                    def.rotated_hitbox_width(new_rot),
                                    def.rotated_hitbox_height(new_rot),
                                );
                                let mut overlaps = false;
                                for (&other_id, other_part) in &game.editor.parts {
                                    if other_id == part_id || Some(other_id) == mirror_id {
                                        continue;
                                    }
                                    if let Some(other_def) = game.part_definitions.get(&other_part.definition_id) {
                                        let other_bounds = sunscatter::editor::EditorState::calc_bounds_pub(
                                            other_part.position,
                                            other_def.rotated_hitbox_width(other_part.rotation),
                                            other_def.rotated_hitbox_height(other_part.rotation),
                                        );
                                        if sunscatter::editor::EditorState::bounds_overlap_pub(&new_bounds, &other_bounds) {
                                            overlaps = true;
                                            break;
                                        }
                                    }
                                }
                                if !overlaps {
                                    if let Some(part) = game.editor.parts.get_mut(&part_id) {
                                        part.rotation = new_rot;
                                    }
                                    // Also rotate mirror partner (opposite direction)
                                    if let Some(mid) = mirror_id {
                                        if let Some(mirror_part) = game.editor.parts.get_mut(&mid) {
                                            mirror_part.rotation = (-new_rot).rem_euclid(std::f64::consts::TAU);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Handle tracking station mouse input (body double-click focus, camera drag)
fn handle_tracking_station_mouse_input(
    game: &mut Game,
    render_state: &mut RenderState,
    state: ElementState,
    button: MouseButton,
    egui_consumed: bool,
    last_click_time: &mut Option<Instant>,
    last_click_pos: &mut [f32; 2],
) {
    const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(300);
    const DOUBLE_CLICK_DIST: f32 = 10.0;

    if !egui_consumed && button == MouseButton::Left {
        if state == ElementState::Pressed {
            let now = Instant::now();
            let mouse_pos = render_state.camera.last_mouse_pos;
            let dx = mouse_pos[0] - last_click_pos[0];
            let dy = mouse_pos[1] - last_click_pos[1];
            let dist = (dx * dx + dy * dy).sqrt();

            // Double-click on body to focus
            let mut was_double_click = false;
            if let Some(last_time) = *last_click_time {
                if now.duration_since(last_time) < DOUBLE_CLICK_TIME && dist < DOUBLE_CLICK_DIST {
                    if let Some(body_idx) = render_state.body_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                        render_state.focus_on_body(body_idx);
                        println!("Focused on: {}", game.solar_system.bodies[body_idx].name);
                        was_double_click = true;
                    }
                    *last_click_time = None;
                } else {
                    *last_click_time = Some(now);
                    *last_click_pos = mouse_pos;
                }
            } else {
                *last_click_time = Some(now);
                *last_click_pos = mouse_pos;
            }

            // Only start drag if this wasn't a double-click focus
            if !was_double_click {
                render_state.camera.is_dragging = true;
            }
        } else {
            render_state.camera.is_dragging = false;
        }
    }
}

/// Handle tracking station cursor movement (camera drag, hover)
fn handle_tracking_station_cursor_moved(
    render_state: &mut RenderState,
    x: f32,
    y: f32,
    egui_consumed: bool,
) {
    if render_state.camera.is_dragging && !egui_consumed {
        let dx = x - render_state.camera.last_mouse_pos[0];
        let dy = y - render_state.camera.last_mouse_pos[1];
        let scale = 2.0 / render_state.size.height as f32;
        render_state.camera.pan(dx * scale, dy * scale);
        // Panning breaks body and vessel tracking
        render_state.tracked_body = None;
        render_state.tracked_vessel = None;
    }

    render_state.camera.last_mouse_pos = [x, y];

    if !egui_consumed {
        render_state.update_hover(x, y);
    }
}

/// Oriented Bounding Box overlap test using the Separating Axis Theorem.
fn obb_overlap(
    pos_a: [f64; 2], rot_a: f64, hw_a: f64, hh_a: f64,
    pos_b: [f64; 2], rot_b: f64, hw_b: f64, hh_b: f64,
) -> bool {
    let (sin_a, cos_a) = rot_a.sin_cos();
    let (sin_b, cos_b) = rot_b.sin_cos();

    // Two axes per OBB: local right and local up
    let axes = [
        [cos_a, sin_a],
        [-sin_a, cos_a],
        [cos_b, sin_b],
        [-sin_b, cos_b],
    ];
    let halves_a = [hw_a, hh_a];
    let halves_b = [hw_b, hh_b];

    let d = [pos_b[0] - pos_a[0], pos_b[1] - pos_a[1]];

    for axis in &axes {
        let dist_proj = (d[0] * axis[0] + d[1] * axis[1]).abs();

        let a_proj = (axes[0][0] * axis[0] + axes[0][1] * axis[1]).abs() * halves_a[0]
            + (axes[1][0] * axis[0] + axes[1][1] * axis[1]).abs() * halves_a[1];

        let b_proj = (axes[2][0] * axis[0] + axes[2][1] * axis[1]).abs() * halves_b[0]
            + (axes[3][0] * axis[0] + axes[3][1] * axis[1]).abs() * halves_b[1];

        if dist_proj > a_proj + b_proj {
            return false;
        }
    }
    true
}
