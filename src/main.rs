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
use space_game::ship::{Ship, ShipInput, SHIP_SIZE};

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
                            let patched_traj_raw = ship.calculate_patched_trajectory(&solar_system);

                            // Get time to intercept from first segment's end_time
                            let time_to_intercept = patched_traj_raw.as_ref()
                                .and_then(|traj| traj.segments.first())
                                .and_then(|seg| seg.end_time);

                            // Auto-reduce time warp if approaching SOI transition too fast
                            // Only applies when warp > 1000x and would reach SOI boundary in < 0.5 seconds
                            if let Some(intercept_time) = time_to_intercept {
                                let current_warp = WARP_LEVELS[warp_index];
                                if current_warp > 1000.0 && intercept_time / current_warp < 0.5 {
                                    // Find the highest warp level that won't reach SOI boundary in < 0.5 seconds
                                    while warp_index > 0 {
                                        warp_index -= 1;
                                        let lower_warp = WARP_LEVELS[warp_index];
                                        // Stop if this warp level is safe (>= 0.5 seconds to boundary)
                                        if intercept_time / lower_warp >= 0.5 {
                                            break;
                                        }
                                    }
                                }
                            }

                            let patched_trajectory = patched_traj_raw
                                .map(|traj| {
                                    traj.segments.iter().enumerate().map(|(i, seg)| {
                                        let parent_pos = scaled_positions[seg.parent_idx];
                                        let parent_soi = solar_system.bodies[seg.parent_idx].soi_radius;
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
                                            render_scale: SCALE * BODY_SCALE,
                                        }
                                    }).collect::<Vec<_>>()
                                })
                                .unwrap_or_default();

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
                            };

                            render_state.update_bodies_orbits_and_ship(&bodies, &orbits, Some(&ship_render), SCALE);

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
                                    render_state.camera.is_dragging = true;

                                    let now = Instant::now();
                                    let mouse_pos = render_state.camera.last_mouse_pos;
                                    let dx = mouse_pos[0] - last_click_pos[0];
                                    let dy = mouse_pos[1] - last_click_pos[1];
                                    let dist = (dx * dx + dy * dy).sqrt();

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
                                        } else {
                                            last_click_time = Some(now);
                                            last_click_pos = mouse_pos;
                                        }
                                    } else {
                                        last_click_time = Some(now);
                                        last_click_pos = mouse_pos;
                                    }
                                } else {
                                    render_state.camera.is_dragging = false;
                                }
                            }
                        }

                        WindowEvent::CursorMoved { position, .. } => {
                            let x = position.x as f32;
                            let y = position.y as f32;

                            if render_state.camera.is_dragging && !egui_consumed {
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
                            match logical_key {
                                Key::Character(c) => {
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
                                _ => {}
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
