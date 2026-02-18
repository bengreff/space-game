use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    event::{ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    window::WindowBuilder,
};

use space_game::editor::{
    render_editor_ui, EditorAction, generate_grid_vertices, generate_part_vertices,
    generate_ghost_vertices, screen_to_world, part_at_screen_pos, BodyInfo,
};
use space_game::game::{Game, GameMode};
use space_game::render::{RenderState, OrbitRenderData, ShipRenderData, ShipOrbitData, OrbitSegmentData, Vertex};
use space_game::ship::{AutopilotTarget, SHIP_SIZE, MAX_THRUST_ACCELERATION};

// 1:1 Real-Scale Solar System Simulation
// All physics use real-world values: masses, radii, distances, orbital velocities
// Rendering scale: 1 world unit = 1 billion meters (1e9 m)
const SCALE: f64 = 1e-9;

// Time warp levels (simulation seconds per real second)
const WARP_LEVELS: &[f64] = &[1.0, 2.0, 3.0, 5.0, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0, 100000000.0, 1000000000.0];

// Visual scale factor for bodies (1.0 = real proportions, no artificial enlargement)
const BODY_SCALE: f64 = 1.0;

fn main() {
    env_logger::init();

    println!("Space Game starting...");
    println!("Controls:");
    println!("  W: Increase throttle");
    println!("  S: Decrease throttle");
    println!("  Z: Full throttle (100%)");
    println!("  X: Cut throttle (0%)");
    println!("  A: Rotate left");
    println!("  D: Rotate right");
    println!("  E: Open Editor");
    println!("  ` (backtick): Focus on ship");
    println!("  Left mouse drag: Pan camera");
    println!("  Scroll wheel: Zoom in/out");
    println!("  Double-click planet: Focus on it");
    println!("  Time warp: Click buttons at top of screen");
    println!("  Close window to exit");
    println!();
    println!("Physics: 1:1 real-scale solar system");
    println!("Earth LEO velocity: ~7.8 km/s (real value)");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Space Game - Solar System")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)
            .unwrap(),
    );

    let mut render_state = pollster::block_on(RenderState::new(window.clone()));
    let mut game = Game::new();
    let mut last_frame = Instant::now();

    // Double-click detection
    let mut last_click_time: Option<Instant> = None;
    let mut last_click_pos: [f32; 2] = [0.0, 0.0];

    // Initial camera zoom to see ship on Earth's surface
    render_state.camera.zoom = 5e6;

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

                            match game.mode {
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
                            }
                        }

                        WindowEvent::MouseInput { state, button, .. } => {
                            match game.mode {
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
                            }
                        }

                        WindowEvent::CursorMoved { position, .. } => {
                            let x = position.x as f32;
                            let y = position.y as f32;

                            match game.mode {
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
                                    GameMode::Flight => {
                                        render_state.camera.zoom_by(zoom_factor);
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

                            match game.mode {
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

    // Update simulation with current time warp
    let time_warp = WARP_LEVELS[game.flight.warp_index];
    game.solar_system.update(dt * time_warp);

    // Update ship physics
    game.flight.ship.update(dt * time_warp, time_warp, &game.flight.ship_input, &game.solar_system);

    // Autopilot rotation control (disabled during on-rails warp)
    let autopilot_target = render_state.get_autopilot_target();
    if autopilot_target != AutopilotTarget::Off && !game.flight.ship.on_rails {
        let maneuver_node = render_state.get_selected_maneuver_node();
        if let Some(target_angle) = game.flight.ship.autopilot_target_angle(autopilot_target, maneuver_node) {
            game.flight.ship.autopilot_rotate(target_angle, dt);
        }
    }

    // Apply burns to maneuver node delta-v
    if game.flight.ship.throttle > 0.0 && render_state.get_selected_maneuver_node().is_some() {
        let delta_v_this_frame = game.flight.ship.throttle * MAX_THRUST_ACCELERATION * dt * time_warp;
        let burn_direction = [game.flight.ship.rotation.cos(), game.flight.ship.rotation.sin()];
        render_state.apply_burn_to_maneuver(burn_direction, delta_v_this_frame);
    }

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
        let ship_abs_pos = game.flight.ship.absolute_position(&game.solar_system);
        render_state.camera.position[0] = ship_abs_pos[0] * SCALE * BODY_SCALE;
        render_state.camera.position[1] = ship_abs_pos[1] * SCALE * BODY_SCALE;
    } else {
        render_state.update_tracking(&scaled_positions, SCALE);
    }

    let bodies: Vec<_> = (0..game.solar_system.bodies.len())
        .map(|i| {
            let body = &game.solar_system.bodies[i];
            let pos = scaled_positions[i];
            (pos[0], pos[1], body.radius * BODY_SCALE, body.color)
        })
        .collect();

    let pixels_per_world_unit = render_state.camera.zoom * render_state.size.height as f32 / 2.0;

    let orbits: Vec<Option<OrbitRenderData>> = (0..game.solar_system.bodies.len())
        .map(|i| {
            let body = &game.solar_system.bodies[i];
            match (body.parent, &body.orbit) {
                (Some(parent_idx), Some(orbit)) => {
                    let body_world_radius = (body.radius * BODY_SCALE * SCALE) as f32;
                    let body_pixels = body_world_radius * pixels_per_world_unit * 2.0;
                    let is_moon = parent_idx != 0;
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
    let ship_abs_pos = game.flight.ship.absolute_position(&game.solar_system);
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

    let patched_trajectory = patched_traj_raw
        .map(|traj| {
            traj.segments.iter().enumerate().map(|(i, seg)| {
                let parent_pos = scaled_positions[seg.parent_idx];
                let parent_soi = game.solar_system.bodies[seg.parent_idx].soi_radius;
                let parent_mass = game.solar_system.bodies[seg.parent_idx].mass;
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
                }
            }).collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let current_true_anomaly = patched_trajectory.first()
        .map(|seg| seg.start_true_anomaly)
        .unwrap_or(0.0);

    let ship_render = ShipRenderData {
        x: ship_abs_pos[0] * SCALE * BODY_SCALE,
        y: ship_abs_pos[1] * SCALE * BODY_SCALE,
        rotation: game.flight.ship.rotation,
        size: SHIP_SIZE * SCALE * BODY_SCALE,
        color: game.flight.ship.color,
        orbit: ship_orbit,
        patched_trajectory,
        velocity,
        altitude,
        soi_body_name: soi_body.name.clone(),
        throttle: game.flight.ship.throttle,
        time_to_intercept,
        acceleration: MAX_THRUST_ACCELERATION,
        current_true_anomaly,
    };

    render_state.update_bodies_orbits_and_ship(&bodies, &orbits, Some(&ship_render), SCALE);

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
            pos, new_vel, parent_idx, &game.solar_system
        ) {
            let segments: Vec<OrbitSegmentData> = pred_traj.segments.iter().enumerate().map(|(i, seg)| {
                let parent_pos = scaled_positions[seg.parent_idx];
                let parent_soi = game.solar_system.bodies[seg.parent_idx].soi_radius;
                let parent_mass = game.solar_system.bodies[seg.parent_idx].mass;
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
                }
            }).collect();
            predicted_trajectories.push(segments);
        }
    }
    render_state.set_predicted_trajectories(predicted_trajectories);

    let body_names: Vec<String> = game.solar_system.bodies.iter().map(|b| b.name.clone()).collect();

    match render_state.render(&body_names, WARP_LEVELS, game.flight.warp_index) {
        Ok(new_warp_index) => {
            game.flight.warp_index = new_warp_index;
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
    vertices.extend(generate_part_vertices(&game.editor, &game.part_definitions));

    // Ghost preview
    vertices.extend(generate_ghost_vertices(&game.editor, &game.part_definitions));

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

    let result = render_state.render_editor(&vertices, |ctx| {
        action = render_editor_ui(
            ctx,
            &mut game.editor,
            &part_defs,
            &blueprint_names,
            &stats,
            &bodies,
        );
    });

    // Handle editor actions
    match action {
        EditorAction::Launch => {
            match game.launch_from_editor() {
                Ok(()) => log::info!("Launched vessel"),
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

    if !egui_consumed && button == MouseButton::Left {
        if state == ElementState::Pressed {
            let now = Instant::now();
            let mouse_pos = render_state.camera.last_mouse_pos;
            let dx = mouse_pos[0] - last_click_pos[0];
            let dy = mouse_pos[1] - last_click_pos[1];
            let dist = (dx * dx + dy * dy).sqrt();

            // Check if clicking on a maneuver node
            if let Some(node_id) = render_state.maneuver_node_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                render_state.start_dragging_node(node_id);
                render_state.selected_maneuver_node = Some(node_id);
                render_state.pending_orbit_click = None;
            } else {
                render_state.camera.is_dragging = true;
            }

            // Check for double-click on body
            if let Some(last_time) = *last_click_time {
                if now.duration_since(last_time) < DOUBLE_CLICK_TIME && dist < DOUBLE_CLICK_DIST {
                    if let Some(body_idx) = render_state.body_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                        render_state.focus_on_body(body_idx);
                        game.flight.tracking_ship = false;
                        println!("Focused on: {}", game.solar_system.bodies[body_idx].name);
                    }
                    *last_click_time = None;
                } else if render_state.dragging_maneuver_node.is_none() {
                    if let Some(orbit_pos) = render_state.orbit_click_position(mouse_pos[0], mouse_pos[1]) {
                        render_state.pending_orbit_click = Some(orbit_pos);
                        render_state.selected_maneuver_node = None;
                    } else {
                        render_state.pending_orbit_click = None;
                    }
                    *last_click_time = Some(now);
                    *last_click_pos = mouse_pos;
                }
            } else if render_state.dragging_maneuver_node.is_none() {
                if let Some(orbit_pos) = render_state.orbit_click_position(mouse_pos[0], mouse_pos[1]) {
                    render_state.pending_orbit_click = Some(orbit_pos);
                    render_state.selected_maneuver_node = None;
                } else {
                    render_state.pending_orbit_click = None;
                }
                *last_click_time = Some(now);
                *last_click_pos = mouse_pos;
            }
        } else {
            render_state.camera.is_dragging = false;
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

    if button == MouseButton::Left {
        if state == ElementState::Pressed {
            // Check if clicking on a placed part - start dragging it
            if let Some(part_id) = part_at_screen_pos(
                mouse_pos[0], mouse_pos[1],
                screen_width, screen_height,
                &game.editor, &game.part_definitions
            ) {
                game.editor.start_drag(part_id);
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
    } else if render_state.camera.is_dragging && !egui_consumed {
        let dx = x - render_state.camera.last_mouse_pos[0];
        let dy = y - render_state.camera.last_mouse_pos[1];
        let scale = 2.0 / render_state.size.height as f32;
        render_state.camera.pan(dx * scale, dy * scale);
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

/// Handle flight mode keyboard input
fn handle_flight_keyboard(
    game: &mut Game,
    render_state: &mut RenderState,
    logical_key: &Key,
    pressed: bool,
) {
    if let Key::Character(c) = logical_key {
        match c.as_str() {
            "w" | "W" => game.flight.ship_input.throttle_up = pressed,
            "s" | "S" => game.flight.ship_input.throttle_down = pressed,
            "z" | "Z" => game.flight.ship_input.throttle_full = pressed,
            "x" | "X" => game.flight.ship_input.throttle_zero = pressed,
            "a" | "A" => game.flight.ship_input.rotate_left = pressed,
            "d" | "D" => game.flight.ship_input.rotate_right = pressed,
            "e" | "E" => {
                if pressed {
                    game.enter_editor();
                }
            }
            "`" => {
                if pressed {
                    game.flight.tracking_ship = true;
                    render_state.tracked_body = None;
                    println!("Focused on: Ship");
                }
            }
            _ => {}
        }
    }
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
            // Other named keys only on press
            winit::keyboard::NamedKey::Escape if pressed => {
                if game.editor.selected_part_def.is_some() {
                    game.editor.deselect();
                } else if game.editor.selected_placed_part.is_some() {
                    game.editor.selected_placed_part = None;
                }
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
                    game.editor.symmetry_mode = game.editor.symmetry_mode.cycle_next();
                }
                _ => {}
            }
        }
    }
}
