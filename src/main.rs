use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::{
    event::{ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    window::WindowBuilder,
};

use space_game::bodies::SolarSystem;
use space_game::render::{RenderState, OrbitRenderData, ShipRenderData, ShipOrbitData, OrbitSegmentData};
use space_game::ship::{AutopilotTarget, Ship, ShipInput, SHIP_SIZE, MAX_THRUST_ACCELERATION};

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
    let mut solar_system = SolarSystem::new();
    let mut ship = Ship::spawn_on_earth(&solar_system);
    let mut ship_input = ShipInput::default();
    let mut last_frame = Instant::now();

    // Double-click detection
    let mut last_click_time: Option<Instant> = None;
    let mut last_click_pos: [f32; 2] = [0.0, 0.0];
    const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(300);
    const DOUBLE_CLICK_DIST: f32 = 10.0; // pixels

    // Time warp state
    let mut warp_index: usize = 0;

    // Ship tracking state (true = follow ship, false = follow body or free camera)
    let mut tracking_ship: bool = true;

    // Initial camera zoom to see ship on Earth's surface
    // Ship is 10m, in world units: 10 * SCALE * BODY_SCALE = 10 * 1e-9 * 4 = 4e-8
    // To see this on screen we need high zoom
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
                            let dt = now.duration_since(last_frame).as_secs_f64();
                            last_frame = now;

                            // Update simulation with current time warp
                            let time_warp = WARP_LEVELS[warp_index];
                            solar_system.update(dt * time_warp);

                            // Update ship physics
                            ship.update(dt * time_warp, time_warp, &ship_input, &solar_system);

                            // Autopilot rotation control (disabled during on-rails warp)
                            let autopilot_target = render_state.get_autopilot_target();
                            if autopilot_target != AutopilotTarget::Off && !ship.on_rails {
                                let maneuver_node = render_state.get_selected_maneuver_node();
                                if let Some(target_angle) = ship.autopilot_target_angle(autopilot_target, maneuver_node) {
                                    ship.autopilot_rotate(target_angle, dt);
                                }
                            }

                            // Apply burns to maneuver node delta-v
                            // When ship is thrusting, reduce the selected maneuver node's delta-v
                            if ship.throttle > 0.0 && render_state.get_selected_maneuver_node().is_some() {
                                let time_warp = WARP_LEVELS[warp_index];
                                let delta_v_this_frame = ship.throttle * MAX_THRUST_ACCELERATION * dt * time_warp;
                                let burn_direction = [ship.rotation.cos(), ship.rotation.sin()];
                                render_state.apply_burn_to_maneuver(burn_direction, delta_v_this_frame);
                            }

                            // Collect body data for rendering
                            let mut scaled_positions: Vec<[f64; 2]> =
                                Vec::with_capacity(solar_system.bodies.len());

                            for i in 0..solar_system.bodies.len() {
                                let pos = solar_system.body_position(i);
                                let body = &solar_system.bodies[i];

                                let scaled_pos = if let Some(parent_idx) = body.parent {
                                    let parent_scaled = scaled_positions[parent_idx];
                                    let parent_unscaled = solar_system.body_position(parent_idx);
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
                            if tracking_ship {
                                // Follow ship position (apply same scaling as bodies)
                                let ship_abs_pos = ship.absolute_position(&solar_system);
                                render_state.camera.position[0] = ship_abs_pos[0] * SCALE * BODY_SCALE;
                                render_state.camera.position[1] = ship_abs_pos[1] * SCALE * BODY_SCALE;
                            } else {
                                render_state.update_tracking(&scaled_positions, SCALE);
                            }

                            let bodies: Vec<_> = (0..solar_system.bodies.len())
                                .map(|i| {
                                    let body = &solar_system.bodies[i];
                                    let pos = scaled_positions[i];
                                    (pos[0], pos[1], body.radius * BODY_SCALE, body.color)
                                })
                                .collect();

                            let pixels_per_world_unit =
                                render_state.camera.zoom * render_state.size.height as f32 / 2.0;

                            let orbits: Vec<Option<OrbitRenderData>> = (0..solar_system.bodies.len())
                                .map(|i| {
                                    let body = &solar_system.bodies[i];
                                    match (body.parent, &body.orbit) {
                                        (Some(parent_idx), Some(orbit)) => {
                                            let body_world_radius =
                                                (body.radius * BODY_SCALE * SCALE) as f32;
                                            let body_pixels =
                                                body_world_radius * pixels_per_world_unit * 2.0;
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
                                                semi_major_axis: orbit.semi_major_axis
                                                    * SCALE
                                                    * BODY_SCALE,
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
                            // Ship position needs the same scaling as bodies
                            let ship_abs_pos = ship.absolute_position(&solar_system);
                            // Update cached orbit (needed when not on rails)
                            let _ = ship.calculate_orbit(&solar_system);
                            let ship_orbit = ship.get_orbital_info(&solar_system).map(|info| {
                                let parent_pos = scaled_positions[info.parent_idx];
                                let parent_body = &solar_system.bodies[info.parent_idx];
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

                            // Calculate velocity and altitude
                            let velocity = (ship.rel_velocity[0].powi(2) + ship.rel_velocity[1].powi(2)).sqrt();
                            let distance_from_soi = (ship.rel_position[0].powi(2) + ship.rel_position[1].powi(2)).sqrt();
                            let soi_body = &solar_system.bodies[ship.soi_body];
                            let altitude = distance_from_soi - soi_body.radius;

                            // Calculate patched conics trajectory
                            let patched_traj_raw = ship.get_patched_trajectory(&solar_system);

                            // Get time to intercept from first segment's end_time
                            let time_to_intercept = patched_traj_raw.as_ref()
                                .and_then(|traj| traj.segments.first())
                                .and_then(|seg| seg.end_time);

                            let patched_trajectory = patched_traj_raw
                                .map(|traj| {
                                    traj.segments.iter().enumerate().map(|(i, seg)| {
                                        let parent_pos = scaled_positions[seg.parent_idx];
                                        let parent_soi = solar_system.bodies[seg.parent_idx].soi_radius;
                                        let parent_mass = solar_system.bodies[seg.parent_idx].mass;
                                        // Dim color for future segments
                                        let alpha = if i == 0 { 0.7 } else { 0.4 };
                                        OrbitSegmentData {
                                            parent_x: parent_pos[0] * SCALE,
                                            parent_y: parent_pos[1] * SCALE,
                                            semi_major_axis: seg.orbit.semi_major_axis * SCALE * BODY_SCALE,
                                            eccentricity: seg.orbit.eccentricity,
                                            argument_of_periapsis: seg.orbit.argument_of_periapsis,
                                            start_true_anomaly: seg.start_true_anomaly,
                                            end_true_anomaly: seg.end_true_anomaly,
                                            color: [ship.color[0] * 0.6, ship.color[1] * 0.6, ship.color[2] * 0.6, alpha],
                                            is_first_segment: i == 0,
                                            retrograde: seg.retrograde,
                                            soi_radius: parent_soi * SCALE * BODY_SCALE,
                                            parent_body_radius: solar_system.bodies[seg.parent_idx].radius,
                                            parent_mass,
                                            parent_idx: seg.parent_idx,
                                            render_scale: SCALE * BODY_SCALE,
                                        }
                                    }).collect::<Vec<_>>()
                                })
                                .unwrap_or_default();

                            // Get current true anomaly from first trajectory segment
                            let current_true_anomaly = patched_trajectory.first()
                                .map(|seg| seg.start_true_anomaly)
                                .unwrap_or(0.0);

                            let ship_render = ShipRenderData {
                                x: ship_abs_pos[0] * SCALE * BODY_SCALE,
                                y: ship_abs_pos[1] * SCALE * BODY_SCALE,
                                rotation: ship.rotation,
                                size: SHIP_SIZE * SCALE * BODY_SCALE,
                                color: ship.color,
                                orbit: ship_orbit,
                                patched_trajectory,
                                velocity,
                                altitude,
                                soi_body_name: soi_body.name.clone(),
                                throttle: ship.throttle,
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

                                // Node calculates position and velocity from stored orbit
                                let scale = node.render_scale;
                                let parent_idx = node.parent_idx;

                                // Get current parent position
                                let current_parent_pos = scaled_positions[parent_idx];
                                let current_parent_x = current_parent_pos[0] * SCALE;
                                let current_parent_y = current_parent_pos[1] * SCALE;

                                let world_pos = node.world_pos(current_parent_x, current_parent_y);
                                let velocity = node.velocity();

                                // Convert world position back to relative position (unscaled)
                                let rel_x = (world_pos[0] - current_parent_x) / scale;
                                let rel_y = (world_pos[1] - current_parent_y) / scale;
                                let pos = [rel_x, rel_y];

                                // Get velocity from node and apply delta-v
                                let prograde = node.prograde_unit();
                                let radial = node.radial_unit();

                                let new_vel = [
                                    velocity[0] + node.delta_v.prograde * prograde[0] + node.delta_v.radial_out * radial[0],
                                    velocity[1] + node.delta_v.prograde * prograde[1] + node.delta_v.radial_out * radial[1],
                                ];

                                // Calculate predicted trajectory using ship's method
                                if let Some(pred_traj) = ship.calculate_predicted_trajectory(
                                    pos, new_vel, parent_idx, &solar_system
                                ) {
                                    let segments: Vec<OrbitSegmentData> = pred_traj.segments.iter().enumerate().map(|(i, seg)| {
                                        let parent_pos = scaled_positions[seg.parent_idx];
                                        let parent_soi = solar_system.bodies[seg.parent_idx].soi_radius;
                                        let parent_mass = solar_system.bodies[seg.parent_idx].mass;
                                        let alpha = if i == 0 { 0.7 } else { 0.5 };
                                        OrbitSegmentData {
                                            parent_x: parent_pos[0] * SCALE,
                                            parent_y: parent_pos[1] * SCALE,
                                            semi_major_axis: seg.orbit.semi_major_axis * SCALE * BODY_SCALE,
                                            eccentricity: seg.orbit.eccentricity,
                                            argument_of_periapsis: seg.orbit.argument_of_periapsis,
                                            start_true_anomaly: seg.start_true_anomaly,
                                            end_true_anomaly: seg.end_true_anomaly,
                                            color: [0.2, 0.8, 0.2, alpha],  // Green
                                            is_first_segment: i == 0,
                                            retrograde: seg.retrograde,
                                            soi_radius: parent_soi * SCALE * BODY_SCALE,
                                            parent_body_radius: solar_system.bodies[seg.parent_idx].radius,
                                            parent_mass,
                                            parent_idx: seg.parent_idx,
                                            render_scale: SCALE * BODY_SCALE,
                                        }
                                    }).collect();
                                    predicted_trajectories.push(segments);
                                }
                            }
                            render_state.set_predicted_trajectories(predicted_trajectories);

                            let body_names: Vec<String> =
                                solar_system.bodies.iter().map(|b| b.name.clone()).collect();

                            match render_state.render(&body_names, WARP_LEVELS, warp_index) {
                                Ok(new_warp_index) => {
                                    warp_index = new_warp_index;
                                }
                                Err(wgpu::SurfaceError::Lost) => {
                                    render_state.resize(render_state.size)
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                                Err(e) => eprintln!("Render error: {:?}", e),
                            }
                        }

                        WindowEvent::MouseInput { state, button, .. } => {
                            if !egui_consumed && *button == MouseButton::Left {
                                if *state == ElementState::Pressed {
                                    let now = Instant::now();
                                    let mouse_pos = render_state.camera.last_mouse_pos;
                                    let dx = mouse_pos[0] - last_click_pos[0];
                                    let dy = mouse_pos[1] - last_click_pos[1];
                                    let dist = (dx * dx + dy * dy).sqrt();

                                    // Check if clicking on a maneuver node - start dragging
                                    if let Some(node_id) = render_state.maneuver_node_at_screen_pos(mouse_pos[0], mouse_pos[1]) {
                                        render_state.start_dragging_node(node_id);
                                        render_state.selected_maneuver_node = Some(node_id);
                                        render_state.pending_orbit_click = None;
                                        // Don't set camera dragging when dragging a node
                                    } else {
                                        render_state.camera.is_dragging = true;
                                    }

                                    // Check for double-click on body
                                    if let Some(last_time) = last_click_time {
                                        if now.duration_since(last_time) < DOUBLE_CLICK_TIME
                                            && dist < DOUBLE_CLICK_DIST
                                        {
                                            if let Some(body_idx) = render_state
                                                .body_at_screen_pos(mouse_pos[0], mouse_pos[1])
                                            {
                                                render_state.focus_on_body(body_idx);
                                                tracking_ship = false;
                                                println!(
                                                    "Focused on: {}",
                                                    solar_system.bodies[body_idx].name
                                                );
                                            }
                                            last_click_time = None;
                                        } else if render_state.dragging_maneuver_node.is_none() {
                                            // Single click - check for orbit click (not when dragging node)
                                            if let Some(orbit_pos) = render_state.orbit_click_position(mouse_pos[0], mouse_pos[1]) {
                                                // Clicked on orbit - show create node button
                                                render_state.pending_orbit_click = Some(orbit_pos);
                                                render_state.selected_maneuver_node = None;
                                            } else {
                                                // Clicked elsewhere - clear pending state
                                                render_state.pending_orbit_click = None;
                                            }
                                            last_click_time = Some(now);
                                            last_click_pos = mouse_pos;
                                        }
                                    } else if render_state.dragging_maneuver_node.is_none() {
                                        // First click - check for orbit click (not when already dragging a node)
                                        if let Some(orbit_pos) = render_state.orbit_click_position(mouse_pos[0], mouse_pos[1]) {
                                            render_state.pending_orbit_click = Some(orbit_pos);
                                            render_state.selected_maneuver_node = None;
                                        } else {
                                            render_state.pending_orbit_click = None;
                                        }
                                        last_click_time = Some(now);
                                        last_click_pos = mouse_pos;
                                    }
                                } else {
                                    // Mouse released
                                    render_state.camera.is_dragging = false;
                                    render_state.stop_dragging_node();
                                }
                            }
                        }

                        WindowEvent::CursorMoved { position, .. } => {
                            let x = position.x as f32;
                            let y = position.y as f32;

                            // Update dragged maneuver node position
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

                        WindowEvent::MouseWheel { delta, .. } => {
                            if !egui_consumed {
                                let scroll_amount = match delta {
                                    MouseScrollDelta::LineDelta(_, y) => *y,
                                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                                };
                                let zoom_factor = 1.0 + scroll_amount * 0.1;
                                render_state.camera.zoom_by(zoom_factor);
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
                            if let Key::Character(c) = logical_key {
                                match c.as_str() {
                                    "w" | "W" => ship_input.throttle_up = pressed,
                                    "s" | "S" => ship_input.throttle_down = pressed,
                                    "z" | "Z" => ship_input.throttle_full = pressed,
                                    "x" | "X" => ship_input.throttle_zero = pressed,
                                    "a" | "A" => ship_input.rotate_left = pressed,
                                    "d" | "D" => ship_input.rotate_right = pressed,
                                    "`" => {
                                        if pressed {
                                            tracking_ship = true;
                                            render_state.tracked_body = None;
                                            println!("Focused on: Ship");
                                        }
                                    }
                                    _ => {}
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
