use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;
use egui_wgpu::ScreenDescriptor;

use super::camera::Camera;
use super::types::{
    BodyData, OrbitRenderData, ShipOrbitData, ShipRenderData, Vertex,
    HYPERBOLIC_RENDER_MARGIN, HYPERBOLIC_SKIP_MARGIN,
};

/// Main render state holding all wgpu resources
pub struct RenderState {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera: Camera,
    pub window: Arc<Window>,
    pub msaa_texture: wgpu::Texture,
    pub msaa_view: wgpu::TextureView,
    pub bodies: Vec<BodyData>,
    pub hovered_body: Option<usize>,
    pub tracked_body: Option<usize>, // Body the camera is following
    pub ship_orbit_info: Option<ShipOrbitData>, // Stored for UI display
    pub ship_velocity: f64,    // Current velocity (m/s)
    pub ship_altitude: f64,    // Current altitude (m)
    pub ship_throttle: f64,    // Current throttle (0.0 to 1.0)
    pub ship_soi_name: String, // Current SOI body name
    pub ship_time_to_intercept: Option<f64>, // Time to next SOI transition (seconds)
    pub ap_markers: Vec<([f64; 2], f64)>, // Apoapsis markers: (world pos relative to camera, altitude)
    pub pe_markers: Vec<([f64; 2], f64)>, // Periapsis markers: (world pos relative to camera, altitude)
    // Egui state
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
}

impl RenderState {
    /// Create a new render state from a window
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window.clone()).unwrap();

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create camera
        let aspect_ratio = size.width as f32 / size.height as f32;
        let camera = Camera::new(aspect_ratio);
        let camera_uniform = camera.to_uniform();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create render pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 4, // 4x MSAA
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create MSAA texture
        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create large buffers for dynamic geometry (multiple bodies)
        // When zoomed in, one body can have up to 4096 segments
        // 20 bodies * ~4100 vertices each = ~82000, plus safety margin
        let max_vertices = 500_000;
        let max_indices = 1_500_000;

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (max_indices * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let num_indices = 0;

        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            camera_buffer,
            camera_bind_group,
            camera,
            window,
            msaa_texture,
            msaa_view,
            bodies: Vec::new(),
            hovered_body: None,
            tracked_body: None,
            ship_orbit_info: None,
            ship_velocity: 0.0,
            ship_altitude: 0.0,
            ship_throttle: 0.0,
            ship_soi_name: String::new(),
            ship_time_to_intercept: None,
            ap_markers: Vec::new(),
            pe_markers: Vec::new(),
            egui_ctx,
            egui_state,
            egui_renderer,
        }
    }

    /// Create vertices and indices for a ship triangle
    /// The triangle points in the direction of `rotation` (0 = right, PI/2 = up)
    pub fn create_ship_triangle(
        cx: f32,
        cy: f32,
        size: f32,
        rotation: f32,
        color: [f32; 4],
    ) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::with_capacity(3);
        let mut indices = Vec::with_capacity(3);

        // Triangle pointing in direction of rotation
        // Nose (front), and two back corners
        let nose_angle = rotation;
        let back_left_angle = rotation + std::f32::consts::PI * 0.8;
        let back_right_angle = rotation - std::f32::consts::PI * 0.8;

        // Nose vertex (front)
        vertices.push(Vertex {
            position: [
                cx + size * nose_angle.cos(),
                cy + size * nose_angle.sin(),
            ],
            color,
        });

        // Back left vertex
        vertices.push(Vertex {
            position: [
                cx + size * 0.6 * back_left_angle.cos(),
                cy + size * 0.6 * back_left_angle.sin(),
            ],
            color,
        });

        // Back right vertex
        vertices.push(Vertex {
            position: [
                cx + size * 0.6 * back_right_angle.cos(),
                cy + size * 0.6 * back_right_angle.sin(),
            ],
            color,
        });

        // Single triangle
        indices.push(0);
        indices.push(1);
        indices.push(2);

        (vertices, indices)
    }

    /// Create vertices and indices for a filled circle
    pub fn create_circle(
        cx: f32,
        cy: f32,
        radius: f32,
        segments: u32,
        color: [f32; 4],
    ) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::with_capacity(segments as usize + 1);
        let mut indices = Vec::with_capacity(segments as usize * 3);

        // Center vertex
        vertices.push(Vertex {
            position: [cx, cy],
            color,
        });

        // Edge vertices
        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin();
            vertices.push(Vertex {
                position: [x, y],
                color,
            });
        }

        // Triangle fan indices
        for i in 0..segments {
            indices.push(0); // Center
            indices.push(i + 1);
            indices.push((i + 1) % segments + 1);
        }

        (vertices, indices)
    }

    /// Handle window resize
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Recreate MSAA texture with new size
            self.msaa_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Texture"),
                size: wgpu::Extent3d {
                    width: new_size.width,
                    height: new_size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa_view = self.msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Update camera aspect ratio
            self.camera.aspect_ratio = new_size.width as f32 / new_size.height as f32;
            self.update_camera_buffer();
        }
    }

    /// Update camera uniform buffer with current camera state
    pub fn update_camera_buffer(&self) {
        let camera_uniform = self.camera.to_uniform();
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
    }

    /// Handle a winit window event for egui
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.egui_state.on_window_event(&self.window, event).consumed
    }

    /// Convert world position to screen position
    pub fn world_to_screen(&self, world_x: f32, world_y: f32) -> (f32, f32) {
        // Apply camera transform: view_pos = (world_pos - camera_pos) * zoom
        let view_x = (world_x - self.camera.position[0] as f32) * self.camera.zoom;
        let view_y = (world_y - self.camera.position[1] as f32) * self.camera.zoom;

        // Correct for aspect ratio
        let ndc_x = view_x / self.camera.aspect_ratio;
        let ndc_y = view_y;

        // Convert NDC to screen coordinates
        let screen_x = (ndc_x + 1.0) * 0.5 * self.size.width as f32;
        let screen_y = (1.0 - ndc_y) * 0.5 * self.size.height as f32;

        (screen_x, screen_y)
    }

    /// Render a frame with body labels and time warp UI
    /// Returns the new warp index (may be changed by UI interaction)
    pub fn render(
        &mut self,
        body_names: &[String],
        warp_levels: &[f64],
        current_warp_index: usize,
    ) -> Result<usize, wgpu::SurfaceError> {
        // Update camera buffer before rendering
        self.update_camera_buffer();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build egui UI for hovered body label and time warp controls
        let hovered = self.hovered_body;
        let bodies_copy = self.bodies.clone();
        let size = self.size;
        let camera_pos = self.camera.position;
        let camera_zoom = self.camera.zoom;
        let aspect_ratio = self.camera.aspect_ratio;
        let scale_factor = self.window.scale_factor() as f32;
        let ship_orbit = self.ship_orbit_info.clone();
        let ship_velocity = self.ship_velocity;
        let ship_altitude = self.ship_altitude;
        let ship_throttle = self.ship_throttle;
        let ship_soi_name = self.ship_soi_name.clone();
        let ship_time_to_intercept = self.ship_time_to_intercept;
        let ap_markers = self.ap_markers.clone();
        let pe_markers = self.pe_markers.clone();

        let mut new_warp_index = current_warp_index;

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Time warp panel at top of screen
            egui::TopBottomPanel::top("time_warp_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Time Warp:");
                    for (i, &warp) in warp_levels.iter().enumerate() {
                        let label = if warp >= 1000000000.0 {
                            format!("{}B", (warp / 1000000000.0) as i32)
                        } else if warp >= 1000000.0 {
                            format!("{}M", (warp / 1000000.0) as i32)
                        } else if warp >= 1000.0 {
                            format!("{}K", (warp / 1000.0) as i32)
                        } else {
                            format!("{}x", warp as i32)
                        };

                        let is_selected = i == current_warp_index;
                        // Block selecting warp > 10x while thrusting
                        let blocked_throttle = ship_throttle > 0.0 && warp > 10.0;
                        // Block warps that would reach SOI boundary in < 0.5 seconds
                        let blocked_intercept = ship_time_to_intercept
                            .map(|t| t / warp < 0.5)
                            .unwrap_or(false);
                        let blocked = blocked_throttle || blocked_intercept;
                        let button = ui.add_enabled(!blocked, egui::SelectableLabel::new(is_selected, &label));
                        if button.clicked() && !blocked {
                            new_warp_index = i;
                        }
                    }

                    // Show current warp value
                    ui.separator();
                    let current_warp = warp_levels[current_warp_index];
                    ui.label(format!("Current: {}x", current_warp as i64));
                });

                // Orbital info display
                if let Some(ref orbit) = ship_orbit {
                    ui.separator();
                    ui.horizontal(|ui| {
                        // Format distance for display
                        let format_distance = |meters: f64, body_radius: f64| -> String {
                            let altitude = meters - body_radius;
                            if altitude.abs() >= 1e9 {
                                format!("{:.2} Gm", altitude / 1e9)
                            } else if altitude.abs() >= 1e6 {
                                format!("{:.1} Mm", altitude / 1e6)
                            } else if altitude.abs() >= 1e3 {
                                format!("{:.1} km", altitude / 1e3)
                            } else {
                                format!("{:.0} m", altitude)
                            }
                        };

                        // Format time for display
                        let format_time = |seconds: f64| -> String {
                            if seconds >= 86400.0 * 365.0 {
                                format!("{:.1}y", seconds / (86400.0 * 365.0))
                            } else if seconds >= 86400.0 {
                                format!("{:.1}d", seconds / 86400.0)
                            } else if seconds >= 3600.0 {
                                format!("{:.1}h", seconds / 3600.0)
                            } else if seconds >= 60.0 {
                                format!("{:.1}m", seconds / 60.0)
                            } else {
                                format!("{:.0}s", seconds)
                            }
                        };

                        let ap_alt = format_distance(orbit.apoapsis, orbit.parent_body_radius);
                        let pe_alt = format_distance(orbit.periapsis, orbit.parent_body_radius);

                        ui.label(format!("⬆ Ap: {}", ap_alt));
                        ui.label(format!("({})", format_time(orbit.time_to_apoapsis)));
                        ui.separator();
                        ui.label(format!("⬇ Pe: {}", pe_alt));
                        ui.label(format!("({})", format_time(orbit.time_to_periapsis)));
                        ui.separator();
                        ui.label(format!("⏱ T: {}", format_time(orbit.orbital_period)));
                        ui.separator();
                        ui.label(format!("e: {:.3}", orbit.eccentricity));
                        ui.separator();
                        ui.label(format!("◉ {}", ship_soi_name));
                    });
                }
            });

            // Bottom panel for velocity and altitude - compact single line
            egui::TopBottomPanel::bottom("flight_info_panel")
                .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(20, 20, 30, 200)))
                .show(ctx, |ui| {
                    // Format velocity
                    let vel_str = if ship_velocity >= 1000.0 {
                        format!("{:.2} km/s", ship_velocity / 1000.0)
                    } else {
                        format!("{:.1} m/s", ship_velocity)
                    };

                    // Format altitude
                    let alt_str = if ship_altitude.abs() >= 1e9 {
                        format!("{:.2} Gm", ship_altitude / 1e9)
                    } else if ship_altitude.abs() >= 1e6 {
                        format!("{:.2} Mm", ship_altitude / 1e6)
                    } else if ship_altitude.abs() >= 1e3 {
                        format!("{:.2} km", ship_altitude / 1e3)
                    } else {
                        format!("{:.1} m", ship_altitude)
                    };

                    ui.horizontal(|ui| {
                        let available = ui.available_width();
                        ui.add_space(available / 2.0 - 120.0);
                        ui.label(egui::RichText::new("VEL").size(11.0).color(egui::Color32::GRAY));
                        ui.label(egui::RichText::new(&vel_str).size(13.0).strong().color(egui::Color32::WHITE));
                        ui.add_space(30.0);
                        ui.label(egui::RichText::new("ALT").size(11.0).color(egui::Color32::GRAY));
                        ui.label(egui::RichText::new(&alt_str).size(13.0).strong().color(egui::Color32::WHITE));
                    });
                });

            // Throttle bar on left side
            egui::SidePanel::left("throttle_panel")
                .exact_width(50.0)
                .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(20, 20, 30, 200)))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("THR").size(10.0).color(egui::Color32::GRAY));
                        ui.add_space(5.0);

                        // Throttle percentage text
                        let throttle_pct = (ship_throttle * 100.0) as i32;
                        ui.label(egui::RichText::new(format!("{}%", throttle_pct))
                            .size(12.0)
                            .strong()
                            .color(egui::Color32::WHITE));
                        ui.add_space(5.0);

                        // Vertical throttle bar
                        let bar_height = 150.0;
                        let bar_width = 20.0;
                        let (rect, _response) = ui.allocate_exact_size(
                            egui::vec2(bar_width, bar_height),
                            egui::Sense::hover()
                        );

                        let painter = ui.painter();

                        // Background (empty part)
                        painter.rect_filled(
                            rect,
                            2.0,
                            egui::Color32::from_rgb(40, 40, 50)
                        );

                        // Filled part (from bottom up)
                        let fill_height = bar_height * ship_throttle as f32;
                        let fill_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x, rect.max.y - fill_height),
                            egui::vec2(bar_width, fill_height)
                        );

                        // Color gradient: green at low, yellow at mid, red at high
                        let fill_color = if ship_throttle < 0.5 {
                            egui::Color32::from_rgb(100, 200, 100) // Green
                        } else if ship_throttle < 0.8 {
                            egui::Color32::from_rgb(200, 200, 100) // Yellow
                        } else {
                            egui::Color32::from_rgb(200, 100, 100) // Red
                        };

                        painter.rect_filled(fill_rect, 2.0, fill_color);

                        // Border
                        painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
                    });
                });

            // Only draw label for hovered body
            if let Some(idx) = hovered {
                if let Some(body) = bodies_copy.get(idx) {
                    if let Some(name) = body_names.get(idx) {
                        let painter = ctx.layer_painter(egui::LayerId::new(
                            egui::Order::Foreground,
                            egui::Id::new("body_labels"),
                        ));

                        // Convert world to screen coordinates (in pixels)
                        // Calculate relative position in f64, then convert to f32 for screen
                        let view_x = ((body.x - camera_pos[0]) as f32) * camera_zoom;
                        let view_y = ((body.y - camera_pos[1]) as f32) * camera_zoom;
                        let ndc_x = view_x / aspect_ratio;
                        let ndc_y = view_y;
                        let screen_x_px = (ndc_x + 1.0) * 0.5 * size.width as f32;
                        let screen_y_px = (1.0 - ndc_y) * 0.5 * size.height as f32;

                        // Convert pixels to egui points
                        let screen_x = screen_x_px / scale_factor;
                        let screen_y = screen_y_px / scale_factor;

                        // Position label above the indicator circle
                        let label_y = screen_y - 20.0;

                        painter.text(
                            egui::pos2(screen_x, label_y),
                            egui::Align2::CENTER_BOTTOM,
                            name,
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }

            // Draw Apoapsis and Periapsis labels
            let marker_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("orbit_marker_labels"),
            ));

            // Helper to convert world position to screen position
            let world_to_screen = |world_rel: [f64; 2]| -> (f32, f32) {
                let view_x = (world_rel[0] as f32) * camera_zoom;
                let view_y = (world_rel[1] as f32) * camera_zoom;
                let ndc_x = view_x / aspect_ratio;
                let ndc_y = view_y;
                let screen_x_px = (ndc_x + 1.0) * 0.5 * size.width as f32;
                let screen_y_px = (1.0 - ndc_y) * 0.5 * size.height as f32;
                (screen_x_px / scale_factor, screen_y_px / scale_factor)
            };

            // Helper to format altitude
            let format_altitude = |meters: f64| -> String {
                if meters.abs() >= 1e9 {
                    format!("{:.2} Gm", meters / 1e9)
                } else if meters.abs() >= 1e6 {
                    format!("{:.1} Mm", meters / 1e6)
                } else if meters.abs() >= 1e3 {
                    format!("{:.1} km", meters / 1e3)
                } else {
                    format!("{:.0} m", meters)
                }
            };

            // Get mouse position for hover detection
            let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
            let hover_radius = 20.0; // pixels

            // Draw all apoapsis markers
            for (pos, altitude) in &ap_markers {
                let (screen_x, screen_y) = world_to_screen(*pos);
                let marker_screen_pos = egui::pos2(screen_x, screen_y);

                // Check if mouse is hovering over marker
                let is_hovered = mouse_pos.map_or(false, |mp| {
                    (mp - marker_screen_pos).length() < hover_radius
                });

                marker_painter.text(
                    egui::pos2(screen_x, screen_y - 12.0),
                    egui::Align2::CENTER_BOTTOM,
                    "Ap",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(255, 153, 51), // Orange
                );

                // Show altitude on hover
                if is_hovered {
                    marker_painter.text(
                        egui::pos2(screen_x, screen_y + 14.0),
                        egui::Align2::CENTER_TOP,
                        &format_altitude(*altitude),
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_rgb(255, 153, 51),
                    );
                }
            }

            // Draw all periapsis markers
            for (pos, altitude) in &pe_markers {
                let (screen_x, screen_y) = world_to_screen(*pos);
                let marker_screen_pos = egui::pos2(screen_x, screen_y);

                // Check if mouse is hovering over marker
                let is_hovered = mouse_pos.map_or(false, |mp| {
                    (mp - marker_screen_pos).length() < hover_radius
                });

                marker_painter.text(
                    egui::pos2(screen_x, screen_y - 12.0),
                    egui::Align2::CENTER_BOTTOM,
                    "Pe",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(77, 204, 255), // Cyan
                );

                // Show altitude on hover
                if is_hovered {
                    marker_painter.text(
                        egui::pos2(screen_x, screen_y + 14.0),
                        egui::Align2::CENTER_TOP,
                        &format_altitude(*altitude),
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_rgb(77, 204, 255),
                    );
                }
            }
        });

        self.egui_state.handle_platform_output(&self.window, full_output.platform_output);

        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_descriptor);

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,        // Render to MSAA texture
                    resolve_target: Some(&view),  // Resolve to swapchain
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Render egui on top (separate pass without MSAA)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Keep the previous render
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.egui_renderer.render(&mut render_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(new_warp_index)
    }

    /// Get window reference
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Update geometry with multiple bodies and their orbits
    /// scale: world units per meter (e.g., 1e-9 means 1 billion meters = 1 world unit)
    pub fn update_bodies_with_orbits(
        &mut self,
        bodies: &[(f64, f64, f64, [f32; 4])],
        orbits: &[Option<OrbitRenderData>],
        scale: f64,
    ) {
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        // Get camera position for relative coordinate calculation
        let cam_x = self.camera.position[0];
        let cam_y = self.camera.position[1];

        // First, draw all orbit lines (so they appear behind bodies)
        for orbit_opt in orbits {
            if let Some(orbit) = orbit_opt {
                let base_index = all_vertices.len() as u32;

                // Ellipse parameters
                let a = orbit.semi_major_axis; // semi-major axis
                let e = orbit.eccentricity;
                let b = a * (1.0 - e * e).sqrt(); // semi-minor axis
                let c = a * e; // distance from center to focus

                // The parent is at one focus, so ellipse center is offset
                let arg_peri = orbit.argument_of_periapsis;
                let center_x = orbit.parent_x - c * arg_peri.cos();
                let center_y = orbit.parent_y - c * arg_peri.sin();

                // Number of segments for the orbit line
                let segments = 256u32;
                let line_width = 0.002 / self.camera.zoom as f64; // Thin line in world units

                // Generate orbit ellipse vertices (inner and outer for line thickness)
                for i in 0..segments {
                    let angle = (i as f64 / segments as f64) * std::f64::consts::TAU;

                    // Point on ellipse (before rotation)
                    let ex = a * angle.cos();
                    let ey = b * angle.sin();

                    // Rotate by argument of periapsis
                    let rx = ex * arg_peri.cos() - ey * arg_peri.sin();
                    let ry = ex * arg_peri.sin() + ey * arg_peri.cos();

                    // Final position
                    let px = center_x + rx;
                    let py = center_y + ry;

                    // Calculate normal for line thickness
                    let next_angle = ((i + 1) as f64 / segments as f64) * std::f64::consts::TAU;
                    let next_ex = a * next_angle.cos();
                    let next_ey = b * next_angle.sin();
                    let next_rx = next_ex * arg_peri.cos() - next_ey * arg_peri.sin();
                    let next_ry = next_ex * arg_peri.sin() + next_ey * arg_peri.cos();

                    let dx = next_rx - rx;
                    let dy = next_ry - ry;
                    let len = (dx * dx + dy * dy).sqrt();
                    let nx = -dy / len * line_width;
                    let ny = dx / len * line_width;

                    // Outer vertex
                    let rel_outer_x = (px + nx - cam_x) as f32;
                    let rel_outer_y = (py + ny - cam_y) as f32;
                    all_vertices.push(Vertex {
                        position: [rel_outer_x, rel_outer_y],
                        color: orbit.color,
                    });

                    // Inner vertex
                    let rel_inner_x = (px - nx - cam_x) as f32;
                    let rel_inner_y = (py - ny - cam_y) as f32;
                    all_vertices.push(Vertex {
                        position: [rel_inner_x, rel_inner_y],
                        color: [orbit.color[0] * 0.5, orbit.color[1] * 0.5, orbit.color[2] * 0.5, orbit.color[3] * 0.7],
                    });
                }

                // Create indices for orbit ring
                for i in 0..segments {
                    let i0 = base_index + i * 2;
                    let i1 = base_index + i * 2 + 1;
                    let i2 = base_index + ((i + 1) % segments) * 2;
                    let i3 = base_index + ((i + 1) % segments) * 2 + 1;

                    all_indices.push(i0);
                    all_indices.push(i2);
                    all_indices.push(i1);

                    all_indices.push(i1);
                    all_indices.push(i2);
                    all_indices.push(i3);
                }
            }
        }

        // Now draw bodies on top of orbit lines
        self.add_body_vertices(&mut all_vertices, &mut all_indices, bodies, scale);

        self.num_indices = all_indices.len() as u32;

        // Update buffers
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_indices));
    }

    /// Update geometry with bodies, orbits, and optionally a ship
    pub fn update_bodies_orbits_and_ship(
        &mut self,
        bodies: &[(f64, f64, f64, [f32; 4])],
        orbits: &[Option<OrbitRenderData>],
        ship: Option<&ShipRenderData>,
        scale: f64,
    ) {
        // Store ship info for UI display
        self.ship_orbit_info = ship.and_then(|s| s.orbit.clone());
        if let Some(s) = ship {
            self.ship_velocity = s.velocity;
            self.ship_altitude = s.altitude;
            self.ship_throttle = s.throttle;
            self.ship_soi_name = s.soi_body_name.clone();
            self.ship_time_to_intercept = s.time_to_intercept;
        }

        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        let cam_x = self.camera.position[0];
        let cam_y = self.camera.position[1];

        // First, draw all orbit lines (so they appear behind bodies)
        for orbit_opt in orbits {
            if let Some(orbit) = orbit_opt {
                let base_index = all_vertices.len() as u32;

                let a = orbit.semi_major_axis;
                let e = orbit.eccentricity;
                let b = a * (1.0 - e * e).sqrt();
                let c = a * e;

                let arg_peri = orbit.argument_of_periapsis;
                let center_x = orbit.parent_x - c * arg_peri.cos();
                let center_y = orbit.parent_y - c * arg_peri.sin();

                let segments = 256u32;
                let line_width = 0.002 / self.camera.zoom as f64;

                for i in 0..segments {
                    let angle = (i as f64 / segments as f64) * std::f64::consts::TAU;

                    let ex = a * angle.cos();
                    let ey = b * angle.sin();

                    let rx = ex * arg_peri.cos() - ey * arg_peri.sin();
                    let ry = ex * arg_peri.sin() + ey * arg_peri.cos();

                    let px = center_x + rx;
                    let py = center_y + ry;

                    let next_angle = ((i + 1) as f64 / segments as f64) * std::f64::consts::TAU;
                    let next_ex = a * next_angle.cos();
                    let next_ey = b * next_angle.sin();
                    let next_rx = next_ex * arg_peri.cos() - next_ey * arg_peri.sin();
                    let next_ry = next_ex * arg_peri.sin() + next_ey * arg_peri.cos();

                    let dx = next_rx - rx;
                    let dy = next_ry - ry;
                    let len = (dx * dx + dy * dy).sqrt();
                    let nx = -dy / len * line_width;
                    let ny = dx / len * line_width;

                    let rel_outer_x = (px + nx - cam_x) as f32;
                    let rel_outer_y = (py + ny - cam_y) as f32;
                    all_vertices.push(Vertex {
                        position: [rel_outer_x, rel_outer_y],
                        color: orbit.color,
                    });

                    let rel_inner_x = (px - nx - cam_x) as f32;
                    let rel_inner_y = (py - ny - cam_y) as f32;
                    all_vertices.push(Vertex {
                        position: [rel_inner_x, rel_inner_y],
                        color: [orbit.color[0] * 0.5, orbit.color[1] * 0.5, orbit.color[2] * 0.5, orbit.color[3] * 0.7],
                    });
                }

                for i in 0..segments {
                    let i0 = base_index + i * 2;
                    let i1 = base_index + i * 2 + 1;
                    let i2 = base_index + ((i + 1) % segments) * 2;
                    let i3 = base_index + ((i + 1) % segments) * 2 + 1;

                    all_indices.push(i0);
                    all_indices.push(i2);
                    all_indices.push(i1);

                    all_indices.push(i1);
                    all_indices.push(i2);
                    all_indices.push(i3);
                }
            }
        }

        // Draw ship orbit line (patched conics) - before bodies, so it appears behind
        // Only show orbit line when ship is small on screen (< 5 pixels)
        self.ap_markers.clear();
        self.pe_markers.clear();

        if let Some(ship_data) = ship {
            let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;
            let ship_pixels = ship_data.size as f32 * pixels_per_world_unit * 2.0;

            if ship_pixels < 5.0 && !ship_data.patched_trajectory.is_empty() {
                let line_width = 0.002 / self.camera.zoom as f64;
                let marker_radius = 0.008 / self.camera.zoom as f64;
                let marker_segments = 16u32;

                // Draw each patched conic segment
                for segment in &ship_data.patched_trajectory {
                    let e = segment.eccentricity;
                    let arg_peri = segment.argument_of_periapsis;

                    if e >= 1.0 {
                        // Hyperbolic trajectory - draw from ship position to SOI exit
                        let a_abs = segment.semi_major_axis.abs();

                        // Semi-latus rectum: p = |a| * (e² - 1)
                        let p = a_abs * (e * e - 1.0);

                        // True anomaly is limited: |ν| < arccos(-1/e)
                        let max_true_anomaly = (-1.0 / e).acos();

                        // Start from ship's current true anomaly
                        let start_ta = segment.start_true_anomaly;

                        // Calculate SOI exit true anomaly if not provided
                        let end_ta = segment.end_true_anomaly.unwrap_or_else(|| {
                            // Calculate true anomaly at SOI exit: r = p / (1 + e*cos(ν))
                            // Solving: cos(ν) = (p / soi_radius - 1) / e
                            let soi_radius = segment.soi_radius;
                            if soi_radius > 0.0 && soi_radius.is_finite() {
                                let cos_nu_exit = (p / soi_radius - 1.0) / e;
                                if cos_nu_exit.abs() <= 1.0 {
                                    let nu_exit = cos_nu_exit.acos();
                                    // Choose exit direction based on orbit direction
                                    // Prograde: exit on outgoing leg (positive ta)
                                    // Retrograde: exit on incoming leg (negative ta)
                                    if segment.retrograde { -nu_exit } else { nu_exit }
                                } else {
                                    // Fallback to asymptote limit
                                    if segment.retrograde { -(max_true_anomaly - HYPERBOLIC_RENDER_MARGIN) } else { max_true_anomaly - HYPERBOLIC_RENDER_MARGIN }
                                }
                            } else {
                                // Fallback to asymptote limit
                                if segment.retrograde { -(max_true_anomaly - HYPERBOLIC_RENDER_MARGIN) } else { max_true_anomaly - HYPERBOLIC_RENDER_MARGIN }
                            }
                        });

                        // Generate points along the hyperbola
                        let num_points = 1024usize;
                        let mut points: Vec<(f64, f64)> = Vec::with_capacity(num_points);

                        for i in 0..num_points {
                            let t = i as f64 / (num_points - 1) as f64;
                            let ta = start_ta + t * (end_ta - start_ta);

                            // Skip if too close to asymptote
                            if ta.abs() >= max_true_anomaly - HYPERBOLIC_SKIP_MARGIN {
                                continue;
                            }

                            // Calculate radius from orbit equation: r = p / (1 + e*cos(ν))
                            let denom = 1.0 + e * ta.cos();
                            if denom <= 0.001 {
                                continue; // Near asymptote
                            }
                            let r = p / denom;

                            // Skip invalid radii
                            if r <= 0.0 || !r.is_finite() {
                                continue;
                            }

                            // Position relative to parent (focus at origin)
                            let angle = ta + arg_peri;
                            let px = segment.parent_x + r * angle.cos();
                            let py = segment.parent_y + r * angle.sin();

                            points.push((px, py));
                        }

                        // Draw line segments between consecutive points
                        if points.len() >= 2 {
                            let base_index = all_vertices.len() as u32;

                            for i in 0..points.len() - 1 {
                                let (px, py) = points[i];
                                let (nx_pt, ny_pt) = points[i + 1];

                                let dx = nx_pt - px;
                                let dy = ny_pt - py;
                                let len = (dx * dx + dy * dy).sqrt();
                                if len < 1e-10 {
                                    continue;
                                }

                                // Perpendicular for line width
                                let nx = -dy / len * line_width;
                                let ny = dx / len * line_width;

                                let rel_outer_x = (px + nx - cam_x) as f32;
                                let rel_outer_y = (py + ny - cam_y) as f32;
                                all_vertices.push(Vertex {
                                    position: [rel_outer_x, rel_outer_y],
                                    color: segment.color,
                                });

                                let rel_inner_x = (px - nx - cam_x) as f32;
                                let rel_inner_y = (py - ny - cam_y) as f32;
                                all_vertices.push(Vertex {
                                    position: [rel_inner_x, rel_inner_y],
                                    color: [segment.color[0] * 0.5, segment.color[1] * 0.5, segment.color[2] * 0.5, segment.color[3] * 0.7],
                                });
                            }

                            let num_line_segments = (all_vertices.len() as u32 - base_index) / 2;
                            for i in 0..num_line_segments.saturating_sub(1) {
                                let i0 = base_index + i * 2;
                                let i1 = base_index + i * 2 + 1;
                                let i2 = base_index + (i + 1) * 2;
                                let i3 = base_index + (i + 1) * 2 + 1;

                                all_indices.push(i0);
                                all_indices.push(i2);
                                all_indices.push(i1);

                                all_indices.push(i1);
                                all_indices.push(i2);
                                all_indices.push(i3);
                            }
                        }

                        // Draw periapsis marker for hyperbolic trajectory
                        // Periapsis is at true anomaly = 0
                        let a_abs = segment.semi_major_axis.abs();
                        let p = a_abs * (e * e - 1.0);
                        let pe_r = p / (1.0 + e); // Distance at periapsis
                        let pe_x = segment.parent_x + pe_r * arg_peri.cos();
                        let pe_y = segment.parent_y + pe_r * arg_peri.sin();

                        // Use dimmer color for future segments
                        let alpha = if segment.is_first_segment { 1.0 } else { 0.6 };
                        let pe_color = [0.3, 0.8, 1.0, alpha];

                        let pe_base = all_vertices.len() as u32;
                        all_vertices.push(Vertex {
                            position: [(pe_x - cam_x) as f32, (pe_y - cam_y) as f32],
                            color: pe_color,
                        });
                        for i in 0..marker_segments {
                            let angle = (i as f64 / marker_segments as f64) * std::f64::consts::TAU;
                            all_vertices.push(Vertex {
                                position: [
                                    (pe_x + marker_radius * angle.cos() - cam_x) as f32,
                                    (pe_y + marker_radius * angle.sin() - cam_y) as f32,
                                ],
                                color: pe_color,
                            });
                        }
                        for i in 0..marker_segments {
                            all_indices.push(pe_base);
                            all_indices.push(pe_base + 1 + i);
                            all_indices.push(pe_base + 1 + (i + 1) % marker_segments);
                        }

                        // Store position and altitude for UI hover
                        // pe_r is in scaled units, convert to meters then subtract body radius
                        let pe_altitude = (pe_r / segment.render_scale) - segment.parent_body_radius;
                        self.pe_markers.push(([pe_x - cam_x, pe_y - cam_y], pe_altitude));

                        continue; // Skip ellipse drawing code
                    }

                    // Elliptical orbit
                    let a = segment.semi_major_axis;
                    let b = a * (1.0 - e * e).sqrt();
                    let c = a * e;

                    let center_x = segment.parent_x - c * arg_peri.cos();
                    let center_y = segment.parent_y - c * arg_peri.sin();

                    // Determine angle range to draw
                    let (start_angle, angle_span) = match segment.end_true_anomaly {
                        Some(end_ta) => {
                            // Partial orbit - convert true anomaly to eccentric anomaly
                            let start_ta = segment.start_true_anomaly;
                            let start_ea = (start_ta.sin() * (1.0 - e * e).sqrt()).atan2(e + start_ta.cos());
                            let end_ea = (end_ta.sin() * (1.0 - e * e).sqrt()).atan2(e + end_ta.cos());

                            // Calculate span based on orbit direction
                            let span = if segment.retrograde {
                                // Retrograde: going from start toward end in decreasing direction
                                let mut s = start_ea - end_ea;
                                if s < 0.0 {
                                    s += std::f64::consts::TAU;
                                }
                                -s // Negative span for retrograde (draw clockwise)
                            } else {
                                // Prograde: going from start toward end in increasing direction
                                let mut s = end_ea - start_ea;
                                if s < 0.0 {
                                    s += std::f64::consts::TAU;
                                }
                                s
                            };
                            (start_ea, span)
                        }
                        None => (0.0, std::f64::consts::TAU), // Full orbit
                    };

                    let num_segments = ((angle_span.abs() / std::f64::consts::TAU) * 512.0).max(16.0) as u32;
                    let base_index = all_vertices.len() as u32;

                    for i in 0..num_segments {
                        let t = i as f64 / num_segments as f64;
                        let angle = start_angle + t * angle_span;

                        let ex = a * angle.cos();
                        let ey = b * angle.sin();

                        let rx = ex * arg_peri.cos() - ey * arg_peri.sin();
                        let ry = ex * arg_peri.sin() + ey * arg_peri.cos();

                        let px = center_x + rx;
                        let py = center_y + ry;

                        let next_t = (i + 1) as f64 / num_segments as f64;
                        let next_angle = start_angle + next_t * angle_span;
                        let next_ex = a * next_angle.cos();
                        let next_ey = b * next_angle.sin();
                        let next_rx = next_ex * arg_peri.cos() - next_ey * arg_peri.sin();
                        let next_ry = next_ex * arg_peri.sin() + next_ey * arg_peri.cos();

                        let dx = next_rx - rx;
                        let dy = next_ry - ry;
                        let len = (dx * dx + dy * dy).sqrt();
                        if len < 1e-10 {
                            continue;
                        }
                        let nx = -dy / len * line_width;
                        let ny = dx / len * line_width;

                        let rel_outer_x = (px + nx - cam_x) as f32;
                        let rel_outer_y = (py + ny - cam_y) as f32;
                        all_vertices.push(Vertex {
                            position: [rel_outer_x, rel_outer_y],
                            color: segment.color,
                        });

                        let rel_inner_x = (px - nx - cam_x) as f32;
                        let rel_inner_y = (py - ny - cam_y) as f32;
                        all_vertices.push(Vertex {
                            position: [rel_inner_x, rel_inner_y],
                            color: [segment.color[0] * 0.5, segment.color[1] * 0.5, segment.color[2] * 0.5, segment.color[3] * 0.7],
                        });
                    }

                    for i in 0..(num_segments.saturating_sub(1)) {
                        let i0 = base_index + i * 2;
                        let i1 = base_index + i * 2 + 1;
                        let i2 = base_index + (i + 1) * 2;
                        let i3 = base_index + (i + 1) * 2 + 1;

                        all_indices.push(i0);
                        all_indices.push(i2);
                        all_indices.push(i1);

                        all_indices.push(i1);
                        all_indices.push(i2);
                        all_indices.push(i3);
                    }

                    // Draw Ap/Pe markers for all segments (dimmer for future segments)
                    // For first segment with intercept, only show markers if they're in the traversed arc
                    let alpha = if segment.is_first_segment { 1.0 } else { 0.6 };

                    // Helper to check if a true anomaly is in the arc from start to end
                    let is_in_arc = |marker_ta: f64, start_ta: f64, end_ta: f64, retrograde: bool| -> bool {
                        let tau = std::f64::consts::TAU;
                        let normalize = |a: f64| a.rem_euclid(tau);
                        let marker = normalize(marker_ta);
                        let start = normalize(start_ta);
                        let end = normalize(end_ta);

                        if retrograde {
                            // Moving in decreasing direction
                            if start >= end {
                                marker <= start && marker >= end
                            } else {
                                marker >= end || marker <= start
                            }
                        } else {
                            // Moving in increasing direction
                            if start <= end {
                                marker >= start && marker <= end
                            } else {
                                marker >= start || marker <= end
                            }
                        }
                    };

                    // Calculate orbital distances for altitude (convert from scaled to meters)
                    let pe_distance = a * (1.0 - e) / segment.render_scale;
                    let ap_distance = a * (1.0 + e) / segment.render_scale;
                    let pe_altitude = pe_distance - segment.parent_body_radius;
                    let ap_altitude = ap_distance - segment.parent_body_radius;

                    // Check if markers are in traversed arc (for first segment with intercept)
                    let (show_pe, show_ap) = if segment.is_first_segment {
                        if let Some(end_ta) = segment.end_true_anomaly {
                            let start_ta = segment.start_true_anomaly;
                            let pe_in_arc = is_in_arc(0.0, start_ta, end_ta, segment.retrograde);
                            let ap_in_arc = is_in_arc(std::f64::consts::PI, start_ta, end_ta, segment.retrograde);
                            (pe_in_arc, ap_in_arc)
                        } else {
                            (true, true) // Full orbit, show both
                        }
                    } else {
                        (true, true) // Future segments always show markers
                    };

                    // Periapsis marker (at true anomaly 0) - cyan/blue
                    if show_pe {
                        let pe_ex = a;
                        let pe_ey = 0.0;
                        let pe_rx = pe_ex * arg_peri.cos() - pe_ey * arg_peri.sin();
                        let pe_ry = pe_ex * arg_peri.sin() + pe_ey * arg_peri.cos();
                        let pe_x = center_x + pe_rx;
                        let pe_y = center_y + pe_ry;
                        let pe_color = [0.3, 0.8, 1.0, alpha];

                        let pe_base = all_vertices.len() as u32;
                        all_vertices.push(Vertex {
                            position: [(pe_x - cam_x) as f32, (pe_y - cam_y) as f32],
                            color: pe_color,
                        });
                        for i in 0..marker_segments {
                            let angle = (i as f64 / marker_segments as f64) * std::f64::consts::TAU;
                            all_vertices.push(Vertex {
                                position: [
                                    (pe_x + marker_radius * angle.cos() - cam_x) as f32,
                                    (pe_y + marker_radius * angle.sin() - cam_y) as f32,
                                ],
                                color: pe_color,
                            });
                        }
                        for i in 0..marker_segments {
                            all_indices.push(pe_base);
                            all_indices.push(pe_base + 1 + i);
                            all_indices.push(pe_base + 1 + (i + 1) % marker_segments);
                        }
                        // Store for UI hover
                        self.pe_markers.push(([pe_x - cam_x, pe_y - cam_y], pe_altitude));
                    }

                    // Apoapsis marker (at true anomaly π) - orange
                    if show_ap {
                        let ap_ex = -a;
                        let ap_ey = 0.0;
                        let ap_rx = ap_ex * arg_peri.cos() - ap_ey * arg_peri.sin();
                        let ap_ry = ap_ex * arg_peri.sin() + ap_ey * arg_peri.cos();
                        let ap_x = center_x + ap_rx;
                        let ap_y = center_y + ap_ry;
                        let ap_color = [1.0, 0.6, 0.2, alpha];

                        let ap_base = all_vertices.len() as u32;
                        all_vertices.push(Vertex {
                            position: [(ap_x - cam_x) as f32, (ap_y - cam_y) as f32],
                            color: ap_color,
                        });
                        for i in 0..marker_segments {
                            let angle = (i as f64 / marker_segments as f64) * std::f64::consts::TAU;
                            all_vertices.push(Vertex {
                                position: [
                                    (ap_x + marker_radius * angle.cos() - cam_x) as f32,
                                    (ap_y + marker_radius * angle.sin() - cam_y) as f32,
                                ],
                                color: ap_color,
                            });
                        }
                        for i in 0..marker_segments {
                            all_indices.push(ap_base);
                            all_indices.push(ap_base + 1 + i);
                            all_indices.push(ap_base + 1 + (i + 1) % marker_segments);
                        }
                        // Store for UI hover
                        self.ap_markers.push(([ap_x - cam_x, ap_y - cam_y], ap_altitude));
                    }
                }
            }
        }

        // Draw bodies
        self.add_body_vertices(&mut all_vertices, &mut all_indices, bodies, scale);

        // Draw ship on top of everything
        if let Some(ship_data) = ship {
            let rel_x = (ship_data.x - cam_x) as f32;
            let rel_y = (ship_data.y - cam_y) as f32;
            let size = ship_data.size as f32;
            let rotation = ship_data.rotation as f32;

            // Calculate ship size in pixels
            let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;
            let ship_pixels = size * pixels_per_world_unit * 2.0;
            let needs_indicator = ship_pixels < 5.0;

            // Draw the actual ship triangle if visible
            if ship_pixels >= 1.0 {
                let base_index = all_vertices.len() as u32;

                let nose_angle = rotation;
                let back_left_angle = rotation + std::f32::consts::PI * 0.8;
                let back_right_angle = rotation - std::f32::consts::PI * 0.8;

                // Nose vertex
                all_vertices.push(Vertex {
                    position: [
                        rel_x + size * nose_angle.cos(),
                        rel_y + size * nose_angle.sin(),
                    ],
                    color: ship_data.color,
                });

                // Back left vertex
                all_vertices.push(Vertex {
                    position: [
                        rel_x + size * 0.6 * back_left_angle.cos(),
                        rel_y + size * 0.6 * back_left_angle.sin(),
                    ],
                    color: ship_data.color,
                });

                // Back right vertex
                all_vertices.push(Vertex {
                    position: [
                        rel_x + size * 0.6 * back_right_angle.cos(),
                        rel_y + size * 0.6 * back_right_angle.sin(),
                    ],
                    color: ship_data.color,
                });

                all_indices.push(base_index);
                all_indices.push(base_index + 1);
                all_indices.push(base_index + 2);
            }

            // Draw triangle indicator when ship is too small
            if needs_indicator {
                let base_index = all_vertices.len() as u32;

                // Fixed screen-size indicator (16 pixels)
                let indicator_screen_radius = 16.0f32;
                let indicator_size = (indicator_screen_radius / pixels_per_world_unit) as f32;

                // Triangle indicator pointing in direction of ship rotation
                let nose_angle = rotation;
                let back_left_angle = rotation + std::f32::consts::PI * 0.8;
                let back_right_angle = rotation - std::f32::consts::PI * 0.8;

                // Outer triangle (indicator)
                all_vertices.push(Vertex {
                    position: [
                        rel_x + indicator_size * nose_angle.cos(),
                        rel_y + indicator_size * nose_angle.sin(),
                    ],
                    color: ship_data.color,
                });

                all_vertices.push(Vertex {
                    position: [
                        rel_x + indicator_size * 0.6 * back_left_angle.cos(),
                        rel_y + indicator_size * 0.6 * back_left_angle.sin(),
                    ],
                    color: ship_data.color,
                });

                all_vertices.push(Vertex {
                    position: [
                        rel_x + indicator_size * 0.6 * back_right_angle.cos(),
                        rel_y + indicator_size * 0.6 * back_right_angle.sin(),
                    ],
                    color: ship_data.color,
                });

                // Inner triangle (darker, for outline effect)
                let inner_size = indicator_size * 0.6;
                let inner_color = [
                    ship_data.color[0] * 0.3,
                    ship_data.color[1] * 0.3,
                    ship_data.color[2] * 0.3,
                    ship_data.color[3],
                ];

                all_vertices.push(Vertex {
                    position: [
                        rel_x + inner_size * nose_angle.cos(),
                        rel_y + inner_size * nose_angle.sin(),
                    ],
                    color: inner_color,
                });

                all_vertices.push(Vertex {
                    position: [
                        rel_x + inner_size * 0.6 * back_left_angle.cos(),
                        rel_y + inner_size * 0.6 * back_left_angle.sin(),
                    ],
                    color: inner_color,
                });

                all_vertices.push(Vertex {
                    position: [
                        rel_x + inner_size * 0.6 * back_right_angle.cos(),
                        rel_y + inner_size * 0.6 * back_right_angle.sin(),
                    ],
                    color: inner_color,
                });

                // Outer triangle
                all_indices.push(base_index);
                all_indices.push(base_index + 1);
                all_indices.push(base_index + 2);

                // Inner triangle
                all_indices.push(base_index + 3);
                all_indices.push(base_index + 4);
                all_indices.push(base_index + 5);
            }
        }

        self.num_indices = all_indices.len() as u32;

        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_indices));
    }

    /// Helper to add body vertices (extracted for reuse)
    fn add_body_vertices(
        &mut self,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        bodies: &[(f64, f64, f64, [f32; 4])],
        scale: f64,
    ) {
        // Store body data for hit testing
        self.bodies.clear();

        // Calculate world units per pixel for indicator sizing
        let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;
        let indicator_screen_radius = 16.0f32;
        let indicator_world_radius = (indicator_screen_radius / pixels_per_world_unit) as f64;
        let min_body_pixels = 5.0f32;

        let cam_x = self.camera.position[0];
        let cam_y = self.camera.position[1];

        for (x, y, radius, color) in bodies {
            let rel_x = ((*x * scale) - cam_x) as f32;
            let rel_y = ((*y * scale) - cam_y) as f32;
            let r = (*radius * scale) as f32;

            let cx = *x * scale;
            let cy = *y * scale;
            let r_f64 = *radius * scale;

            let body_pixel_radius = r * pixels_per_world_unit;
            let body_pixels = body_pixel_radius * 2.0;
            let needs_indicator = body_pixels < min_body_pixels;

            self.bodies.push(BodyData {
                x: cx,
                y: cy,
                radius: r_f64,
                indicator_radius: if needs_indicator { indicator_world_radius } else { 0.0 },
            });

            let min_draw_pixels = 1.0;
            let body_is_visible = body_pixels >= min_draw_pixels;

            if body_is_visible {
                let base_index = all_vertices.len() as u32;
                let draw_r = r;

                let draw_pixel_radius = draw_r * pixels_per_world_unit;
                let circumference_pixels = 2.0 * std::f32::consts::PI * draw_pixel_radius;
                let raw_segments = (circumference_pixels / 3.0) as u32;
                let segments = raw_segments.clamp(64, 4096) & !1;

                all_vertices.push(Vertex {
                    position: [rel_x, rel_y],
                    color: *color,
                });

                for i in 0..segments {
                    let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                    all_vertices.push(Vertex {
                        position: [rel_x + draw_r * angle.cos(), rel_y + draw_r * angle.sin()],
                        color: *color,
                    });
                }

                for i in 0..segments {
                    all_indices.push(base_index);
                    all_indices.push(base_index + i + 1);
                    all_indices.push(base_index + ((i + 1) % segments) + 1);
                }
            }

            if needs_indicator {
                let base_index = all_vertices.len() as u32;
                let ring_outer = indicator_world_radius as f32;
                let ring_inner = (indicator_world_radius * 0.7) as f32;
                let ring_segments = 64u32;

                for i in 0..ring_segments {
                    let angle = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    all_vertices.push(Vertex {
                        position: [rel_x + ring_outer * cos_a, rel_y + ring_outer * sin_a],
                        color: *color,
                    });
                    all_vertices.push(Vertex {
                        position: [rel_x + ring_inner * cos_a, rel_y + ring_inner * sin_a],
                        color: [color[0] * 0.3, color[1] * 0.3, color[2] * 0.3, color[3] * 0.5],
                    });
                }

                for i in 0..ring_segments {
                    let i0 = base_index + i * 2;
                    let i1 = base_index + i * 2 + 1;
                    let i2 = base_index + ((i + 1) % ring_segments) * 2;
                    let i3 = base_index + ((i + 1) % ring_segments) * 2 + 1;

                    all_indices.push(i0);
                    all_indices.push(i2);
                    all_indices.push(i1);

                    all_indices.push(i1);
                    all_indices.push(i2);
                    all_indices.push(i3);
                }
            }
        }
    }

    /// Update geometry with multiple bodies (legacy method without orbits)
    /// scale: world units per meter (e.g., 1e-9 means 1 billion meters = 1 world unit)
    pub fn update_bodies(&mut self, bodies: &[(f64, f64, f64, [f32; 4])], scale: f64) {
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        // Store body data for hit testing
        self.bodies.clear();

        // Calculate world units per pixel for indicator sizing
        let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;
        let indicator_screen_radius = 16.0f32; // pixels
        let indicator_world_radius = (indicator_screen_radius / pixels_per_world_unit) as f64;
        let min_body_pixels = 5.0f32;

        // Get camera position for relative coordinate calculation
        let cam_x = self.camera.position[0];
        let cam_y = self.camera.position[1];

        for (x, y, radius, color) in bodies {
            // Calculate position relative to camera in f64 first, then convert to f32
            // This preserves precision for small bodies far from origin
            let rel_x = ((*x * scale) - cam_x) as f32;
            let rel_y = ((*y * scale) - cam_y) as f32;
            let r = (*radius * scale) as f32;

            // For hit testing, we still need absolute world coordinates (f64 for precision)
            let cx = *x * scale;
            let cy = *y * scale;
            let r_f64 = *radius * scale;

            // Calculate body size in pixels
            let body_pixel_radius = r * pixels_per_world_unit;
            let body_pixels = body_pixel_radius * 2.0;
            let needs_indicator = body_pixels < min_body_pixels;

            // Store body for hit testing
            self.bodies.push(BodyData {
                x: cx,
                y: cy,
                radius: r_f64,
                indicator_radius: if needs_indicator { indicator_world_radius } else { 0.0 },
            });

            // Draw the body itself (filled circle)
            // Only draw if body is at least 1 pixel, otherwise just show indicator
            let min_draw_pixels = 1.0;
            let body_is_visible = body_pixels >= min_draw_pixels;

            if body_is_visible {
                let base_index = all_vertices.len() as u32;

                // Use actual world radius - no clamping!
                // The indicator ring handles visibility when zoomed out
                // When zoomed in, the body grows naturally via GPU transform
                let draw_r = r;

                // Calculate segments based on current screen size
                let draw_pixel_radius = draw_r * pixels_per_world_unit;
                let circumference_pixels = 2.0 * std::f32::consts::PI * draw_pixel_radius;
                // Each segment spans at most 3 pixels of arc length
                let raw_segments = (circumference_pixels / 3.0) as u32;
                let segments = raw_segments.clamp(64, 4096) & !1; // Make even, min 64, max 4096

                // Center vertex (relative to camera)
                all_vertices.push(Vertex {
                    position: [rel_x, rel_y],
                    color: *color,
                });

                // Edge vertices (relative to camera)
                for i in 0..segments {
                    let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                    all_vertices.push(Vertex {
                        position: [rel_x + draw_r * angle.cos(), rel_y + draw_r * angle.sin()],
                        color: *color,
                    });
                }

                // Triangle fan indices
                for i in 0..segments {
                    all_indices.push(base_index);
                    all_indices.push(base_index + i + 1);
                    all_indices.push(base_index + ((i + 1) % segments) + 1);
                }
            }

            // Draw indicator ring if body is too small
            if needs_indicator {
                let base_index = all_vertices.len() as u32;
                // Cast to f32 for vertex positions
                let ring_outer = indicator_world_radius as f32;
                let ring_inner = (indicator_world_radius * 0.7) as f32;
                let ring_segments = 64u32; // Smooth indicator rings

                // Create ring vertices (inner and outer circles, relative to camera)
                for i in 0..ring_segments {
                    let angle = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Outer vertex
                    all_vertices.push(Vertex {
                        position: [rel_x + ring_outer * cos_a, rel_y + ring_outer * sin_a],
                        color: *color,
                    });
                    // Inner vertex
                    all_vertices.push(Vertex {
                        position: [rel_x + ring_inner * cos_a, rel_y + ring_inner * sin_a],
                        color: [color[0] * 0.3, color[1] * 0.3, color[2] * 0.3, color[3] * 0.5],
                    });
                }

                // Create ring triangles
                for i in 0..ring_segments {
                    let i0 = base_index + i * 2;
                    let i1 = base_index + i * 2 + 1;
                    let i2 = base_index + ((i + 1) % ring_segments) * 2;
                    let i3 = base_index + ((i + 1) % ring_segments) * 2 + 1;

                    // Two triangles per segment
                    all_indices.push(i0);
                    all_indices.push(i2);
                    all_indices.push(i1);

                    all_indices.push(i1);
                    all_indices.push(i2);
                    all_indices.push(i3);
                }
            }
        }

        self.num_indices = all_indices.len() as u32;

        // Update buffers (u32 indices are already 4-byte aligned)
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_indices));
    }

    /// Update hover state based on mouse position
    pub fn update_hover(&mut self, screen_x: f32, screen_y: f32) {
        let world_pos = self.camera.screen_to_world(
            screen_x,
            screen_y,
            self.size.width as f32,
            self.size.height as f32,
        );

        self.hovered_body = None;
        let mut closest_dist = f64::MAX;

        for (i, body) in self.bodies.iter().enumerate() {
            let dx = world_pos[0] - body.x;
            let dy = world_pos[1] - body.y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Use indicator radius if present, otherwise body radius
            let hover_radius = if body.indicator_radius > 0.0 {
                body.indicator_radius
            } else {
                body.radius
            };

            if dist <= hover_radius && dist < closest_dist {
                closest_dist = dist;
                self.hovered_body = Some(i);
            }
        }
    }

    /// Find body at screen position, returns index of closest body within click range
    pub fn body_at_screen_pos(&self, screen_x: f32, screen_y: f32) -> Option<usize> {
        let world_pos = self.camera.screen_to_world(
            screen_x,
            screen_y,
            self.size.width as f32,
            self.size.height as f32,
        );

        let mut closest: Option<(usize, f64)> = None;

        for (i, body) in self.bodies.iter().enumerate() {
            let dx = world_pos[0] - body.x;
            let dy = world_pos[1] - body.y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Use indicator radius if present, otherwise use body radius with minimum
            let click_radius = if body.indicator_radius > 0.0 {
                body.indicator_radius
            } else {
                body.radius
            };

            if dist <= click_radius {
                // Select closest body center to click point
                match closest {
                    None => closest = Some((i, dist)),
                    Some((_, prev_dist)) if dist < prev_dist => closest = Some((i, dist)),
                    _ => {}
                }
            }
        }

        closest.map(|(i, _)| i)
    }

    /// Focus camera on a body by index and start tracking it
    pub fn focus_on_body(&mut self, index: usize) {
        if let Some(body) = self.bodies.get(index) {
            self.camera.focus_on([body.x, body.y]); // Both are now f64
            self.tracked_body = Some(index);
        }
    }

    /// Update camera to follow tracked body using current positions
    pub fn update_tracking(&mut self, positions: &[[f64; 2]], scale: f64) {
        if let Some(index) = self.tracked_body {
            if let Some(pos) = positions.get(index) {
                // Set camera position directly to body position (in f64 for precision)
                self.camera.position[0] = pos[0] * scale;
                self.camera.position[1] = pos[1] * scale;
            }
        }
    }
}
