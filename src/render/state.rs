use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;
use egui_wgpu::ScreenDescriptor;

use crate::ship::{AutopilotTarget, RAILS_WARP_THRESHOLD};
use super::camera::Camera;
use super::textures::BodyTextureMap;
use super::types::{
    BodyData, BodyInfoData, MainMenuAction, ManeuverNode, OrbitRenderData, PauseAction,
    ShipOrbitData, ShipRenderData, TitleScreenAction, TrackingStationAction,
    TrackingVesselData, Vertex, HYPERBOLIC_RENDER_MARGIN, HYPERBOLIC_SKIP_MARGIN,
};

/// Format seconds into a human-readable duration string (e.g., "1d 2h 3m 4s")
fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "---".to_string();
    }
    let total = seconds as u64;
    let d = total / 86400;
    let h = (total % 86400) / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if d > 0 {
        format!("{}d {}h {}m {}s", d, h, m, s)
    } else if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

/// Format a distance in meters to a human-readable string with appropriate unit
fn format_distance(meters: f64) -> String {
    const AU: f64 = 1.496e11;
    if meters >= AU * 0.1 {
        format!("{:.3} AU", meters / AU)
    } else if meters >= 1e9 {
        format!("{:.1} Mm", meters / 1e6)
    } else if meters >= 1e6 {
        format!("{:.1} km", meters / 1e3)
    } else if meters >= 1e3 {
        format!("{:.1} km", meters / 1e3)
    } else {
        format!("{:.0} m", meters)
    }
}

/// Format mass in kg to a human-readable string with scientific notation
fn format_mass(kg: f64) -> String {
    if kg >= 1e24 {
        format!("{:.3e} kg", kg)
    } else if kg >= 1e18 {
        format!("{:.3e} kg", kg)
    } else if kg >= 1e6 {
        format!("{:.3e} kg", kg)
    } else {
        format!("{:.1} kg", kg)
    }
}

/// Format pressure in Pascals to a human-readable string
fn format_power_si(watts: f64) -> String {
    if watts >= 1e12 {
        format!("{:.1} TW", watts / 1e12)
    } else if watts >= 1e9 {
        format!("{:.1} GW", watts / 1e9)
    } else if watts >= 1e6 {
        format!("{:.1} MW", watts / 1e6)
    } else if watts >= 1e3 {
        format!("{:.1} kW", watts / 1e3)
    } else {
        format!("{:.0} W", watts)
    }
}

fn format_pressure(pa: f64) -> String {
    if pa >= 101_325.0 * 0.5 {
        format!("{:.2} atm", pa / 101_325.0)
    } else if pa >= 1000.0 {
        format!("{:.1} kPa", pa / 1000.0)
    } else {
        format!("{:.1} Pa", pa)
    }
}

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
    pub fps: f32,              // Frames per second (smoothed)
    pub ship_velocity: f64,    // Current velocity (m/s)
    pub ship_altitude: f64,    // Current altitude (m)
    pub ship_throttle: f64,    // Current throttle (0.0 to 1.0)
    pub ship_soi_name: String, // Current SOI body name
    pub ship_time_to_intercept: Option<f64>, // Time to next SOI transition (seconds)
    pub ship_acceleration: f64,            // Ship's max thrust acceleration (m/s^2)
    pub ship_current_true_anomaly: f64,    // Ship's current position in its orbit (radians)
    // Vessel stats for HUD
    pub vessel_total_mass: Option<f64>,    // tonnes
    pub vessel_fuel_fraction: Option<f64>, // 0.0-1.0
    pub vessel_power_generation: Option<f64>,  // Watts
    pub vessel_power_consumption: Option<f64>, // Watts
    pub vessel_electricity_fraction: Option<f64>, // 0.0-1.0
    pub vessel_electricity_stored: Option<f64>,   // Wh
    pub vessel_electricity_max: Option<f64>,     // Wh max capacity
    pub vessel_thrust_kn: Option<f64>,     // kN
    pub vessel_drag_kn: f64,               // kN, aerodynamic drag
    pub vessel_delta_v: Option<f64>,       // m/s
    pub vessel_current_stage: Option<usize>,  // Stages activated so far
    pub vessel_total_stages: Option<usize>,   // Total stages
    pub vessel_stages: Vec<Vec<super::types::StagedPartInfo>>,  // Full stage data for UI
    pub vessel_stage_delta_vs: Vec<f64>,  // Per-stage delta-v (m/s, vacuum)
    pub staging_reorder: Option<Vec<Vec<usize>>>,  // Request to reorder stages (part indices)
    pub ship_soi_surface_gravity: f64,     // m/s², for TWR
    pub ship_g_force: f64,                 // Felt acceleration in g's (thrust + drag, not gravity)
    // Thermal state
    pub ship_temperature: f64,            // Kelvin
    pub ship_heat_fraction: f32,          // 0.0-1.0, for visual effects
    pub ship_heat_flux: f64,              // W/m², for HUD display
    pub ship_below_landing_altitude: bool, // Whether warp > 10x should be blocked
    pub ship_velocity_direction: [f64; 2], // Normalized velocity unit vector for prograde arrow
    // Part click state
    pub selected_flight_part: Option<usize>,  // index into flight_parts_cache
    pub flight_parts_cache: Vec<super::types::ShipPartRenderData>,
    pub ship_render_x: f64,
    pub ship_render_y: f64,
    pub ship_body_center: [f64; 2],  // SOI body position in render units (large, galaxy-scale)
    pub ship_rel_offset: [f64; 2],   // Ship offset from SOI body in render units (small, local)
    pub ship_render_rotation: f64,
    pub ship_render_scale: f64,     // SCALE * BODY_SCALE used for rendering
    pub engine_toggle_request: Option<(usize, bool)>,  // (part_index, enabled)
    pub crossfeed_toggle_request: Option<(usize, bool)>,  // (part_index, crossfeed_enabled)
    pub decouple_request: Option<usize>,  // part_index to manually decouple
    pub fairing_deploy_request: Option<usize>,  // part_index to deploy fairing
    pub solar_deploy_request: Option<(usize, bool)>,  // (part_index, deploy)
    pub parachute_deploy_request: Option<usize>,  // part_index to deploy parachute
    pub ship_in_atmosphere: bool,  // Whether the active vessel is in atmosphere
    pub ship_is_landed: bool,      // Whether the active vessel is landed
    pub ap_markers: Vec<([f64; 2], f64)>, // Apoapsis markers: (world pos relative to camera, altitude)
    pub pe_markers: Vec<([f64; 2], f64)>, // Periapsis markers: (world pos relative to camera, altitude)
    pub closest_approach_world_pos: Option<([f64; 2], f64)>, // (render world pos, distance meters) - set by main.rs
    pub closest_approach_marker: Option<([f64; 2], f64)>, // (camera-relative pos, distance meters) - for egui hover
    // Simulation time (updated each frame from main.rs)
    pub simulation_time: f64,
    // Maneuver node state
    pub pending_orbit_click: Option<(f64, super::types::OrbitSegmentData)>,  // (true_anomaly, segment_data) - awaiting node creation
    pub selected_maneuver_node: Option<u64>,        // ID of selected node
    pub maneuver_nodes: Vec<ManeuverNode>,
    pub time_to_node: Option<f64>,      // seconds until first maneuver node
    pub burn_time: Option<f64>,         // estimated burn duration (seconds)
    pub warp_to_node: bool,             // auto-warp to node active
    pub next_node_id: u64,
    pub maneuver_node_screen_positions: Vec<(u64, [f32; 2])>, // (node_id, screen_pos) for click detection
    pub current_trajectory: Vec<super::types::OrbitSegmentData>, // Stored for click detection
    pub predicted_trajectories: Vec<Vec<super::types::OrbitSegmentData>>, // Predicted trajectories after maneuver burns (one per node)
    pub dragging_maneuver_node: Option<u64>, // ID of node being dragged
    // Background vessel screen positions for click detection
    pub background_vessel_screen_positions: Vec<(u64, [f32; 2])>,
    // Vessel the camera is following (tracking station)
    pub tracked_vessel: Option<u64>,
    // Autopilot state
    pub autopilot_target: AutopilotTarget,
    // Navigation target state
    pub selected_target: Option<super::types::SelectedTarget>,
    pub selected_target_name: String,
    pub selected_target_angle: Option<f64>,
    pub target_popup: Option<super::types::TargetPopup>,
    // RCS toggle (off by default)
    pub rcs_enabled: bool,
    // RCS was auto-disabled when entering on-rails warp; re-enable when returning to physics warp
    pub rcs_disabled_by_rails: bool,
    // Transfer planner state
    pub transfer_planner_open: bool,
    pub transfer_planner_mode: u8,              // 0 = Hohmann, 1 = Lambert
    pub transfer_selected_target: Option<usize>,
    pub transfer_departure_offset: f64,         // seconds, Lambert mode
    pub transfer_arrival_offset: f64,           // seconds, Lambert mode
    pub transfer_display: Option<crate::ship::transfer::TransferDisplay>,
    pub transfer_hohmann_targets: Vec<(usize, String)>,
    pub transfer_interplanetary_targets: Vec<(usize, String)>,
    pub transfer_node_request: Option<(f64, f64, f64, f64)>, // (position_angle, prograde_dv, radial_dv, time_to_window)
    // Quicksave UI state
    pub show_quicksave_list: bool,
    // Debug menu
    pub debug_menu_open: bool,
    pub debug_infinite_fuel: bool,
    pub debug_teleport_leo: bool,  // Request flag, consumed by main.rs
    // Body textures
    pub body_texture_bind_group: wgpu::BindGroup,
    pub body_texture_map: BodyTextureMap,
    // Sprite atlas
    pub sprite_atlas: super::sprites::SpriteAtlas,
    pub plume_start_time: std::time::Instant,
    // Egui state
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
}

impl RenderState {
    /// Create a new render state from a window
    pub async fn new(window: Arc<Window>, body_names: &[String]) -> Self {
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

        // Load body textures
        let (body_texture_view, body_sampler, body_texture_map) =
            super::textures::load_body_textures(&device, &queue, body_names);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let body_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&body_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&body_sampler),
                },
            ],
            label: Some("body_texture_bind_group"),
        });

        // Load sprite atlas
        let sprite_atlas = super::sprites::load_sprite_atlas(&device, &queue);
        let sprite_bind_group_layout = super::sprites::create_sprite_bind_group_layout(&device);
        let plume_start_time = std::time::Instant::now();

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create render pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout, &sprite_bind_group_layout],
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
            fps: 0.0,
            ship_velocity: 0.0,
            ship_altitude: 0.0,
            ship_throttle: 0.0,
            ship_soi_name: String::new(),
            ship_time_to_intercept: None,
            ship_acceleration: 20.0,  // Default max thrust acceleration
            ship_current_true_anomaly: 0.0,
            vessel_total_mass: None,
            vessel_fuel_fraction: None,
            vessel_power_generation: None,
            vessel_power_consumption: None,
            vessel_electricity_fraction: None,
            vessel_electricity_stored: None,
            vessel_electricity_max: None,
            vessel_thrust_kn: None,
            vessel_drag_kn: 0.0,
            vessel_delta_v: None,
            vessel_current_stage: None,
            vessel_total_stages: None,
            vessel_stages: Vec::new(),
            vessel_stage_delta_vs: Vec::new(),
            staging_reorder: None,
            ship_soi_surface_gravity: 9.81,
            ship_g_force: 0.0,
            ship_temperature: 300.0,
            ship_heat_fraction: 0.0,
            ship_heat_flux: 0.0,
            ship_below_landing_altitude: false,
            ship_velocity_direction: [0.0, 0.0],
            selected_flight_part: None,
            flight_parts_cache: Vec::new(),
            ship_render_x: 0.0,
            ship_render_y: 0.0,
            ship_body_center: [0.0, 0.0],
            ship_rel_offset: [0.0, 0.0],
            ship_render_rotation: 0.0,
            ship_render_scale: 1.0,
            engine_toggle_request: None,
            crossfeed_toggle_request: None,
            decouple_request: None,
            fairing_deploy_request: None,
            solar_deploy_request: None,
            parachute_deploy_request: None,
            ship_in_atmosphere: false,
            ship_is_landed: false,
            ap_markers: Vec::new(),
            pe_markers: Vec::new(),
            closest_approach_world_pos: None,
            closest_approach_marker: None,
            simulation_time: 0.0,
            pending_orbit_click: None,
            selected_maneuver_node: None,
            maneuver_nodes: Vec::new(),
            time_to_node: None,
            burn_time: None,
            warp_to_node: false,
            next_node_id: 1,
            maneuver_node_screen_positions: Vec::new(),
            current_trajectory: Vec::new(),
            predicted_trajectories: Vec::new(),
            dragging_maneuver_node: None,
            background_vessel_screen_positions: Vec::new(),
            tracked_vessel: None,
            autopilot_target: AutopilotTarget::Off,
            selected_target: None,
            selected_target_name: String::new(),
            selected_target_angle: None,
            target_popup: None,
            rcs_enabled: false,
            rcs_disabled_by_rails: false,
            transfer_planner_open: false,
            transfer_planner_mode: 0,
            transfer_selected_target: None,
            transfer_departure_offset: 0.0,
            transfer_arrival_offset: 0.0,
            transfer_display: None,
            transfer_hohmann_targets: Vec::new(),
            transfer_interplanetary_targets: Vec::new(),
            transfer_node_request: None,
            show_quicksave_list: false,
            debug_menu_open: false,
            debug_infinite_fuel: false,
            debug_teleport_leo: false,
            body_texture_bind_group,
            body_texture_map,
            sprite_atlas,
            plume_start_time,
            egui_ctx,
            egui_state,
            egui_renderer,
        }
    }

    // Note: create_circle and create_ship_triangle are in geometry.rs

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
    pub fn world_to_screen(&self, world_x: f64, world_y: f64) -> (f32, f32) {
        // Subtract in f64 to preserve precision at large distances, then cast to f32
        let rel_x = (world_x - self.camera.position[0]) as f32;
        let rel_y = (world_y - self.camera.position[1]) as f32;

        // Apply rotation
        let cos_r = self.camera.rotation.cos();
        let sin_r = self.camera.rotation.sin();
        let rotated_x = rel_x * cos_r - rel_y * sin_r;
        let rotated_y = rel_x * sin_r + rel_y * cos_r;

        // Apply zoom and aspect ratio correction
        let view_x = rotated_x * self.camera.zoom;
        let view_y = rotated_y * self.camera.zoom;
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
        paused: bool,
        date_str: &str,
        can_exit_flight: bool,
        can_recover: bool,
        quicksaves: &[crate::save::QuicksaveInfo],
    ) -> Result<(usize, PauseAction), wgpu::SurfaceError> {
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
        let camera_rotation = self.camera.rotation;
        let aspect_ratio = self.camera.aspect_ratio;
        let scale_factor = self.window.scale_factor() as f32;
        let fps = self.fps;
        let ship_orbit = self.ship_orbit_info.clone();
        let ship_velocity = self.ship_velocity;
        let ship_altitude = self.ship_altitude;
        let ship_throttle = self.ship_throttle;
        let ship_soi_name = self.ship_soi_name.clone();
        let ship_time_to_intercept = self.ship_time_to_intercept;
        let vessel_total_mass = self.vessel_total_mass;
        let vessel_fuel_fraction = self.vessel_fuel_fraction;
        let vessel_electricity_fraction = self.vessel_electricity_fraction;
        let vessel_electricity_stored = self.vessel_electricity_stored;
        let vessel_electricity_max = self.vessel_electricity_max;
        let vessel_power_generation = self.vessel_power_generation;
        let vessel_power_consumption = self.vessel_power_consumption;
        let vessel_thrust_kn = self.vessel_thrust_kn;
        let vessel_drag_kn = self.vessel_drag_kn;
        let vessel_delta_v = self.vessel_delta_v;
        let vessel_current_stage = self.vessel_current_stage;

        let ship_acceleration = self.ship_acceleration;
        let ship_below_landing_altitude = self.ship_below_landing_altitude;
        let ship_soi_surface_gravity = self.ship_soi_surface_gravity;
        let ship_g_force = self.ship_g_force;
        let ship_temperature = self.ship_temperature;
        let ship_heat_fraction = self.ship_heat_fraction;
        let selected_flight_part = self.selected_flight_part;
        let flight_parts_cache = self.flight_parts_cache.clone();
        let ap_markers = self.ap_markers.clone();
        let pe_markers = self.pe_markers.clone();
        let pending_orbit_click = self.pending_orbit_click.clone();
        let selected_maneuver_node = self.selected_maneuver_node;
        let maneuver_nodes = self.maneuver_nodes.clone();
        let time_to_node = self.time_to_node;
        let burn_time = self.burn_time;
        let warp_to_node_active = self.warp_to_node;
        let current_autopilot = self.autopilot_target;
        let vessel_stages = self.vessel_stages.clone();
        let vessel_stage_delta_vs = self.vessel_stage_delta_vs.clone();

        let mut new_warp_index = current_warp_index;
        let mut pause_action = PauseAction::None;
        let mut create_node_at: Option<(f64, super::types::OrbitSegmentData)> = None;
        let mut delete_node_id: Option<u64> = None;
        let mut close_maneuver_panel = false;
        let mut start_warp_to_node = false;
        let mut cancel_warp_to_node = false;
        let mut prograde_delta: f64 = 0.0;
        let mut radial_delta: f64 = 0.0;
        let mut new_autopilot_target = current_autopilot;
        let mut engine_toggle_req: Option<(usize, bool)> = None;
        let mut crossfeed_toggle_req: Option<(usize, bool)> = None;
        let mut decouple_req: Option<usize> = None;
        let mut fairing_deploy_req: Option<usize> = None;
        let mut solar_deploy_req: Option<(usize, bool)> = None;
        let mut parachute_deploy_req: Option<usize> = None;
        let ship_in_atmosphere = self.ship_in_atmosphere;
        let ship_is_landed = self.ship_is_landed;
        let mut staging_reorder_req: Option<Vec<Vec<usize>>> = None;

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
                        // Block selecting warp > max physics warp while actually producing thrust
                        let actually_thrusting = ship_throttle > 0.0 && ship_acceleration > 0.0;
                        let blocked_throttle = actually_thrusting && warp > RAILS_WARP_THRESHOLD;
                        // Block on-rails warps that would reach SOI boundary in < 0.5 seconds
                        // Physics warp (≤10x) handles SOI transitions via substeps, so only block on-rails
                        let blocked_intercept = warp > RAILS_WARP_THRESHOLD && ship_time_to_intercept
                            .map(|t| t / warp < 0.5)
                            .unwrap_or(false);
                        // Block on-rails warp when below landing altitude
                        let blocked_landing = ship_below_landing_altitude && warp > RAILS_WARP_THRESHOLD;
                        let blocked = blocked_throttle || blocked_intercept || blocked_landing;
                        let button = ui.add_enabled(!blocked, egui::SelectableLabel::new(is_selected, &label));
                        if button.clicked() && !blocked {
                            new_warp_index = i;
                        }
                    }

                    // Show current warp value
                    ui.separator();
                    let current_warp = warp_levels[current_warp_index];
                    ui.label(format!("Current: {}x", current_warp as i64));

                    ui.separator();
                    ui.label(date_str);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{:.0} fps", fps)).size(11.0).color(egui::Color32::GRAY));
                    });
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

            // Bottom panel for autopilot buttons and velocity/altitude display
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

                    // Autopilot buttons row
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);

                        // RCS toggle button
                        {
                            let rcs_btn_color = if self.rcs_enabled {
                                egui::Color32::from_rgb(80, 150, 80)
                            } else {
                                egui::Color32::from_rgb(60, 60, 70)
                            };
                            let rcs_text_color = if self.rcs_enabled {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::LIGHT_GRAY
                            };
                            let rcs_btn = egui::Button::new(egui::RichText::new("RCS").size(11.0).color(rcs_text_color))
                                .fill(rcs_btn_color)
                                .min_size(egui::vec2(35.0, 20.0));
                            if ui.add(rcs_btn).clicked() {
                                self.rcs_enabled = !self.rcs_enabled;
                            }
                        }

                        ui.add_space(5.0);
                        ui.label(egui::RichText::new("SAS").size(11.0).color(egui::Color32::GRAY));
                        ui.add_space(5.0);

                        // Helper to create autopilot button
                        let autopilot_btn = |ui: &mut egui::Ui, label: &str, target: AutopilotTarget, current: AutopilotTarget| -> bool {
                            let is_active = current == target;
                            let btn_color = if is_active {
                                egui::Color32::from_rgb(80, 150, 80)
                            } else {
                                egui::Color32::from_rgb(60, 60, 70)
                            };
                            let text_color = if is_active {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::LIGHT_GRAY
                            };
                            let btn = egui::Button::new(egui::RichText::new(label).size(11.0).color(text_color))
                                .fill(btn_color)
                                .min_size(egui::vec2(35.0, 20.0));
                            ui.add(btn).clicked()
                        };

                        // Prograde button
                        if autopilot_btn(ui, "PRO", AutopilotTarget::Prograde, new_autopilot_target) {
                            new_autopilot_target = if new_autopilot_target == AutopilotTarget::Prograde {
                                AutopilotTarget::Off
                            } else {
                                AutopilotTarget::Prograde
                            };
                        }

                        // Retrograde button
                        if autopilot_btn(ui, "RET", AutopilotTarget::Retrograde, new_autopilot_target) {
                            new_autopilot_target = if new_autopilot_target == AutopilotTarget::Retrograde {
                                AutopilotTarget::Off
                            } else {
                                AutopilotTarget::Retrograde
                            };
                        }

                        // Radial In button
                        if autopilot_btn(ui, "R-", AutopilotTarget::RadialIn, new_autopilot_target) {
                            new_autopilot_target = if new_autopilot_target == AutopilotTarget::RadialIn {
                                AutopilotTarget::Off
                            } else {
                                AutopilotTarget::RadialIn
                            };
                        }

                        // Radial Out button
                        if autopilot_btn(ui, "R+", AutopilotTarget::RadialOut, new_autopilot_target) {
                            new_autopilot_target = if new_autopilot_target == AutopilotTarget::RadialOut {
                                AutopilotTarget::Off
                            } else {
                                AutopilotTarget::RadialOut
                            };
                        }

                        // Maneuver node button (only if there's a selected node)
                        if selected_maneuver_node.is_some() {
                            if autopilot_btn(ui, "MAN", AutopilotTarget::ManeuverNode, new_autopilot_target) {
                                new_autopilot_target = if new_autopilot_target == AutopilotTarget::ManeuverNode {
                                    AutopilotTarget::Off
                                } else {
                                    AutopilotTarget::ManeuverNode
                                };
                            }
                        }

                        // Target button (only if a target is selected)
                        if self.selected_target.is_some() {
                            if autopilot_btn(ui, "TGT", AutopilotTarget::Target, new_autopilot_target) {
                                new_autopilot_target = if new_autopilot_target == AutopilotTarget::Target {
                                    AutopilotTarget::Off
                                } else {
                                    AutopilotTarget::Target
                                };
                            }
                        }

                        // Target name display
                        if self.selected_target.is_some() {
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(format!("→ {}", self.selected_target_name))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(130, 190, 255)));
                            // Clear target button
                            let x_btn = egui::Button::new(egui::RichText::new("x").size(10.0).color(egui::Color32::GRAY))
                                .fill(egui::Color32::TRANSPARENT)
                                .min_size(egui::vec2(16.0, 16.0));
                            if ui.add(x_btn).clicked() {
                                self.selected_target = None;
                                self.selected_target_name.clear();
                                self.selected_target_angle = None;
                                if new_autopilot_target == AutopilotTarget::Target {
                                    new_autopilot_target = AutopilotTarget::Off;
                                }
                            }
                        }

                        // Vessel stats (if vessel loaded)
                        if let Some(mass) = vessel_total_mass {
                            ui.separator();
                            ui.label(egui::RichText::new("M").size(11.0).color(egui::Color32::GRAY));
                            ui.label(egui::RichText::new(format!("{:.2}t", mass)).size(11.0).color(egui::Color32::WHITE));
                        }
                        if let Some(thrust) = vessel_thrust_kn {
                            ui.label(egui::RichText::new("T").size(11.0).color(egui::Color32::GRAY));
                            ui.label(egui::RichText::new(format!("{:.0}kN", thrust)).size(11.0).color(egui::Color32::WHITE));

                            if vessel_drag_kn > 0.01 {
                                ui.label(egui::RichText::new("D").size(11.0).color(egui::Color32::GRAY));
                                ui.label(egui::RichText::new(format!("{:.1}kN", vessel_drag_kn)).size(11.0).color(egui::Color32::WHITE));
                            }

                            // TWR display
                            if let Some(mass) = vessel_total_mass {
                                if mass > 0.0 && ship_soi_surface_gravity > 0.0 {
                                    let twr = thrust / (mass * ship_soi_surface_gravity);
                                    let twr_color = if twr >= 1.0 {
                                        egui::Color32::from_rgb(100, 220, 100)
                                    } else {
                                        egui::Color32::from_rgb(220, 80, 80)
                                    };
                                    ui.label(egui::RichText::new("TWR").size(11.0).color(egui::Color32::GRAY));
                                    ui.label(egui::RichText::new(format!("{:.2}", twr)).size(11.0).color(twr_color));
                                }
                            }
                        }

                        // G-force display
                        {
                            let g_color = if ship_g_force < 3.0 {
                                egui::Color32::WHITE
                            } else if ship_g_force < 6.0 {
                                egui::Color32::from_rgb(220, 200, 80)
                            } else {
                                egui::Color32::from_rgb(220, 80, 80)
                            };
                            ui.label(egui::RichText::new("G").size(11.0).color(egui::Color32::GRAY));
                            ui.label(egui::RichText::new(format!("{:.1}", ship_g_force)).size(11.0).color(g_color));
                        }
                        if let Some(dv) = vessel_delta_v {
                            ui.label(egui::RichText::new("Δv").size(11.0).color(egui::Color32::GRAY));
                            let dv_str = if dv >= 1000.0 {
                                format!("{:.1}km/s", dv / 1000.0)
                            } else {
                                format!("{:.0}m/s", dv)
                            };
                            ui.label(egui::RichText::new(&dv_str).size(11.0).color(egui::Color32::WHITE));
                        }

                        // Velocity and altitude display (right side)
                        let remaining = ui.available_width();
                        ui.add_space((remaining / 2.0 - 100.0).max(10.0));
                        ui.label(egui::RichText::new("VEL").size(11.0).color(egui::Color32::GRAY));
                        ui.label(egui::RichText::new(&vel_str).size(13.0).strong().color(egui::Color32::WHITE));
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("ALT").size(11.0).color(egui::Color32::GRAY));
                        ui.label(egui::RichText::new(&alt_str).size(13.0).strong().color(egui::Color32::WHITE));



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
                        let rel_x = (body.x - camera_pos[0]) as f32;
                        let rel_y = (body.y - camera_pos[1]) as f32;
                        let cos_r = camera_rotation.cos();
                        let sin_r = camera_rotation.sin();
                        let rot_x = rel_x * cos_r - rel_y * sin_r;
                        let rot_y = rel_x * sin_r + rel_y * cos_r;
                        let view_x = rot_x * camera_zoom;
                        let view_y = rot_y * camera_zoom;
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
                let rx = world_rel[0] as f32;
                let ry = world_rel[1] as f32;
                let cos_r = camera_rotation.cos();
                let sin_r = camera_rotation.sin();
                let rot_x = rx * cos_r - ry * sin_r;
                let rot_y = rx * sin_r + ry * cos_r;
                let view_x = rot_x * camera_zoom;
                let view_y = rot_y * camera_zoom;
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

            // Draw closest approach marker label
            if let Some((pos, distance)) = &self.closest_approach_marker {
                let (screen_x, screen_y) = world_to_screen(*pos);
                let marker_screen_pos = egui::pos2(screen_x, screen_y);

                let is_hovered = mouse_pos.map_or(false, |mp| {
                    (mp - marker_screen_pos).length() < hover_radius
                });

                marker_painter.text(
                    egui::pos2(screen_x, screen_y - 12.0),
                    egui::Align2::CENTER_BOTTOM,
                    "CA",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_rgb(255, 255, 0), // Yellow
                );

                if is_hovered {
                    marker_painter.text(
                        egui::pos2(screen_x, screen_y + 14.0),
                        egui::Align2::CENTER_TOP,
                        &format!("Closest Approach: {}", format_altitude(*distance)),
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_rgb(255, 255, 0),
                    );
                }
            }

            // Maneuver nodes calculate world position from stored orbit + current parent position
            let node_world_pos = |node: &super::types::ManeuverNode| -> Option<[f64; 2]> {
                let parent = bodies_copy.get(node.parent_idx)?;
                Some(node.world_pos(parent.x, parent.y))
            };

            // Draw "Create Maneuver Node" button if pending click
            if let Some((ta, ref segment)) = pending_orbit_click {
                let e = segment.eccentricity;
                let arg_peri = segment.argument_of_periapsis;

                let r = if e >= 1.0 {
                    let a_abs = segment.semi_major_axis.abs();
                    let p = a_abs * (e * e - 1.0);
                    let denom = 1.0 + e * ta.cos();
                    if denom > 0.001 { p / denom } else { 0.0 }
                } else {
                    let a = segment.semi_major_axis;
                    let p = a * (1.0 - e * e);
                    p / (1.0 + e * ta.cos())
                };

                if r > 0.0 && r.is_finite() {
                    let angle = ta + arg_peri;
                    let world_x = segment.parent_x + r * angle.cos();
                    let world_y = segment.parent_y + r * angle.sin();

                    let (scr_x, scr_y) = world_to_screen([world_x - camera_pos[0], world_y - camera_pos[1]]);

                    // Draw a small window with the create button
                    egui::Area::new(egui::Id::new("create_node_popup"))
                        .fixed_pos(egui::pos2(scr_x + 15.0, scr_y - 15.0))
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                if ui.button("Create Maneuver Node").clicked() {
                                    create_node_at = Some((ta, segment.clone()));
                                }
                            });
                        });
                }
            }

            // Draw "Select as Target" popup if a body/vessel was single-clicked
            if let Some(ref popup) = self.target_popup {
                let popup_target = popup.target;
                let popup_name = popup.name.clone();

                // Compute screen position dynamically from the target's world position
                // (world_to_screen inlined to avoid borrow conflict with egui closure)
                // Output in egui logical points (divided by scale_factor)
                let cam_x = self.camera.position[0];
                let cam_y = self.camera.position[1];
                let cam_zoom = self.camera.zoom;
                let cam_rot = self.camera.rotation;
                let cam_aspect = self.camera.aspect_ratio;
                let scr_w = self.size.width as f32;
                let scr_h = self.size.height as f32;
                let w2s = |wx: f64, wy: f64| -> (f32, f32) {
                    let rel_x = (wx - cam_x) as f32;
                    let rel_y = (wy - cam_y) as f32;
                    let cos_r = cam_rot.cos();
                    let sin_r = cam_rot.sin();
                    let rx = rel_x * cos_r - rel_y * sin_r;
                    let ry = rel_x * sin_r + rel_y * cos_r;
                    let nx = rx * cam_zoom / cam_aspect;
                    let ny = ry * cam_zoom;
                    (
                        (nx + 1.0) * 0.5 * scr_w / scale_factor,
                        (1.0 - ny) * 0.5 * scr_h / scale_factor,
                    )
                };
                let points_per_world_unit = cam_zoom * scr_h / 2.0 / scale_factor;

                let screen_pos = match popup.target {
                    super::types::SelectedTarget::Body(idx) => {
                        if let Some(body) = self.bodies.get(idx) {
                            let (sx, sy) = w2s(body.x, body.y);
                            let visual_radius = if body.indicator_radius > 0.0 {
                                body.indicator_radius as f32 * points_per_world_unit
                            } else {
                                body.radius as f32 * points_per_world_unit
                            };
                            let offset_below = visual_radius.max(8.0) + 6.0;
                            Some((sx, sy + offset_below))
                        } else {
                            None
                        }
                    }
                    super::types::SelectedTarget::Vessel(id) => {
                        self.background_vessel_screen_positions.iter()
                            .find(|&&(vid, _)| vid == id)
                            .map(|&(_, pos)| (pos[0] / scale_factor, pos[1] / scale_factor + 14.0))
                    }
                };

                if let Some((popup_x, popup_y)) = screen_pos {
                    egui::Area::new(egui::Id::new("target_select_popup"))
                        .fixed_pos(egui::pos2(popup_x, popup_y))
                        .pivot(egui::Align2::CENTER_TOP)
                        .show(ctx, |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.label(egui::RichText::new(&popup_name).strong());
                                if ui.button("Set as Target").clicked() {
                                    self.selected_target = Some(popup_target);
                                    self.selected_target_name = popup_name.clone();
                                    self.target_popup = None;
                                }
                            });
                        });
                } else {
                    // Target not visible on screen, clear popup
                    self.target_popup = None;
                }
            }

            // Right panel - Staging (always visible when vessel has stages)
            if !vessel_stages.is_empty() {
                egui::SidePanel::right("flight_staging_panel")
                    .default_width(150.0)
                    .show(ctx, |ui| {
                        ui.heading("Staging");
                        ui.separator();

                        // Total Δv
                        let total_dv: f64 = vessel_stage_delta_vs.iter().sum();
                        if total_dv > 0.0 {
                            let dv_str = if total_dv >= 1000.0 {
                                format!("{:.1} km/s", total_dv / 1000.0)
                            } else {
                                format!("{:.0} m/s", total_dv)
                            };
                            ui.label(egui::RichText::new(format!("Total Δv: {}", dv_str))
                                .size(12.0).strong());
                        }

                        if let Some(current) = vessel_current_stage {
                            ui.label(egui::RichText::new(format!("Active: {}/{}", current, vessel_stages.len()))
                                .size(11.0).color(egui::Color32::GRAY));
                        }
                        ui.separator();

                        // Drag payload: either a part or a whole stage
                        #[derive(Clone, Copy)]
                        enum FlightStageDrag {
                            Part(usize),   // part_index
                            Stage(usize),  // stage_index
                        }

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut insert_stage_at: Option<usize> = None;
                            let mut move_stage_to: Option<(usize, usize)> = None;
                            let mut delete_stage_at: Option<usize> = None;
                            let mut drop_action: Option<(FlightStageDrag, usize)> = None;

                            // Helper: "+" gap that doubles as a drop zone for stage reordering
                            let plus_gap = |ui: &mut egui::Ui, insert_pos: usize,
                                                 insert_out: &mut Option<usize>,
                                                 move_out: &mut Option<(usize, usize)>| {
                                let frame = egui::Frame::none();
                                let (inner, dropped) = ui.dnd_drop_zone::<FlightStageDrag, ()>(frame, |ui| {
                                    if ui.small_button("+").on_hover_text("Insert stage here").clicked() {
                                        *insert_out = Some(insert_pos);
                                    }
                                });
                                if inner.response.hovered() && egui::DragAndDrop::has_any_payload(ui.ctx()) {
                                    inner.response.highlight();
                                }
                                if let Some(payload) = dropped {
                                    if let FlightStageDrag::Stage(from_idx) = *payload {
                                        *move_out = Some((from_idx, insert_pos));
                                    }
                                }
                            };

                            for stage_idx in (0..vessel_stages.len()).rev() {
                                plus_gap(ui, stage_idx + 1, &mut insert_stage_at, &mut move_stage_to);

                                let frame = egui::Frame::group(ui.style());
                                let (_, dropped_payload) = ui.dnd_drop_zone::<FlightStageDrag, ()>(frame, |ui| {
                                    ui.horizontal(|ui| {
                                        let activated = vessel_current_stage.map_or(false, |c| stage_idx < c);
                                        let label_color = if activated {
                                            egui::Color32::DARK_GRAY
                                        } else {
                                            egui::Color32::WHITE
                                        };
                                        let stage_drag_id = egui::Id::new(("flight_staging_stage", stage_idx));
                                        ui.dnd_drag_source(stage_drag_id, FlightStageDrag::Stage(stage_idx), |ui| {
                                            ui.label(egui::RichText::new(format!("Stage {}", stage_idx + 1)).color(label_color));
                                        });
                                        // Per-stage Δv
                                        let stage_dv = vessel_stage_delta_vs.get(stage_idx).copied().unwrap_or(0.0);
                                        if stage_dv > 0.0 {
                                            let dv_str = if stage_dv >= 1000.0 {
                                                format!("{:.1}km/s", stage_dv / 1000.0)
                                            } else {
                                                format!("{:.0}m/s", stage_dv)
                                            };
                                            ui.label(egui::RichText::new(dv_str)
                                                .size(10.0).color(egui::Color32::from_rgb(120, 200, 120)));
                                        }
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button("\u{2715}").on_hover_text("Delete stage").clicked() {
                                                delete_stage_at = Some(stage_idx);
                                            }
                                        });
                                    });

                                    if vessel_stages[stage_idx].is_empty() {
                                        ui.weak("(empty)");
                                    }

                                    let selected_vessel_part = selected_flight_part
                                        .and_then(|ci| flight_parts_cache.get(ci))
                                        .map(|p| p.part_index);

                                    for part_info in &vessel_stages[stage_idx] {
                                        let item_id = egui::Id::new(("flight_staging_item", part_info.part_index));
                                        let is_selected = selected_vessel_part == Some(part_info.part_index);
                                        ui.dnd_drag_source(item_id, FlightStageDrag::Part(part_info.part_index), |ui| {
                                            let text = egui::RichText::new(&part_info.name);
                                            let text = if is_selected {
                                                text.color(egui::Color32::from_rgb(128, 179, 255))
                                            } else {
                                                text
                                            };
                                            ui.label(text);
                                        });
                                    }
                                });

                                if let Some(payload) = dropped_payload {
                                    drop_action = Some((*payload, stage_idx));
                                }
                            }

                            plus_gap(ui, 0, &mut insert_stage_at, &mut move_stage_to);

                            // Build new stages if any action occurred
                            if move_stage_to.is_some() || insert_stage_at.is_some() || delete_stage_at.is_some() || drop_action.is_some() {
                                let mut new_stages: Vec<Vec<usize>> = vessel_stages.iter()
                                    .map(|stage| stage.iter().map(|p| p.part_index).collect())
                                    .collect();

                                if let Some((from_idx, to_pos)) = move_stage_to {
                                    if from_idx < new_stages.len() {
                                        let stage = new_stages.remove(from_idx);
                                        let insert_at = if from_idx < to_pos {
                                            (to_pos - 1).min(new_stages.len())
                                        } else {
                                            to_pos.min(new_stages.len())
                                        };
                                        new_stages.insert(insert_at, stage);
                                    }
                                } else if let Some(idx) = insert_stage_at {
                                    new_stages.insert(idx, Vec::new());
                                } else if let Some(idx) = delete_stage_at {
                                    if idx < new_stages.len() {
                                        new_stages.remove(idx);
                                    }
                                } else if let Some((drag, target_idx)) = drop_action {
                                    match drag {
                                        FlightStageDrag::Part(part_idx) => {
                                            for stage in &mut new_stages {
                                                stage.retain(|&idx| idx != part_idx);
                                            }
                                            if target_idx < new_stages.len() {
                                                new_stages[target_idx].push(part_idx);
                                            }
                                        }
                                        FlightStageDrag::Stage(from_idx) => {
                                            if from_idx != target_idx && from_idx < new_stages.len() {
                                                let stage = new_stages.remove(from_idx);
                                                let insert_at = if from_idx < target_idx {
                                                    (target_idx - 1).min(new_stages.len())
                                                } else {
                                                    target_idx.min(new_stages.len())
                                                };
                                                new_stages.insert(insert_at, stage);
                                            }
                                        }
                                    }
                                }

                                staging_reorder_req = Some(new_stages);
                            }
                        });
                    });
            }

            // Throttle bar on right side (left of staging panel)
            egui::SidePanel::right("throttle_panel")
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

            // Left panel - fuel, electricity, heat, stage, XFER, debug
            egui::SidePanel::left("status_panel")
                .exact_width(50.0)
                .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(20, 20, 30, 200)))
                .show(ctx, |ui| {
                    // Fuel bar (if vessel loaded)
                    if let Some(fuel_frac) = vessel_fuel_fraction {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("FUEL").size(10.0).color(egui::Color32::GRAY));
                        ui.add_space(3.0);

                        let fuel_pct = (fuel_frac * 100.0) as i32;
                        ui.label(egui::RichText::new(format!("{}%", fuel_pct))
                            .size(11.0)
                            .color(egui::Color32::WHITE));
                        ui.add_space(3.0);

                        let fuel_bar_height = 80.0;
                        let bar_width = 20.0;
                        let (fuel_rect, _) = ui.allocate_exact_size(
                            egui::vec2(bar_width, fuel_bar_height),
                            egui::Sense::hover()
                        );

                        let fuel_painter = ui.painter();
                        fuel_painter.rect_filled(fuel_rect, 2.0, egui::Color32::from_rgb(40, 40, 50));

                        let fuel_fill = fuel_bar_height * fuel_frac as f32;
                        let fuel_fill_rect = egui::Rect::from_min_size(
                            egui::pos2(fuel_rect.min.x, fuel_rect.max.y - fuel_fill),
                            egui::vec2(bar_width, fuel_fill)
                        );
                        let fuel_color = if fuel_frac > 0.3 {
                            egui::Color32::from_rgb(80, 160, 220)
                        } else if fuel_frac > 0.1 {
                            egui::Color32::from_rgb(220, 180, 80)
                        } else {
                            egui::Color32::from_rgb(220, 80, 80)
                        };
                        fuel_painter.rect_filled(fuel_fill_rect, 2.0, fuel_color);
                        fuel_painter.rect_stroke(fuel_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
                    }

                    // Electricity bar (if vessel has batteries)
                    if let Some(elec_frac) = vessel_electricity_fraction {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("ELEC").size(10.0).color(egui::Color32::GRAY));
                        ui.add_space(3.0);

                        let stored = vessel_electricity_stored.unwrap_or(0.0);
                        let max = vessel_electricity_max.unwrap_or(0.0);
                        let fmt_wh = |v: f64| -> String {
                            if v >= 1000.0 { format!("{:.1}k", v / 1000.0) } else { format!("{:.0}", v) }
                        };
                        ui.label(egui::RichText::new(format!("{} / {} Wh", fmt_wh(stored), fmt_wh(max)))
                            .size(11.0)
                            .color(egui::Color32::WHITE));
                        ui.add_space(3.0);

                        let elec_bar_height = 80.0;
                        let bar_width = 20.0;
                        let (elec_rect, _) = ui.allocate_exact_size(
                            egui::vec2(bar_width, elec_bar_height),
                            egui::Sense::hover()
                        );

                        let elec_painter = ui.painter();
                        elec_painter.rect_filled(elec_rect, 2.0, egui::Color32::from_rgb(40, 40, 50));

                        let elec_fill = elec_bar_height * elec_frac as f32;
                        let elec_fill_rect = egui::Rect::from_min_size(
                            egui::pos2(elec_rect.min.x, elec_rect.max.y - elec_fill),
                            egui::vec2(bar_width, elec_fill)
                        );
                        let elec_color = if elec_frac > 0.3 {
                            egui::Color32::from_rgb(200, 190, 60)  // gold/yellow
                        } else if elec_frac > 0.1 {
                            egui::Color32::from_rgb(220, 140, 40)  // orange
                        } else {
                            egui::Color32::from_rgb(220, 60, 60)   // red
                        };
                        elec_painter.rect_filled(elec_fill_rect, 2.0, elec_color);
                        elec_painter.rect_stroke(elec_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY));

                        // Power generation/consumption text
                        if let (Some(gen), Some(cons)) = (vessel_power_generation, vessel_power_consumption) {
                            ui.add_space(3.0);
                            let net_text = format!("+{:.0}W\n-{:.0}W", gen, cons);
                            ui.label(egui::RichText::new(net_text).size(9.0).color(egui::Color32::GRAY));

                            // Power duration when draining
                            let net = gen - cons;
                            if net < 0.0 && stored > 0.0 {
                                let seconds = (stored / net.abs()) * 3600.0;
                                ui.label(egui::RichText::new(format_duration(seconds))
                                    .size(9.0)
                                    .color(egui::Color32::from_rgb(220, 180, 40)));
                            }
                        }
                    }

                    // Heat bar (shown when temperature > 350K)
                    if ship_temperature > 350.0 {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("HEAT").size(10.0).color(egui::Color32::GRAY));
                        ui.add_space(3.0);

                        ui.label(egui::RichText::new(format!("{}K", ship_temperature as i32))
                            .size(11.0)
                            .color(egui::Color32::WHITE));
                        ui.add_space(3.0);

                        let heat_bar_height = 80.0;
                        let bar_width = 20.0;
                        let (heat_rect, _) = ui.allocate_exact_size(
                            egui::vec2(bar_width, heat_bar_height),
                            egui::Sense::hover()
                        );

                        let heat_painter = ui.painter();
                        heat_painter.rect_filled(heat_rect, 2.0, egui::Color32::from_rgb(40, 40, 50));

                        let heat_fill = heat_bar_height * ship_heat_fraction;
                        let heat_fill_rect = egui::Rect::from_min_size(
                            egui::pos2(heat_rect.min.x, heat_rect.max.y - heat_fill),
                            egui::vec2(bar_width, heat_fill)
                        );
                        let heat_color = if ship_heat_fraction < 0.33 {
                            egui::Color32::from_rgb(220, 200, 80)  // yellow
                        } else if ship_heat_fraction < 0.66 {
                            egui::Color32::from_rgb(220, 140, 40)  // orange
                        } else {
                            egui::Color32::from_rgb(220, 60, 60)   // red
                        };
                        heat_painter.rect_filled(heat_fill_rect, 2.0, heat_color);
                        heat_painter.rect_stroke(heat_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY));

                        // Show the critical part (highest heat_fraction)
                        if let Some(hottest) = flight_parts_cache.iter()
                            .max_by(|a, b| a.heat_fraction.partial_cmp(&b.heat_fraction).unwrap_or(std::cmp::Ordering::Equal))
                        {
                            if hottest.heat_fraction > 0.01 {
                                ui.add_space(5.0);
                                let crit_color = if hottest.heat_fraction < 0.33 {
                                    egui::Color32::from_rgb(220, 200, 80)
                                } else if hottest.heat_fraction < 0.66 {
                                    egui::Color32::from_rgb(220, 140, 40)
                                } else {
                                    egui::Color32::from_rgb(220, 60, 60)
                                };
                                // Truncate name to fit the narrow panel
                                let name = if hottest.name.len() > 8 {
                                    &hottest.name[..8]
                                } else {
                                    &hottest.name
                                };
                                ui.label(egui::RichText::new(name)
                                    .size(8.0).color(crit_color));
                            }
                        }
                    }


                    // XFER button anchored at bottom
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        let xfer_active = self.transfer_planner_open;
                        let xfer_btn_color = if xfer_active {
                            egui::Color32::from_rgb(80, 120, 180)
                        } else {
                            egui::Color32::from_rgb(60, 60, 70)
                        };
                        let xfer_text_color = if xfer_active {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::LIGHT_GRAY
                        };

                        // Ellipse icon + XFER label as button
                        let btn_size = egui::vec2(40.0, 32.0);
                        let (rect, response) = ui.allocate_exact_size(btn_size, egui::Sense::click());

                        // Draw button background
                        let painter = ui.painter();
                        painter.rect_filled(rect, 4.0, xfer_btn_color);

                        // Draw ellipse icon (orbit shape) using line segments
                        let cx = rect.center().x;
                        let cy = rect.center().y - 4.0;
                        let rx = 10.0_f32;
                        let ry = 6.0_f32;
                        let n = 24;
                        let ellipse_points: Vec<egui::Pos2> = (0..=n).map(|i| {
                            let angle = std::f32::consts::TAU * i as f32 / n as f32;
                            egui::pos2(cx + rx * angle.cos(), cy + ry * angle.sin())
                        }).collect();
                        painter.add(egui::Shape::line(ellipse_points, egui::Stroke::new(1.5, xfer_text_color)));

                        // Draw "XFER" label below icon
                        painter.text(
                            egui::pos2(rect.center().x, rect.max.y - 6.0),
                            egui::Align2::CENTER_CENTER,
                            "XFER",
                            egui::FontId::proportional(8.0),
                            xfer_text_color,
                        );

                        if response.clicked() {
                            self.transfer_planner_open = !self.transfer_planner_open;
                        }

                        // Debug menu button (above XFER)
                        ui.add_space(5.0);
                        let dbg_color = if self.debug_menu_open {
                            egui::Color32::from_rgb(180, 80, 80)
                        } else {
                            egui::Color32::from_rgb(60, 60, 70)
                        };
                        let dbg_btn = egui::Button::new(
                            egui::RichText::new("DBG").size(9.0).color(egui::Color32::LIGHT_GRAY)
                        ).fill(dbg_color).min_size(egui::vec2(40.0, 20.0));
                        if ui.add(dbg_btn).clicked() {
                            self.debug_menu_open = !self.debug_menu_open;
                        }
                    });
                });

            // Right panel for selected maneuver node
            if let Some(node_id) = selected_maneuver_node {
                if let Some(node) = maneuver_nodes.iter().find(|n| n.id == node_id) {
                    let remaining_dv = node.total_remaining_delta_v();

                    egui::SidePanel::right("maneuver_panel")
                        .exact_width(200.0)
                        .show(ctx, |ui| {
                            ui.heading("Maneuver Node");
                            ui.separator();

                            // Remaining delta-v display
                            ui.label(format!("Remaining Δv: {:.1} m/s", remaining_dv));

                            // Time-to-node and burn time
                            if let Some(ttn) = time_to_node {
                                ui.label(format!("T- {}", format_duration(ttn)));
                                if let Some(bt) = burn_time {
                                    if bt > 0.5 {
                                        ui.label(format!("Burn: {}", format_duration(bt)));
                                    }
                                }
                                ui.add_space(4.0);
                                if warp_to_node_active {
                                    if ui.button("Cancel Warp").clicked() {
                                        cancel_warp_to_node = true;
                                        new_warp_index = 0;
                                    }
                                } else if ttn > 10.0 {
                                    if ui.button("Warp to Node").clicked() {
                                        start_warp_to_node = true;
                                    }
                                }
                            }
                            ui.separator();

                            // Prograde/Retrograde slider (show remaining values)
                            ui.label("Prograde / Retrograde:");
                            ui.horizontal(|ui| {
                                ui.label(format!("{:+.1} m/s", node.remaining_delta_v.prograde));
                            });

                            // Snap-back slider for prograde
                            let prograde_response = ui.add(
                                egui::Slider::new(&mut prograde_delta, -100.0..=100.0)
                                    .show_value(false)
                                    .text("")
                            );
                            if prograde_response.drag_stopped() {
                                prograde_delta = 0.0;
                            }

                            ui.add_space(10.0);

                            // Radial slider (show remaining values)
                            ui.label("Radial Out / In:");
                            ui.horizontal(|ui| {
                                ui.label(format!("{:+.1} m/s", node.remaining_delta_v.radial_out));
                            });

                            // Snap-back slider for radial
                            let radial_response = ui.add(
                                egui::Slider::new(&mut radial_delta, -100.0..=100.0)
                                    .show_value(false)
                                    .text("")
                            );
                            if radial_response.drag_stopped() {
                                radial_delta = 0.0;
                            }

                            ui.add_space(10.0);
                            ui.separator();

                            // Delete and close buttons
                            ui.horizontal(|ui| {
                                if ui.button("Delete").clicked() {
                                    delete_node_id = Some(node_id);
                                }
                                if ui.button("Close").clicked() {
                                    close_maneuver_panel = true;
                                }
                            });
                        });
                }
            }

            // Part info popup
            if let Some(cache_idx) = selected_flight_part {
                if let Some(part) = flight_parts_cache.get(cache_idx) {
                    egui::Window::new(&part.name)
                        .id(egui::Id::new("flight_part_info"))
                        .collapsible(false)
                        .resizable(false)
                        .default_width(200.0)
                        .show(ctx, |ui| {
                            ui.label(format!("Mass: {:.3} t", part.dry_mass));

                            // Engine info
                            if let Some(thrust_vac) = part.engine_thrust_vac {
                                ui.separator();
                                ui.label(egui::RichText::new("Engine").strong());
                                ui.label(format!("Thrust (vac): {:.1} kN", thrust_vac));
                                if let Some(thrust_asl) = part.engine_thrust_asl {
                                    ui.label(format!("Thrust (ASL): {:.1} kN", thrust_asl));
                                }
                                if let Some(isp_vac) = part.engine_isp_vac {
                                    ui.label(format!("ISP (vac): {:.0} s", isp_vac));
                                }
                                if let Some(isp_asl) = part.engine_isp_asl {
                                    ui.label(format!("ISP (ASL): {:.0} s", isp_asl));
                                }
                                if let Some(ref prop) = part.propellant_name {
                                    ui.label(format!("Propellant: {}", prop));
                                }
                                let status_text = if part.engine_enabled {
                                    if part.engine_active {
                                        egui::RichText::new("Active").color(egui::Color32::from_rgb(100, 220, 100))
                                    } else {
                                        egui::RichText::new("No Fuel").color(egui::Color32::from_rgb(220, 180, 80))
                                    }
                                } else {
                                    egui::RichText::new("Disabled").color(egui::Color32::from_rgb(220, 80, 80))
                                };
                                ui.label(status_text);

                                let btn_label = if part.engine_enabled { "Deactivate" } else { "Activate" };
                                if ui.button(btn_label).clicked() {
                                    engine_toggle_req = Some((part.part_index, !part.engine_enabled));
                                }
                            }

                            // Tank info
                            if let Some(ref fuel_name) = part.fuel_type_name {
                                ui.separator();
                                ui.label(egui::RichText::new("Fuel Tank").strong());
                                ui.label(format!("Type: {}", fuel_name));
                                // Oxidizer bar
                                if let (Some(current), Some(max)) = (part.ox_current, part.ox_max) {
                                    if max > 0.0 {
                                        let frac = (current / max) as f32;
                                        let bar = egui::ProgressBar::new(frac)
                                            .text(format!("O2: {:.0}/{:.0} kg", current, max))
                                            .fill(egui::Color32::from_rgb(80, 140, 200));
                                        ui.add(bar);
                                    }
                                }
                                // Fuel bar
                                if let (Some(current), Some(max)) = (part.fuel_current, part.fuel_max) {
                                    if max > 0.0 {
                                        let frac = (current / max) as f32;
                                        let fuel_label = fuel_name.split('/').last().unwrap_or("Fuel");
                                        let bar = egui::ProgressBar::new(frac)
                                            .text(format!("{}: {:.0}/{:.0} kg", fuel_label, current, max))
                                            .fill(egui::Color32::from_rgb(200, 160, 60));
                                        ui.add(bar);
                                    }
                                }
                            }

                            // Pod info
                            if let Some(crew) = part.crew_capacity {
                                ui.separator();
                                ui.label(egui::RichText::new("Command Pod").strong());
                                ui.label(format!("Crew capacity: {}", crew));

                                // Monopropellant bar
                                if let (Some(current), Some(max)) = (part.monoprop_current, part.monoprop_max) {
                                    if max > 0.0 {
                                        let frac = (current / max) as f32;
                                        let bar = egui::ProgressBar::new(frac)
                                            .text(format!("Monoprop: {:.1}/{:.0} kg", current, max))
                                            .fill(egui::Color32::from_rgb(180, 200, 80));
                                        ui.add(bar);
                                    }
                                }
                            }

                            // Battery info
                            if let (Some(current), Some(max)) = (part.battery_current, part.battery_max) {
                                ui.separator();
                                ui.label(egui::RichText::new("Battery").strong());
                                ui.label(format!("Capacity: {:.0} Wh", max));
                                if max > 0.0 {
                                    let frac = (current / max) as f32;
                                    let bar = egui::ProgressBar::new(frac)
                                        .text(format!("{:.0} / {:.0} Wh", current, max))
                                        .fill(egui::Color32::from_rgb(200, 190, 60));
                                    ui.add(bar);
                                }
                            }

                            // Solar panel info
                            if let Some(output) = part.solar_output {
                                ui.separator();
                                ui.label(egui::RichText::new("Solar Panel").strong());
                                ui.label(format!("Output: {:.0} W", output));
                                let label = if part.deploy_fraction >= 0.5 { "Retract" } else { "Extend" };
                                if ui.button(label).clicked() {
                                    solar_deploy_req = Some((part.part_index, part.deploy_fraction < 0.5));
                                }
                            }

                            // RTG info
                            if let Some(output) = part.rtg_output {
                                ui.separator();
                                ui.label(egui::RichText::new("RTG").strong());
                                ui.label(format!("Output: {:.0} W", output));
                            }

                            // Reactor info
                            if let Some(output) = part.reactor_output {
                                ui.separator();
                                ui.label(egui::RichText::new("Reactor").strong());
                                ui.label(format!("Output: {}", format_power_si(output)));
                            }

                            // Shield info
                            if let Some(ref shield_type) = part.shield_type {
                                ui.separator();
                                ui.label(egui::RichText::new("Shield").strong());
                                ui.label(format!("Type: {}", shield_type));
                                if let Some(max_c) = part.shield_max_c {
                                    ui.label(format!("Max Velocity: {:.0}% c", max_c * 100.0));
                                }
                                if let Some(power) = part.shield_power {
                                    if power > 0.0 {
                                        ui.label(format!("Power Draw: {}", format_power_si(power)));
                                    } else {
                                        ui.label("Power Draw: None (passive)");
                                    }
                                }
                            }

                            // Decoupler info
                            if part.is_decoupler {
                                ui.separator();
                                ui.label(egui::RichText::new("Decoupler").strong());

                                let crossfeed_label = if part.crossfeed_enabled {
                                    "Disable Crossfeed"
                                } else {
                                    "Enable Crossfeed"
                                };
                                if ui.button(crossfeed_label).clicked() {
                                    crossfeed_toggle_req = Some((part.part_index, !part.crossfeed_enabled));
                                }

                                if ui.button("Decouple").clicked() {
                                    decouple_req = Some(part.part_index);
                                }
                            }

                            // Fairing info
                            if part.is_fairing {
                                ui.separator();
                                ui.label(egui::RichText::new("Fairing").strong());
                                if ui.button("Deploy").clicked() {
                                    fairing_deploy_req = Some(part.part_index);
                                }
                            }

                            // Parachute info
                            if part.is_parachute {
                                ui.separator();
                                ui.label(egui::RichText::new("Parachute").strong());
                                ui.label(format!("Deployed Width: {:.1} m", part.parachute_deployed_width_m));
                                if part.parachute_spent {
                                    let btn = ui.add_enabled(false, egui::Button::new("Spent"));
                                    btn.on_disabled_hover_text("Parachute already used");
                                } else if part.parachute_deployed {
                                    ui.label("Status: Deployed");
                                } else {
                                    let can_deploy = ship_in_atmosphere && !ship_is_landed;
                                    let btn = ui.add_enabled(can_deploy, egui::Button::new("Deploy"));
                                    if btn.clicked() {
                                        parachute_deploy_req = Some(part.part_index);
                                    }
                                    if !can_deploy {
                                        if !ship_in_atmosphere {
                                            btn.on_disabled_hover_text("Cannot deploy in vacuum");
                                        } else {
                                            btn.on_disabled_hover_text("Cannot deploy while landed");
                                        }
                                    }
                                }
                            }
                        });
                }
            }

            // Draw maneuver node markers
            let node_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("maneuver_node_markers"),
            ));

            for node in &maneuver_nodes {
                if let Some(world_pos) = node_world_pos(node) {
                    let (scr_x, scr_y) = world_to_screen([world_pos[0] - camera_pos[0], world_pos[1] - camera_pos[1]]);

                    let is_selected = selected_maneuver_node == Some(node.id);
                    let marker_color = if is_selected {
                        egui::Color32::from_rgb(255, 200, 100) // Bright gold when selected
                    } else {
                        egui::Color32::from_rgb(200, 150, 50) // Dimmer gold
                    };

                    // Draw a diamond marker
                    let marker_size = if is_selected { 10.0 } else { 8.0 };
                    let center = egui::pos2(scr_x, scr_y);
                    let points = vec![
                        egui::pos2(center.x, center.y - marker_size),
                        egui::pos2(center.x + marker_size, center.y),
                        egui::pos2(center.x, center.y + marker_size),
                        egui::pos2(center.x - marker_size, center.y),
                    ];
                    node_painter.add(egui::Shape::convex_polygon(
                        points,
                        marker_color,
                        egui::Stroke::new(1.5, egui::Color32::WHITE),
                    ));

                    // Show remaining delta-v on hover or when selected
                    if is_selected || mouse_pos.map_or(false, |mp| (mp - center).length() < 15.0) {
                        let remaining_dv = node.total_remaining_delta_v();
                        if remaining_dv > 0.0 {
                            node_painter.text(
                                egui::pos2(scr_x, scr_y - 18.0),
                                egui::Align2::CENTER_BOTTOM,
                                format!("{:.0} m/s", remaining_dv),
                                egui::FontId::proportional(10.0),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
            }

            // Transfer planner window
            if self.transfer_planner_open {
                let mut close_planner = false;
                egui::Area::new(egui::Id::new("transfer_planner"))
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .order(egui::Order::Middle)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 35, 240))
                            .inner_margin(egui::Margin::same(12.0))
                            .rounding(egui::Rounding::same(6.0))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 80)))
                            .show(ui, |ui| {
                                ui.set_min_width(280.0);
                                ui.set_max_width(320.0);

                                // Title bar with close button
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Transfer Planner").size(14.0).color(egui::Color32::WHITE).strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.small_button("X").clicked() {
                                            close_planner = true;
                                        }
                                    });
                                });
                                ui.separator();

                                // Mode selector
                                ui.horizontal(|ui| {
                                    ui.selectable_value(&mut self.transfer_planner_mode, 0, "Hohmann");
                                    ui.selectable_value(&mut self.transfer_planner_mode, 1, "Lambert");
                                });
                                ui.add_space(4.0);

                                // Target dropdown
                                let targets = if self.transfer_planner_mode == 0 {
                                    &self.transfer_hohmann_targets
                                } else {
                                    &self.transfer_interplanetary_targets
                                };

                                if targets.is_empty() {
                                    ui.label(egui::RichText::new("No valid targets").size(11.0).color(egui::Color32::from_rgb(200, 150, 100)));
                                } else {
                                    let current_name = self.transfer_selected_target
                                        .and_then(|idx| targets.iter().find(|(i, _)| *i == idx).map(|(_, n)| n.as_str()))
                                        .unwrap_or("Select target...");
                                    egui::ComboBox::from_label("")
                                        .selected_text(current_name)
                                        .show_ui(ui, |ui| {
                                            for (idx, name) in targets {
                                                let selected = self.transfer_selected_target == Some(*idx);
                                                if ui.selectable_label(selected, name).clicked() {
                                                    self.transfer_selected_target = Some(*idx);
                                                    // Reset Lambert offsets when changing target
                                                    self.transfer_departure_offset = 0.0;
                                                    self.transfer_arrival_offset = 0.0;
                                                }
                                            }
                                        });
                                }

                                ui.add_space(6.0);

                                // Lambert mode sliders
                                if self.transfer_planner_mode == 1 {
                                    ui.label(egui::RichText::new("Departure offset").size(10.0).color(egui::Color32::GRAY));
                                    ui.add(egui::Slider::new(&mut self.transfer_departure_offset, -6.3e7..=6.3e7)
                                        .text("s")
                                        .logarithmic(true)
                                        .clamp_to_range(true));
                                    ui.label(egui::RichText::new("Transfer time offset").size(10.0).color(egui::Color32::GRAY));
                                    ui.add(egui::Slider::new(&mut self.transfer_arrival_offset, -6.3e7..=6.3e7)
                                        .text("s")
                                        .logarithmic(true)
                                        .clamp_to_range(true));
                                    ui.add_space(4.0);
                                }

                                // Display results
                                if let Some(ref display) = self.transfer_display {
                                    if display.valid {
                                        ui.separator();
                                        let fmt_dv = |dv: f64| -> String {
                                            if dv >= 1000.0 { format!("{:.2} km/s", dv / 1000.0) }
                                            else { format!("{:.1} m/s", dv) }
                                        };
                                        let fmt_time = |t: f64| -> String {
                                            if t >= 86400.0 * 365.25 {
                                                format!("{:.1}y", t / (86400.0 * 365.25))
                                            } else if t >= 86400.0 {
                                                format!("{:.1}d", t / 86400.0)
                                            } else if t >= 3600.0 {
                                                format!("{:.1}h", t / 3600.0)
                                            } else {
                                                format!("{:.0}s", t)
                                            }
                                        };

                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Departure dv:").size(11.0).color(egui::Color32::GRAY));
                                            ui.label(egui::RichText::new(fmt_dv(display.departure_dv)).size(11.0).color(egui::Color32::WHITE));
                                        });
                                        if display.mode == 0 {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("Arrival dv:").size(11.0).color(egui::Color32::GRAY));
                                                ui.label(egui::RichText::new(fmt_dv(display.arrival_dv)).size(11.0).color(egui::Color32::WHITE));
                                            });
                                        } else {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("Arrival v_inf:").size(11.0).color(egui::Color32::GRAY));
                                                ui.label(egui::RichText::new(fmt_dv(display.arrival_dv)).size(11.0).color(egui::Color32::WHITE));
                                            });
                                        }
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Transfer time:").size(11.0).color(egui::Color32::GRAY));
                                            ui.label(egui::RichText::new(fmt_time(display.transfer_time)).size(11.0).color(egui::Color32::WHITE));
                                        });

                                        // Phase angle display
                                        let phase_diff = (display.current_phase_angle - display.required_phase_angle).abs();
                                        let phase_color = if phase_diff < 5.0 {
                                            egui::Color32::from_rgb(100, 220, 100)
                                        } else if phase_diff < 20.0 {
                                            egui::Color32::from_rgb(220, 220, 100)
                                        } else {
                                            egui::Color32::from_rgb(200, 200, 200)
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Phase angle:").size(11.0).color(egui::Color32::GRAY));
                                            ui.label(egui::RichText::new(format!("{:.1}", display.current_phase_angle)).size(11.0).color(phase_color));
                                            ui.label(egui::RichText::new(format!("/ {:.1}", display.required_phase_angle)).size(11.0).color(egui::Color32::GRAY));
                                        });
                                        if display.time_to_window > 60.0 {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("Window in:").size(11.0).color(egui::Color32::GRAY));
                                                ui.label(egui::RichText::new(fmt_time(display.time_to_window)).size(11.0).color(egui::Color32::WHITE));
                                            });
                                        }

                                        ui.add_space(6.0);
                                        let create_btn = egui::Button::new(
                                            egui::RichText::new("Create Node").size(12.0).color(egui::Color32::WHITE)
                                        ).fill(egui::Color32::from_rgb(60, 120, 60));
                                        if ui.add(create_btn).clicked() {
                                            self.transfer_node_request = Some((
                                                display.departure_position_angle,
                                                display.prograde_dv,
                                                display.radial_dv,
                                                display.time_to_window,
                                            ));
                                            close_planner = true;
                                        }
                                    } else {
                                        ui.label(egui::RichText::new("No valid transfer found").size(11.0).color(egui::Color32::from_rgb(220, 100, 100)));
                                    }
                                }
                            });
                    });
                if close_planner {
                    self.transfer_planner_open = false;
                }
            }

            // Debug menu window
            if self.debug_menu_open {
                egui::Window::new("Debug")
                    .collapsible(false)
                    .resizable(false)
                    .default_pos(egui::pos2(60.0, 200.0))
                    .show(ctx, |ui| {
                        // Infinite fuel toggle
                        let fuel_label = if self.debug_infinite_fuel { "Infinite Fuel: ON" } else { "Infinite Fuel: OFF" };
                        let fuel_color = if self.debug_infinite_fuel {
                            egui::Color32::from_rgb(60, 130, 60)
                        } else {
                            egui::Color32::from_rgb(80, 80, 90)
                        };
                        if ui.add(egui::Button::new(fuel_label).fill(fuel_color).min_size(egui::vec2(160.0, 24.0))).clicked() {
                            self.debug_infinite_fuel = !self.debug_infinite_fuel;
                        }

                        ui.add_space(4.0);

                        // Teleport to LEO
                        if ui.add(egui::Button::new("Set Orbit (LEO)").min_size(egui::vec2(160.0, 24.0))).clicked() {
                            self.debug_teleport_leo = true;
                        }
                    });
            }

            // Pause overlay (drawn on top of everything)
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
                                    if self.show_quicksave_list {
                                        // Quicksave list view
                                        ui.heading(egui::RichText::new("Load Quicksave").size(28.0).color(egui::Color32::WHITE));
                                        ui.add_space(12.0);
                                        egui::ScrollArea::vertical()
                                            .max_height(300.0)
                                            .show(ui, |ui| {
                                                for qs in quicksaves {
                                                    let label = format!(
                                                        "#{} — {}",
                                                        qs.index,
                                                        crate::game::format_date(qs.simulation_time),
                                                    );
                                                    if ui.button(egui::RichText::new(&label).size(16.0)).clicked() {
                                                        pause_action = PauseAction::LoadQuicksave(qs.filename.clone());
                                                        self.show_quicksave_list = false;
                                                    }
                                                }
                                            });
                                        ui.add_space(12.0);
                                        if ui.button(egui::RichText::new("Back").size(18.0)).clicked() {
                                            self.show_quicksave_list = false;
                                        }
                                    } else {
                                        // Default pause view
                                        ui.heading(egui::RichText::new("Paused").size(32.0).color(egui::Color32::WHITE));
                                        ui.add_space(20.0);
                                        if ui.button(egui::RichText::new("Quicksave").size(18.0)).clicked() {
                                            pause_action = PauseAction::Quicksave;
                                        }
                                        if !quicksaves.is_empty() {
                                            ui.add_space(8.0);
                                            if ui.button(egui::RichText::new("Load Quicksave").size(18.0)).clicked() {
                                                self.show_quicksave_list = true;
                                            }
                                        }
                                        if can_recover {
                                            ui.add_space(8.0);
                                            let recover_btn = egui::Button::new(
                                                egui::RichText::new("Recover Vessel").size(18.0)
                                            ).fill(egui::Color32::from_rgb(60, 130, 60));
                                            if ui.add(recover_btn).clicked() {
                                                pause_action = PauseAction::RecoverVessel;
                                            }
                                        }
                                        ui.add_space(8.0);
                                        if can_exit_flight {
                                            if ui.button(egui::RichText::new("Main Menu").size(18.0)).clicked() {
                                                pause_action = PauseAction::MainMenu;
                                            }
                                        } else {
                                            ui.add_enabled(false, egui::Button::new(egui::RichText::new("Main Menu").size(18.0)));
                                            ui.label(egui::RichText::new("Cannot exit while in atmosphere or landing zone")
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(200, 150, 100)));
                                        }
                                    }
                                });
                            });
                    });
            }
        });

        self.egui_state.handle_platform_output(&self.window, full_output.platform_output);

        // Handle maneuver node UI actions
        if let Some((ta, ref segment)) = create_node_at {
            self.create_maneuver_node(ta, segment);
        }
        if let Some(node_id) = delete_node_id {
            // Cancel warp-to-node if the first node is being deleted
            if self.maneuver_nodes.first().map(|n| n.id) == Some(node_id) {
                self.warp_to_node = false;
            }
            self.delete_maneuver_node(node_id);
        }
        if close_maneuver_panel {
            self.selected_maneuver_node = None;
        }
        if start_warp_to_node {
            self.warp_to_node = true;
        }
        if cancel_warp_to_node {
            self.warp_to_node = false;
        }
        // Update autopilot target
        self.autopilot_target = new_autopilot_target;
        // Store engine toggle request for main.rs to process
        if engine_toggle_req.is_some() {
            self.engine_toggle_request = engine_toggle_req;
        }
        // Store crossfeed toggle and decouple requests for main.rs to process
        if crossfeed_toggle_req.is_some() {
            self.crossfeed_toggle_request = crossfeed_toggle_req;
        }
        if decouple_req.is_some() {
            self.decouple_request = decouple_req;
        }
        if fairing_deploy_req.is_some() {
            self.fairing_deploy_request = fairing_deploy_req;
        }
        if solar_deploy_req.is_some() {
            self.solar_deploy_request = solar_deploy_req;
        }
        if parachute_deploy_req.is_some() {
            self.parachute_deploy_request = parachute_deploy_req;
        }
        // Store staging reorder request for main.rs to process
        if staging_reorder_req.is_some() {
            self.staging_reorder = staging_reorder_req;
        }
        // Apply slider deltas to selected node with non-linear scaling
        // Full deflection = 1000 m/s per second, minimal = ~1 m/s per second
        if let Some(node_id) = self.selected_maneuver_node {
            if prograde_delta.abs() > 0.001 || radial_delta.abs() > 0.001 {
                if let Some(node) = self.get_maneuver_node_mut(node_id) {
                    // Non-linear scaling: use power of 2 for smooth precision control
                    // At 60fps: max deflection (100) = ~16.67 m/s/frame = 1000 m/s/s
                    // Small deflection (~3%) = ~0.017 m/s/frame = ~1 m/s/s
                    let apply_curve = |delta: f64| -> f64 {
                        let normalized = delta / 100.0; // -1 to 1
                        let curved = normalized.signum() * normalized.abs().powf(2.0);
                        curved * 16.67 // Scale to m/s per frame
                    };
                    let prograde_change = apply_curve(prograde_delta);
                    let radial_change = apply_curve(radial_delta);
                    // Update both original (for trajectory) and remaining (for display)
                    node.delta_v.prograde += prograde_change;
                    node.delta_v.radial_out += radial_change;
                    node.remaining_delta_v.prograde += prograde_change;
                    node.remaining_delta_v.radial_out += radial_change;
                }
            }
        }

        // Update maneuver node screen positions for click detection
        self.maneuver_node_screen_positions.clear();
        let scale_factor = self.window.scale_factor() as f32;
        for node in &self.maneuver_nodes {
            if let Some(world_pos) = self.maneuver_node_world_position(node) {
                let rel_x = (world_pos[0] - self.camera.position[0]) as f32;
                let rel_y = (world_pos[1] - self.camera.position[1]) as f32;
                let cos_r = self.camera.rotation.cos();
                let sin_r = self.camera.rotation.sin();
                let rot_x = rel_x * cos_r - rel_y * sin_r;
                let rot_y = rel_x * sin_r + rel_y * cos_r;
                let view_x = rot_x * self.camera.zoom;
                let view_y = rot_y * self.camera.zoom;
                let ndc_x = view_x / self.camera.aspect_ratio;
                let ndc_y = view_y;
                let scr_x = (ndc_x + 1.0) * 0.5 * self.size.width as f32 / scale_factor;
                let scr_y = (1.0 - ndc_y) * 0.5 * self.size.height as f32 / scale_factor;
                self.maneuver_node_screen_positions.push((node.id, [scr_x, scr_y]));
            }
        }

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
            render_pass.set_bind_group(1, &self.body_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.sprite_atlas.bind_group, &[]);
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

        Ok((new_warp_index, pause_action))
    }

    /// Get window reference
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Update geometry with multiple bodies and their orbits
    /// scale: world units per meter (e.g., 1e-9 means 1 billion meters = 1 world unit)
    pub fn update_bodies_with_orbits(
        &mut self,
        bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)],
        orbits: &[Option<OrbitRenderData>],
        scale: f64,
    ) {
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        // Get camera position for relative coordinate calculation
        let cam_x = self.camera.body_center[0];
        let cam_y = self.camera.body_center[1];
        let off_x = self.camera.ship_offset[0];
        let off_y = self.camera.ship_offset[1];

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
                    let rel_outer_x = (px + nx - cam_x - off_x) as f32;
                    let rel_outer_y = (py + ny - cam_y - off_y) as f32;
                    all_vertices.push(Vertex::new([rel_outer_x, rel_outer_y], orbit.color));

                    // Inner vertex
                    let rel_inner_x = (px - nx - cam_x - off_x) as f32;
                    let rel_inner_y = (py - ny - cam_y - off_y) as f32;
                    all_vertices.push(Vertex::new([rel_inner_x, rel_inner_y], [orbit.color[0] * 0.5, orbit.color[1] * 0.5, orbit.color[2] * 0.5, orbit.color[3] * 0.7]));
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
        bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)],
        orbits: &[Option<OrbitRenderData>],
        ship: Option<&ShipRenderData>,
        scale: f64,
        part_defs: Option<&crate::parts::PartDefinitions>,
    ) {
        self.update_bodies_orbits_ship_and_vessels(bodies, orbits, ship, scale, part_defs, &[], &[], false);
    }

    /// Update geometry with bodies, orbits, optionally a ship, and background vessels
    pub fn update_bodies_orbits_ship_and_vessels(
        &mut self,
        bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)],
        orbits: &[Option<OrbitRenderData>],
        ship: Option<&ShipRenderData>,
        scale: f64,
        part_defs: Option<&crate::parts::PartDefinitions>,
        background_vessels: &[TrackingVesselData],
        accretion_discs: &[Option<crate::bodies::AccretionDisc>],
        in_galaxy_view: bool,
    ) {
        // Store ship info for UI display
        self.ship_orbit_info = ship.and_then(|s| s.orbit.clone());
        if let Some(s) = ship {
            self.ship_velocity = s.velocity;
            self.ship_altitude = s.altitude;
            self.ship_throttle = s.throttle;
            self.ship_soi_name = s.soi_body_name.clone();
            self.ship_time_to_intercept = s.time_to_intercept;
            self.ship_acceleration = s.acceleration;
            self.ship_current_true_anomaly = s.current_true_anomaly;
            self.vessel_total_mass = s.total_mass;
            self.vessel_fuel_fraction = s.fuel_fraction;
            self.vessel_power_generation = s.power_generation;
            self.vessel_power_consumption = s.power_consumption;
            self.vessel_electricity_fraction = s.electricity_fraction;
            self.vessel_electricity_stored = s.electricity_stored;
            self.vessel_electricity_max = s.electricity_max;
            self.vessel_thrust_kn = s.thrust_kn;
            self.vessel_drag_kn = s.drag_kn;
            self.vessel_delta_v = s.delta_v;
            self.vessel_current_stage = s.current_stage;
            self.vessel_total_stages = s.total_stages;
            self.vessel_stages = s.stages.clone().unwrap_or_default();
            self.vessel_stage_delta_vs = s.stage_delta_vs.clone().unwrap_or_default();
            self.ship_soi_surface_gravity = s.soi_surface_gravity;
            self.ship_g_force = s.g_force;
            self.ship_temperature = s.temperature;
            self.ship_heat_fraction = s.heat_fraction;
            self.ship_heat_flux = s.heat_flux;
            self.ship_below_landing_altitude = s.below_landing_altitude;
            self.ship_velocity_direction = s.velocity_direction;
            self.ship_render_x = s.x;
            self.ship_render_y = s.y;
            self.ship_render_rotation = s.rotation;
            self.ship_render_scale = if s.size > 0.0 { scale } else { 1.0 };
            if let Some(ref parts) = s.parts {
                self.flight_parts_cache = parts.clone();
            } else {
                self.flight_parts_cache.clear();
            }
            // Store trajectory for orbit click detection
            self.current_trajectory = s.patched_trajectory.clone();
            // Update predicted orbits with new trajectory data
        } else {
            self.current_trajectory.clear();
            self.predicted_trajectories.clear();
        }

        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        let cam_x = self.camera.body_center[0];
        let cam_y = self.camera.body_center[1];
        let off_x = self.camera.ship_offset[0];
        let off_y = self.camera.ship_offset[1];

        // Draw atmosphere behind everything
        self.add_atmosphere_vertices(&mut all_vertices, &mut all_indices, bodies, scale);

        // Draw accretion discs (behind orbits and bodies)
        self.add_accretion_disc_vertices(&mut all_vertices, &mut all_indices, bodies, accretion_discs, scale);

        // Draw galaxy star field dots (behind orbits and bodies, on top of accretion disc)
        self.add_galaxy_texture_quad(&mut all_vertices, &mut all_indices, in_galaxy_view, scale);

        // Draw all orbit lines (on top of atmosphere, behind bodies)
        for orbit_opt in orbits {
            if let Some(orbit) = orbit_opt {
                let base_index = all_vertices.len() as u32;

                let a = orbit.semi_major_axis;
                let e = orbit.eccentricity;
                let b = a * (1.0 - e * e).sqrt();
                let c = a * e;

                let arg_peri = orbit.argument_of_periapsis;
                // Subtract camera from parent first for precision — both are galaxy-scale,
                // their difference is solar-system-scale, so orbit geometry stays precise.
                let pcam_x = orbit.parent_x - cam_x - off_x;
                let pcam_y = orbit.parent_y - cam_y - off_y;
                let center_x = pcam_x - c * arg_peri.cos();
                let center_y = pcam_y - c * arg_peri.sin();

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

                    let rel_outer_x = (px + nx) as f32;
                    let rel_outer_y = (py + ny) as f32;
                    all_vertices.push(Vertex::new([rel_outer_x, rel_outer_y], orbit.color));

                    let rel_inner_x = (px - nx) as f32;
                    let rel_inner_y = (py - ny) as f32;
                    all_vertices.push(Vertex::new([rel_inner_x, rel_inner_y], [orbit.color[0] * 0.5, orbit.color[1] * 0.5, orbit.color[2] * 0.5, orbit.color[3] * 0.7]));
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

        // Draw ship orbit line (patched conics) on top of celestial orbits but BELOW bodies
        // Only show orbit line when ship is small on screen (< 5 pixels)
        self.ap_markers.clear();
        self.pe_markers.clear();
        self.closest_approach_marker = None;

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

                        // Subtract camera from parent first for orbit precision
                        let pcam_x = segment.parent_x - cam_x - off_x;
                        let pcam_y = segment.parent_y - cam_y - off_y;

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

                            // Position relative to camera (focus at parent, camera-relative)
                            let angle = ta + arg_peri;
                            let px = pcam_x + r * angle.cos();
                            let py = pcam_y + r * angle.sin();

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

                                let rel_outer_x = (px + nx) as f32;
                                let rel_outer_y = (py + ny) as f32;
                                all_vertices.push(Vertex::new([rel_outer_x, rel_outer_y], segment.color));

                                let rel_inner_x = (px - nx) as f32;
                                let rel_inner_y = (py - ny) as f32;
                                all_vertices.push(Vertex::new([rel_inner_x, rel_inner_y], [segment.color[0] * 0.5, segment.color[1] * 0.5, segment.color[2] * 0.5, segment.color[3] * 0.7]));
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

                        // Check if periapsis (true anomaly = 0) will be reached
                        // For hyperbolic orbits, the ship travels monotonically from start_ta to end_ta
                        // Prograde: ta increases, so Pe is reached if start_ta <= 0 <= end_ta
                        // Retrograde: ta decreases, so Pe is reached if start_ta >= 0 >= end_ta
                        let pe_will_be_reached = if segment.retrograde {
                            start_ta >= 0.0 && end_ta <= 0.0
                        } else {
                            start_ta <= 0.0 && end_ta >= 0.0
                        };

                        // Only draw periapsis marker if it will be reached
                        if pe_will_be_reached {
                            // Draw periapsis marker for hyperbolic trajectory
                            // Periapsis is at true anomaly = 0
                            let a_abs = segment.semi_major_axis.abs();
                            let p = a_abs * (e * e - 1.0);
                            let pe_r = p / (1.0 + e); // Distance at periapsis
                            let pe_x = pcam_x + pe_r * arg_peri.cos();
                            let pe_y = pcam_y + pe_r * arg_peri.sin();

                            // Use dimmer color for future segments
                            let alpha = if segment.is_first_segment { 1.0 } else { 0.6 };
                            let pe_color = [0.3, 0.8, 1.0, alpha];

                            let pe_base = all_vertices.len() as u32;
                            all_vertices.push(Vertex::new([pe_x as f32, pe_y as f32], pe_color));
                            for i in 0..marker_segments {
                                let angle = (i as f64 / marker_segments as f64) * std::f64::consts::TAU;
                                all_vertices.push(Vertex::new([
                                        (pe_x + marker_radius * angle.cos()) as f32,
                                        (pe_y + marker_radius * angle.sin()) as f32,
                                    ], pe_color));
                            }
                            for i in 0..marker_segments {
                                all_indices.push(pe_base);
                                all_indices.push(pe_base + 1 + i);
                                all_indices.push(pe_base + 1 + (i + 1) % marker_segments);
                            }

                            // Store position and altitude for UI hover
                            // pe_r is in scaled units, convert to meters then subtract body radius
                            let pe_altitude = (pe_r / segment.render_scale) - segment.parent_body_radius;
                            self.pe_markers.push(([pe_x, pe_y], pe_altitude));
                        }

                        continue; // Skip ellipse drawing code
                    }

                    // Elliptical orbit — subtract camera from parent first for precision
                    let a = segment.semi_major_axis;
                    let b = a * (1.0 - e * e).sqrt();
                    let c = a * e;

                    let pcam_x = segment.parent_x - cam_x - off_x;
                    let pcam_y = segment.parent_y - cam_y - off_y;
                    let center_x = pcam_x - c * arg_peri.cos();
                    let center_y = pcam_y - c * arg_peri.sin();

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
                        None => {
                            // Full orbit - but start from the ship's entry point
                            // Convert start_true_anomaly to eccentric anomaly
                            let start_ta = segment.start_true_anomaly;
                            let start_ea = (start_ta.sin() * (1.0 - e * e).sqrt()).atan2(e + start_ta.cos());
                            (start_ea, std::f64::consts::TAU)
                        }
                    };

                    let is_full_orbit = segment.end_true_anomaly.is_none();
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

                        let rel_outer_x = (px + nx) as f32;
                        let rel_outer_y = (py + ny) as f32;
                        all_vertices.push(Vertex::new([rel_outer_x, rel_outer_y], segment.color));

                        let rel_inner_x = (px - nx) as f32;
                        let rel_inner_y = (py - ny) as f32;
                        all_vertices.push(Vertex::new([rel_inner_x, rel_inner_y], [segment.color[0] * 0.5, segment.color[1] * 0.5, segment.color[2] * 0.5, segment.color[3] * 0.7]));
                    }

                    // For full orbits, wrap around to connect last segment to first
                    // For partial orbits, only connect consecutive segments
                    let index_count = if is_full_orbit { num_segments } else { num_segments.saturating_sub(1) };
                    for i in 0..index_count {
                        let i0 = base_index + i * 2;
                        let i1 = base_index + i * 2 + 1;
                        let next_i = if is_full_orbit { (i + 1) % num_segments } else { i + 1 };
                        let i2 = base_index + next_i * 2;
                        let i3 = base_index + next_i * 2 + 1;

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
                        all_vertices.push(Vertex::new([pe_x as f32, pe_y as f32], pe_color));
                        for i in 0..marker_segments {
                            let angle = (i as f64 / marker_segments as f64) * std::f64::consts::TAU;
                            all_vertices.push(Vertex::new([
                                    (pe_x + marker_radius * angle.cos()) as f32,
                                    (pe_y + marker_radius * angle.sin()) as f32,
                                ], pe_color));
                        }
                        for i in 0..marker_segments {
                            all_indices.push(pe_base);
                            all_indices.push(pe_base + 1 + i);
                            all_indices.push(pe_base + 1 + (i + 1) % marker_segments);
                        }
                        // Store for UI hover
                        self.pe_markers.push(([pe_x, pe_y], pe_altitude));
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
                        all_vertices.push(Vertex::new([ap_x as f32, ap_y as f32], ap_color));
                        for i in 0..marker_segments {
                            let angle = (i as f64 / marker_segments as f64) * std::f64::consts::TAU;
                            all_vertices.push(Vertex::new([
                                    (ap_x + marker_radius * angle.cos()) as f32,
                                    (ap_y + marker_radius * angle.sin()) as f32,
                                ], ap_color));
                        }
                        for i in 0..marker_segments {
                            all_indices.push(ap_base);
                            all_indices.push(ap_base + 1 + i);
                            all_indices.push(ap_base + 1 + (i + 1) % marker_segments);
                        }
                        // Store for UI hover
                        self.ap_markers.push(([ap_x, ap_y], ap_altitude));
                    }
                }

                // Draw closest approach marker (yellow dot)
                if let Some((world_pos, dist)) = self.closest_approach_world_pos {
                    // Subtract camera first for precision (both galaxy-scale)
                    let ca_x = world_pos[0] - cam_x - off_x;
                    let ca_y = world_pos[1] - cam_y - off_y;
                    let ca_color = [1.0, 1.0, 0.0, 0.9_f32];

                    let ca_base = all_vertices.len() as u32;
                    all_vertices.push(Vertex::new([ca_x as f32, ca_y as f32], ca_color));
                    for i in 0..marker_segments {
                        let angle = (i as f64 / marker_segments as f64) * std::f64::consts::TAU;
                        all_vertices.push(Vertex::new([
                            (ca_x + marker_radius * angle.cos()) as f32,
                            (ca_y + marker_radius * angle.sin()) as f32,
                        ], ca_color));
                    }
                    for i in 0..marker_segments {
                        all_indices.push(ca_base);
                        all_indices.push(ca_base + 1 + i);
                        all_indices.push(ca_base + 1 + (i + 1) % marker_segments);
                    }
                    self.closest_approach_marker = Some(([ca_x, ca_y], dist));
                }
            }
        }

        // Draw predicted trajectories as solid green lines
        for trajectory in &self.predicted_trajectories {
            for (seg_idx, segment) in trajectory.iter().enumerate() {
                let e = segment.eccentricity;
                let arg_peri = segment.argument_of_periapsis;
                let a = segment.semi_major_axis;

                let line_width = 0.0015 / self.camera.zoom as f64;

                // Green for first segment, dimmer for subsequent segments
                let alpha = if seg_idx == 0 { 0.9 } else { 0.6 };
                let seg_color = [0.0, 1.0, 0.0, alpha];

                // Subtract camera from parent first for precision — both are galaxy-scale,
                // their difference is solar-system-scale, so orbit geometry stays precise.
                let pcam_x = segment.parent_x - cam_x - off_x;
                let pcam_y = segment.parent_y - cam_y - off_y;

                if e >= 1.0 {
                    // Hyperbolic orbit segment
                    let a_abs = a.abs();
                    let p = a_abs * (e * e - 1.0);
                    let max_ta = (-1.0 / e).acos();

                    let start_ta = segment.start_true_anomaly;
                    let end_ta = segment.end_true_anomaly.unwrap_or_else(|| {
                        if segment.retrograde {
                            -(max_ta - HYPERBOLIC_RENDER_MARGIN)
                        } else {
                            max_ta - HYPERBOLIC_RENDER_MARGIN
                        }
                    });

                    let num_points = 512usize;
                    let mut points: Vec<(f64, f64)> = Vec::with_capacity(num_points);

                    for i in 0..num_points {
                        let t = i as f64 / (num_points - 1) as f64;
                        let ta = start_ta + t * (end_ta - start_ta);

                        if ta.abs() >= max_ta - HYPERBOLIC_SKIP_MARGIN {
                            continue;
                        }

                        let denom = 1.0 + e * ta.cos();
                        if denom <= 0.001 {
                            continue;
                        }
                        let r = p / denom;
                        if r <= 0.0 || !r.is_finite() {
                            continue;
                        }

                        let angle = ta + arg_peri;
                        let px = pcam_x + r * angle.cos();
                        let py = pcam_y + r * angle.sin();
                        points.push((px, py));
                    }

                    // Draw solid line segments
                    for i in 0..points.len().saturating_sub(1) {
                        let (px, py) = points[i];
                        let (nx, ny) = points[i + 1];

                        let dx = nx - px;
                        let dy = ny - py;
                        let seg_len = (dx * dx + dy * dy).sqrt();

                        if seg_len < 1e-10 {
                            continue;
                        }

                        let base_index = all_vertices.len() as u32;
                        let len = seg_len;
                        let nx_perp = -dy / len * line_width;
                        let ny_perp = dx / len * line_width;

                        all_vertices.push(Vertex::new([(px + nx_perp) as f32, (py + ny_perp) as f32], seg_color));
                        all_vertices.push(Vertex::new([(px - nx_perp) as f32, (py - ny_perp) as f32], seg_color));
                        all_vertices.push(Vertex::new([(nx + nx_perp) as f32, (ny + ny_perp) as f32], seg_color));
                        all_vertices.push(Vertex::new([(nx - nx_perp) as f32, (ny - ny_perp) as f32], seg_color));

                        all_indices.push(base_index);
                        all_indices.push(base_index + 2);
                        all_indices.push(base_index + 1);
                        all_indices.push(base_index + 1);
                        all_indices.push(base_index + 2);
                        all_indices.push(base_index + 3);
                    }

                    // Draw periapsis marker for hyperbolic (if we'll reach it)
                    let start_ta_norm = start_ta;
                    let end_ta_norm = end_ta;
                    let pe_will_be_reached = if segment.retrograde {
                        start_ta_norm >= 0.0 && end_ta_norm <= 0.0
                    } else {
                        start_ta_norm <= 0.0 && end_ta_norm >= 0.0
                    };

                    if pe_will_be_reached {
                        let pe_r = p / (1.0 + e);
                        let pe_x = pcam_x + pe_r * arg_peri.cos();
                        let pe_y = pcam_y + pe_r * arg_peri.sin();
                        let marker_radius = 0.006 / self.camera.zoom as f64;
                        let marker_segments = 12u32;
                        let marker_alpha = if seg_idx == 0 { 0.7f32 } else { 0.5f32 };
                        let pe_color = [0.2, 0.7, 0.9, marker_alpha];

                        let pe_base = all_vertices.len() as u32;
                        all_vertices.push(Vertex::new([pe_x as f32, pe_y as f32], pe_color));
                        for j in 0..marker_segments {
                            let angle = (j as f64 / marker_segments as f64) * std::f64::consts::TAU;
                            all_vertices.push(Vertex::new([
                                    (pe_x + marker_radius * angle.cos()) as f32,
                                    (pe_y + marker_radius * angle.sin()) as f32,
                                ], pe_color));
                        }
                        for j in 0..marker_segments {
                            all_indices.push(pe_base);
                            all_indices.push(pe_base + 1 + j);
                            all_indices.push(pe_base + 1 + (j + 1) % marker_segments);
                        }

                        // Store for UI hover display
                        let pe_distance = pe_r / segment.render_scale;
                        let pe_altitude = pe_distance - segment.parent_body_radius;
                        self.pe_markers.push(([pe_x, pe_y], pe_altitude));
                    }
                } else {
                    // Elliptical orbit segment
                    let b = a * (1.0 - e * e).sqrt();
                    let c = a * e;
                    let center_x = pcam_x - c * arg_peri.cos();
                    let center_y = pcam_y - c * arg_peri.sin();

                    let start_ta = segment.start_true_anomaly;
                    let start_ea = (start_ta.sin() * (1.0 - e * e).sqrt()).atan2(e + start_ta.cos());

                    // Calculate angle span
                    let angle_span = match segment.end_true_anomaly {
                        Some(end_ta) => {
                            let end_ea = (end_ta.sin() * (1.0 - e * e).sqrt()).atan2(e + end_ta.cos());
                            if segment.retrograde {
                                let mut s = start_ea - end_ea;
                                if s < 0.0 { s += std::f64::consts::TAU; }
                                -s
                            } else {
                                let mut s = end_ea - start_ea;
                                if s < 0.0 { s += std::f64::consts::TAU; }
                                s
                            }
                        }
                        None => std::f64::consts::TAU,
                    };

                    let num_segments_draw = 512u32;
                    let mut prev_point: Option<(f64, f64)> = None;

                    for i in 0..=num_segments_draw {
                        let t = i as f64 / num_segments_draw as f64;
                        let ea = start_ea + t * angle_span;

                        let ex = a * ea.cos();
                        let ey = b * ea.sin();
                        let rx = ex * arg_peri.cos() - ey * arg_peri.sin();
                        let ry = ex * arg_peri.sin() + ey * arg_peri.cos();
                        let px = center_x + rx;
                        let py = center_y + ry;

                        if let Some((prev_x, prev_y)) = prev_point {
                            let dx = px - prev_x;
                            let dy = py - prev_y;
                            let seg_len = (dx * dx + dy * dy).sqrt();

                            if seg_len >= 1e-10 {
                                let base_index = all_vertices.len() as u32;
                                let len = seg_len;
                                let nx_perp = -dy / len * line_width;
                                let ny_perp = dx / len * line_width;

                                all_vertices.push(Vertex::new([(prev_x + nx_perp) as f32, (prev_y + ny_perp) as f32], seg_color));
                                all_vertices.push(Vertex::new([(prev_x - nx_perp) as f32, (prev_y - ny_perp) as f32], seg_color));
                                all_vertices.push(Vertex::new([(px + nx_perp) as f32, (py + ny_perp) as f32], seg_color));
                                all_vertices.push(Vertex::new([(px - nx_perp) as f32, (py - ny_perp) as f32], seg_color));

                                all_indices.push(base_index);
                                all_indices.push(base_index + 2);
                                all_indices.push(base_index + 1);
                                all_indices.push(base_index + 1);
                                all_indices.push(base_index + 2);
                                all_indices.push(base_index + 3);
                            }
                        }

                        prev_point = Some((px, py));
                    }

                    // Draw Ap/Pe markers for all segments of predicted trajectories
                    let marker_radius = 0.006 / self.camera.zoom as f64;
                    let marker_segments = 12u32;
                    let marker_alpha = if seg_idx == 0 { 0.7f32 } else { 0.5f32 };

                    // Helper to check if a true anomaly is in the arc from start to end
                    let is_in_arc = |marker_ta: f64, start_ta: f64, end_ta: f64, retrograde: bool| -> bool {
                        let tau = std::f64::consts::TAU;
                        let normalize = |ang: f64| ang.rem_euclid(tau);
                        let marker = normalize(marker_ta);
                        let start = normalize(start_ta);
                        let end = normalize(end_ta);

                        if retrograde {
                            if start >= end { marker <= start && marker >= end }
                            else { marker >= end || marker <= start }
                        } else {
                            if start <= end { marker >= start && marker <= end }
                            else { marker >= start || marker <= end }
                        }
                    };

                    // Determine which markers to show
                    let (show_pe, show_ap) = if let Some(end_ta) = segment.end_true_anomaly {
                        let start_ta = segment.start_true_anomaly;
                        let pe_in_arc = is_in_arc(0.0, start_ta, end_ta, segment.retrograde);
                        let ap_in_arc = is_in_arc(std::f64::consts::PI, start_ta, end_ta, segment.retrograde);
                        (pe_in_arc, ap_in_arc)
                    } else {
                        (true, true) // Full orbit, show both
                    };

                    // Periapsis (ta = 0) - cyan
                    if show_pe {
                        let pe_r = a * (1.0 - e);
                        let pe_x = pcam_x + pe_r * arg_peri.cos();
                        let pe_y = pcam_y + pe_r * arg_peri.sin();
                        let pe_color = [0.2, 0.7, 0.9, marker_alpha];

                        let pe_base = all_vertices.len() as u32;
                        all_vertices.push(Vertex::new([pe_x as f32, pe_y as f32], pe_color));
                        for j in 0..marker_segments {
                            let angle = (j as f64 / marker_segments as f64) * std::f64::consts::TAU;
                            all_vertices.push(Vertex::new([
                                    (pe_x + marker_radius * angle.cos()) as f32,
                                    (pe_y + marker_radius * angle.sin()) as f32,
                                ], pe_color));
                        }
                        for j in 0..marker_segments {
                            all_indices.push(pe_base);
                            all_indices.push(pe_base + 1 + j);
                            all_indices.push(pe_base + 1 + (j + 1) % marker_segments);
                        }

                        // Store for UI hover display
                        let pe_distance = a * (1.0 - e) / segment.render_scale;
                        let pe_altitude = pe_distance - segment.parent_body_radius;
                        self.pe_markers.push(([pe_x, pe_y], pe_altitude));
                    }

                    // Apoapsis (ta = π) - orange
                    if show_ap {
                        let ap_r = a * (1.0 + e);
                        let ap_angle = arg_peri + std::f64::consts::PI;
                        let ap_x = pcam_x + ap_r * ap_angle.cos();
                        let ap_y = pcam_y + ap_r * ap_angle.sin();
                        let ap_color = [0.9, 0.5, 0.1, marker_alpha];

                        let ap_base = all_vertices.len() as u32;
                        all_vertices.push(Vertex::new([ap_x as f32, ap_y as f32], ap_color));
                        for j in 0..marker_segments {
                            let angle = (j as f64 / marker_segments as f64) * std::f64::consts::TAU;
                            all_vertices.push(Vertex::new([
                                    (ap_x + marker_radius * angle.cos()) as f32,
                                    (ap_y + marker_radius * angle.sin()) as f32,
                                ], ap_color));
                        }
                        for j in 0..marker_segments {
                            all_indices.push(ap_base);
                            all_indices.push(ap_base + 1 + j);
                            all_indices.push(ap_base + 1 + (j + 1) % marker_segments);
                        }

                        // Store for UI hover display
                        let ap_distance = a * (1.0 + e) / segment.render_scale;
                        let ap_altitude = ap_distance - segment.parent_body_radius;
                        self.ap_markers.push(([ap_x, ap_y], ap_altitude));
                    }
                }
            }
        }

        // Draw bodies on top of orbit lines
        self.add_body_vertices(&mut all_vertices, &mut all_indices, bodies, scale);

        // Draw launchpad on body surface (ship view only)
        if let Some(ship_data) = ship {
            self.add_launchpad_vertices(&mut all_vertices, &mut all_indices, bodies, scale, ship_data);
        }

        // Draw ship on top of everything
        if let Some(ship_data) = ship {
            // Ship position relative to camera, using two-step subtraction for precision.
            // Each subtraction is between values of similar magnitude, preserving f64 precision.
            let rel_x = ((self.ship_body_center[0] - cam_x) + (self.ship_rel_offset[0] - off_x)) as f32;
            let rel_y = ((self.ship_body_center[1] - cam_y) + (self.ship_rel_offset[1] - off_y)) as f32;
            let size = ship_data.size as f32;
            let rotation = ship_data.rotation as f32;

            // Calculate ship size in pixels
            let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;
            let ship_pixels = size * pixels_per_world_unit * 2.0;
            let needs_indicator = ship_pixels < 5.0;

            // Draw the actual ship (parts or triangle) if visible
            if ship_pixels >= 1.0 {
                let has_parts = ship_data.parts.is_some() && part_defs.is_some();

                if has_parts {
                    // Part-based rendering: render each part at its position
                    // Offset by -π/2 because editor parts are Y-up but rotation=0 means +X
                    let visual_rotation = rotation - std::f32::consts::FRAC_PI_2;
                    let parts = ship_data.parts.as_ref().unwrap();
                    let defs = part_defs.unwrap();
                    let cos_r = visual_rotation.cos();
                    let sin_r = visual_rotation.sin();
                    let render_scale = scale as f32;


                    for part_data in parts {
                        if let Some(def) = defs.get(&part_data.definition_id) {
                            // Transform part local position to world-relative position
                            let local_x = part_data.local_x as f32 * render_scale;
                            let local_y = part_data.local_y as f32 * render_scale;

                            // Rotate local position by vessel rotation
                            let rotated_x = local_x * cos_r - local_y * sin_r;
                            let rotated_y = local_x * sin_r + local_y * cos_r;

                            // Generate vertices at origin, then transform
                            // Skip base disc for fairing half debris (shell-only)
                            let mut part_verts: Vec<Vertex> = Vec::new();
                            if part_data.fairing_half.is_none() {
                                crate::editor::generate_part_shape_vertices(
                                    &mut part_verts, def, 0.0, 0.0, 1.0,
                                    Some(&self.sprite_atlas),
                                    if part_data.is_solar_panel { Some(part_data.deploy_fraction) } else { None },
                                );
                            }

                            // Add engine plume if this engine is firing
                            let plume_elapsed = self.plume_start_time.elapsed().as_secs_f64();
                            if part_data.engine_active && ship_data.throttle > 0.0 && def.engine.is_some() {
                                crate::editor::generate_engine_plume_vertices(
                                    &mut part_verts, def, 0.0, 0.0, ship_data.throttle as f32,
                                    Some(&self.sprite_atlas), plume_elapsed,
                                );
                            }

                            // Add RCS plumes if nozzles are active
                            if let Some(ref nozzle_state) = part_data.rcs_nozzle_state {
                                if def.rcs.is_some() {
                                    if def.category == crate::parts::PartCategory::Pods {
                                        // Pods have bilateral nozzles — use pod-specific plume function
                                        crate::editor::generate_pod_rcs_plume_vertices(
                                            &mut part_verts, def, 0.0, 0.0, nozzle_state,
                                        );
                                    } else {
                                        crate::editor::generate_rcs_plume_vertices(
                                            &mut part_verts, def, 0.0, 0.0, nozzle_state,
                                        );
                                    }
                                }
                            }

                            // Apply part rotation and gimbal rotation for engine parts, then scale
                            // and rotate each vertex by vessel rotation.
                            // PRECISION: compute vertex as rel + (local + vert) not (rel + local) + vert.
                            // The inner sum (local + vert) stays near zero with full f32 precision.
                            // Adding rel (~0.006) last ensures adjacent part boundaries that share
                            // the same mathematical position round to the same f32 value.
                            let gimbal = if def.engine.is_some() {
                                part_data.gimbal_angle as f32
                            } else {
                                0.0
                            };
                            let part_rot = part_data.rotation as f32;
                            let base_index = all_vertices.len() as u32;
                            let scale_factor = render_scale;
                            for vert in &part_verts {
                                let mut vx = vert.position[0] * scale_factor;
                                let mut vy = vert.position[1] * scale_factor;
                                // Apply part rotation in part-local space
                                if part_rot.abs() > 1e-6 {
                                    let pc = part_rot.cos();
                                    let ps = part_rot.sin();
                                    let px = vx * pc - vy * ps;
                                    let py = vx * ps + vy * pc;
                                    vx = px;
                                    vy = py;
                                }
                                // Apply gimbal rotation in part-local space
                                if gimbal.abs() > 1e-6 {
                                    let gc = gimbal.cos();
                                    let gs = gimbal.sin();
                                    let gx = vx * gc - vy * gs;
                                    let gy = vx * gs + vy * gc;
                                    vx = gx;
                                    vy = gy;
                                }
                                // Rotate around origin by vessel rotation
                                let rx = vx * cos_r - vy * sin_r;
                                let ry = vx * sin_r + vy * cos_r;
                                // Apply per-part heat tinting (blackbody glow)
                                let color = apply_heat_tint(vert.color, part_data.temperature);
                                all_vertices.push(Vertex {
                                    position: [rel_x + (rotated_x + rx), rel_y + (rotated_y + ry)],
                                    color,
                                    uv: vert.uv,  // preserve sprite UVs
                                });
                            }

                            // Part vertices are triangle lists (every 3 verts = 1 triangle)
                            let num_part_verts = part_verts.len() as u32;
                            for i in (0..num_part_verts).step_by(3) {
                                if i + 2 < num_part_verts {
                                    all_indices.push(base_index + i);
                                    all_indices.push(base_index + i + 1);
                                    all_indices.push(base_index + i + 2);
                                }
                            }
                        }
                    }

                    // Second pass: draw decoupler adapter fairings
                    for part_data in parts {
                        if let Some(decoupler_def) = defs.get(&part_data.definition_id) {
                            if decoupler_def.decoupler.is_none() {
                                continue;
                            }

                            // Generate adapter vertices at origin using the same function
                            // We need to build a temporary parts map for the adapter check
                            let dec_x = part_data.local_x as f32;
                            let dec_y = part_data.local_y as f32;
                            let mut adapter_verts: Vec<Vertex> = Vec::new();
                            crate::editor::generate_flight_decoupler_adapter(
                                &mut adapter_verts, decoupler_def,
                                dec_x, dec_y, parts, defs, 1.0,
                            );

                            if !adapter_verts.is_empty() {
                                let base_index = all_vertices.len() as u32;
                                for vert in &adapter_verts {
                                    let vx = vert.position[0] * render_scale;
                                    let vy = vert.position[1] * render_scale;
                                    let rx = vx * cos_r - vy * sin_r;
                                    let ry = vx * sin_r + vy * cos_r;
                                    all_vertices.push(Vertex::new([rel_x + rx, rel_y + ry], vert.color));
                                }
                                let num_verts = adapter_verts.len() as u32;
                                for i in (0..num_verts).step_by(3) {
                                    if i + 2 < num_verts {
                                        all_indices.push(base_index + i);
                                        all_indices.push(base_index + i + 1);
                                        all_indices.push(base_index + i + 2);
                                    }
                                }
                            }
                        }
                    }

                    // Third pass: draw fairing shells
                    for part_data in parts {
                        let Some(ref shape) = part_data.fairing_shape else { continue };
                        let Some(fairing_def) = defs.get(&part_data.definition_id) else { continue };
                        if fairing_def.fairing.is_none() { continue; }

                        let px = part_data.local_x as f32;
                        let py = part_data.local_y as f32;
                        let hitbox_half_h = part_data.hitbox_half_h as f32;
                        let base_half_w = (fairing_def.width() / 2.0) as f32;
                        let mut shell_verts: Vec<Vertex> = Vec::new();
                        crate::editor::generate_flight_fairing_shell(
                            &mut shell_verts, shape,
                            px, py, hitbox_half_h, base_half_w, 1.0,
                            part_data.fairing_half,
                        );

                        if !shell_verts.is_empty() {
                            let base_index = all_vertices.len() as u32;
                            for vert in &shell_verts {
                                let vx = vert.position[0] * render_scale;
                                let vy = vert.position[1] * render_scale;
                                let rx = vx * cos_r - vy * sin_r;
                                let ry = vx * sin_r + vy * cos_r;
                                all_vertices.push(Vertex::new([rel_x + rx, rel_y + ry], vert.color));
                            }
                            let num_verts = shell_verts.len() as u32;
                            for i in (0..num_verts).step_by(3) {
                                if i + 2 < num_verts {
                                    all_indices.push(base_index + i);
                                    all_indices.push(base_index + i + 1);
                                    all_indices.push(base_index + i + 2);
                                }
                            }
                        }
                    }

                    // Fourth pass: draw deployed parachute canopies
                    {
                        // Retrograde direction in world frame
                        let vdir = ship_data.velocity_direction;
                        let vel_mag = (vdir[0] * vdir[0] + vdir[1] * vdir[1]).sqrt();
                        let (retro_world_x, retro_world_y) = if vel_mag > 0.1 {
                            (-vdir[0] as f32, -vdir[1] as f32)
                        } else {
                            let heading = ship_data.rotation as f32;
                            (-heading.cos(), -heading.sin())
                        };

                        // Convert retrograde from world frame to vessel-local frame
                        // (undo the visual_rotation so canopy directions are in local meter space)
                        let retro_local_x = retro_world_x * cos_r + retro_world_y * sin_r;
                        let retro_local_y = -retro_world_x * sin_r + retro_world_y * cos_r;

                        for part_data in parts {
                            if !part_data.is_parachute || part_data.parachute_deploy_fraction < 1e-6 {
                                continue;
                            }

                            // Anchor cables to dome top (bottom-aligned sprite, lowered 0.25 grid squares)
                            let anchor_local_x = part_data.local_x as f32;
                            let anchor_local_y = part_data.local_y as f32 - part_data.hitbox_half_h as f32 + (part_data.sprite_half_h * 2.0) as f32 - 0.125;

                            // Generate canopy in meter space relative to anchor (0,0)
                            let mut canopy_verts: Vec<Vertex> = Vec::new();
                            let visual_scale = if part_data.parachute_fully_deployed { 1.0 } else { 0.5 };
                            crate::editor::generate_parachute_canopy_vertices(
                                &mut canopy_verts,
                                retro_local_x, retro_local_y,
                                part_data.parachute_deployed_width_m,
                                part_data.parachute_deploy_fraction,
                                visual_scale,
                            );

                            if !canopy_verts.is_empty() {
                                let base_index = all_vertices.len() as u32;
                                // Transform from meter space to screen: offset by anchor, scale, rotate
                                for vert in &canopy_verts {
                                    let mx = (vert.position[0] + anchor_local_x) * render_scale;
                                    let my = (vert.position[1] + anchor_local_y) * render_scale;
                                    let rx = mx * cos_r - my * sin_r;
                                    let ry = mx * sin_r + my * cos_r;
                                    all_vertices.push(Vertex::new([rel_x + rx, rel_y + ry], vert.color));
                                }
                                let num_verts = canopy_verts.len() as u32;
                                for i in (0..num_verts).step_by(3) {
                                    if i + 2 < num_verts {
                                        all_indices.push(base_index + i);
                                        all_indices.push(base_index + i + 1);
                                        all_indices.push(base_index + i + 2);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Fallback: draw triangle when no parts available
                    let base_index = all_vertices.len() as u32;

                    // Apply heat tinting to ship color
                    let tri_color = apply_heat_tint(ship_data.color, ship_data.temperature);

                    let nose_angle = rotation;
                    let back_left_angle = rotation + std::f32::consts::PI * 0.8;
                    let back_right_angle = rotation - std::f32::consts::PI * 0.8;

                    all_vertices.push(Vertex::new([
                            rel_x + size * nose_angle.cos(),
                            rel_y + size * nose_angle.sin(),
                        ], tri_color));
                    all_vertices.push(Vertex::new([
                            rel_x + size * 0.6 * back_left_angle.cos(),
                            rel_y + size * 0.6 * back_left_angle.sin(),
                        ], tri_color));
                    all_vertices.push(Vertex::new([
                            rel_x + size * 0.6 * back_right_angle.cos(),
                            rel_y + size * 0.6 * back_right_angle.sin(),
                        ], tri_color));

                    all_indices.push(base_index);
                    all_indices.push(base_index + 1);
                    all_indices.push(base_index + 2);
                }
            }

            // Draw prograde direction arrow at screen edge in ship view
            if !needs_indicator {
                let vdir = ship_data.velocity_direction;
                let has_velocity = vdir[0] != 0.0 || vdir[1] != 0.0;
                if has_velocity {
                    let arrow_color = [1.0_f32, 1.0, 1.0, 0.85];
                    let vdx = vdir[0] as f32;
                    let vdy = vdir[1] as f32;

                    // Scale velocity direction to rendering coordinates
                    let scale_f = scale as f32;
                    let vdx_s = vdx * scale_f;
                    let vdy_s = vdy * scale_f;
                    let vmag = (vdx_s * vdx_s + vdy_s * vdy_s).sqrt();
                    let (vdx_n, vdy_n) = if vmag > 0.0 { (vdx_s / vmag, vdy_s / vmag) } else { (0.0, 1.0) };

                    // Asymmetric margins to keep arrow inside the flight viewport (outside GUI panels)
                    let margin_left = 60.0_f32;    // status panel (50px) + buffer
                    let margin_right = 220.0_f32;  // staging (150) + throttle (50) + buffer (20)
                    let margin_top = 40.0_f32;     // time warp panel + buffer
                    let margin_bottom = 80.0_f32;  // flight info panel + buffer

                    let screen_w = self.size.width as f32;
                    let screen_h = self.size.height as f32;
                    // Bounds relative to screen center
                    let bound_left = -(screen_w / 2.0 - margin_left);
                    let bound_right = screen_w / 2.0 - margin_right;
                    let bound_bottom = -(screen_h / 2.0 - margin_bottom);
                    let bound_top = screen_h / 2.0 - margin_top;

                    // Ship position in screen pixels relative to screen center
                    let ship_scr_x = rel_x * pixels_per_world_unit;
                    let ship_scr_y = rel_y * pixels_per_world_unit;

                    // Direction in screen pixels
                    let dir_scr_x = vdx_n * pixels_per_world_unit;
                    let dir_scr_y = vdy_n * pixels_per_world_unit;

                    // Ray-cast: find t where ship_scr + t*dir_scr hits the bounded viewport edge
                    let mut t = f32::MAX;
                    if dir_scr_x.abs() > 1e-6 {
                        let tx = if dir_scr_x > 0.0 { (bound_right - ship_scr_x) / dir_scr_x } else { (bound_left - ship_scr_x) / dir_scr_x };
                        if tx > 0.0 { t = t.min(tx); }
                    }
                    if dir_scr_y.abs() > 1e-6 {
                        let ty = if dir_scr_y > 0.0 { (bound_top - ship_scr_y) / dir_scr_y } else { (bound_bottom - ship_scr_y) / dir_scr_y };
                        if ty > 0.0 { t = t.min(ty); }
                    }
                    if t == f32::MAX { t = 1.0; }

                    // Arrow tip in world-rendering coords
                    let tip_x = rel_x + vdx_n * t;
                    let tip_y = rel_y + vdy_n * t;

                    // Fixed screen-size arrow: 80px head, 25px half-width (5x original)
                    let arrow_len = 80.0 / pixels_per_world_unit;
                    let half_width = 25.0 / pixels_per_world_unit;

                    // Stem dimensions: extends from arrow base toward the ship
                    let stem_length = 120.0 / pixels_per_world_unit;
                    let stem_half_width = 6.0 / pixels_per_world_unit;

                    // Perpendicular direction
                    let perp_x = -vdy_n;
                    let perp_y = vdx_n;

                    // Arrow base center (where head meets stem)
                    let base_cx = tip_x - vdx_n * arrow_len;
                    let base_cy = tip_y - vdy_n * arrow_len;

                    // Filled triangle head: tip + two base corners
                    let base_index = all_vertices.len() as u32;
                    all_vertices.push(Vertex::new([tip_x, tip_y], arrow_color));
                    all_vertices.push(Vertex::new([
                        base_cx + perp_x * half_width,
                        base_cy + perp_y * half_width,
                    ], arrow_color));
                    all_vertices.push(Vertex::new([
                        base_cx - perp_x * half_width,
                        base_cy - perp_y * half_width,
                    ], arrow_color));
                    all_indices.push(base_index);
                    all_indices.push(base_index + 1);
                    all_indices.push(base_index + 2);

                    // Stem: rectangle from arrow base toward the ship (two triangles)
                    let stem_end_x = base_cx - vdx_n * stem_length;
                    let stem_end_y = base_cy - vdy_n * stem_length;

                    let si = all_vertices.len() as u32;
                    // Four corners of the stem rectangle
                    all_vertices.push(Vertex::new([
                        base_cx + perp_x * stem_half_width,
                        base_cy + perp_y * stem_half_width,
                    ], arrow_color)); // si+0: base left
                    all_vertices.push(Vertex::new([
                        base_cx - perp_x * stem_half_width,
                        base_cy - perp_y * stem_half_width,
                    ], arrow_color)); // si+1: base right
                    all_vertices.push(Vertex::new([
                        stem_end_x - perp_x * stem_half_width,
                        stem_end_y - perp_y * stem_half_width,
                    ], arrow_color)); // si+2: end right
                    all_vertices.push(Vertex::new([
                        stem_end_x + perp_x * stem_half_width,
                        stem_end_y + perp_y * stem_half_width,
                    ], arrow_color)); // si+3: end left
                    all_indices.extend_from_slice(&[si, si + 1, si + 2, si, si + 2, si + 3]);
                }
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

                // Apply heat tinting to indicator color
                let indicator_color = apply_heat_tint(ship_data.color, ship_data.temperature);

                // Outer triangle (indicator)
                all_vertices.push(Vertex::new([
                        rel_x + indicator_size * nose_angle.cos(),
                        rel_y + indicator_size * nose_angle.sin(),
                    ], indicator_color));

                all_vertices.push(Vertex::new([
                        rel_x + indicator_size * 0.6 * back_left_angle.cos(),
                        rel_y + indicator_size * 0.6 * back_left_angle.sin(),
                    ], indicator_color));

                all_vertices.push(Vertex::new([
                        rel_x + indicator_size * 0.6 * back_right_angle.cos(),
                        rel_y + indicator_size * 0.6 * back_right_angle.sin(),
                    ], indicator_color));

                // Inner triangle (darker, for outline effect)
                let inner_size = indicator_size * 0.6;
                let inner_color = [
                    indicator_color[0] * 0.3,
                    indicator_color[1] * 0.3,
                    indicator_color[2] * 0.3,
                    indicator_color[3],
                ];

                all_vertices.push(Vertex::new([
                        rel_x + inner_size * nose_angle.cos(),
                        rel_y + inner_size * nose_angle.sin(),
                    ], inner_color));

                all_vertices.push(Vertex::new([
                        rel_x + inner_size * 0.6 * back_left_angle.cos(),
                        rel_y + inner_size * 0.6 * back_left_angle.sin(),
                    ], inner_color));

                all_vertices.push(Vertex::new([
                        rel_x + inner_size * 0.6 * back_right_angle.cos(),
                        rel_y + inner_size * 0.6 * back_right_angle.sin(),
                    ], inner_color));

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

        // Background vessels (tracking station, flight map view)
        if !background_vessels.is_empty() {
            let cam_x = self.camera.body_center[0];
            let cam_y = self.camera.body_center[1];
            let off_x = self.camera.ship_offset[0];
            let off_y = self.camera.ship_offset[1];
            let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;

            self.background_vessel_screen_positions.clear();

            for vessel in background_vessels {
                let rel_x = ((vessel.body_center[0] - cam_x) + (vessel.rel_offset[0] - off_x)) as f32;
                let rel_y = ((vessel.body_center[1] - cam_y) + (vessel.rel_offset[1] - off_y)) as f32;

                let has_parts = vessel.parts.is_some() && part_defs.is_some();

                // Estimate vessel size in pixels to decide if we need an indicator
                let vessel_size_world = if has_parts {
                    let parts = vessel.parts.as_ref().unwrap();
                    let max_extent = parts.iter()
                        .map(|p| (p.local_x.abs() + p.hitbox_half_h).max(p.local_y.abs() + p.hitbox_half_h))
                        .fold(0.0f64, f64::max);
                    (max_extent * 2.0 * scale) as f32
                } else {
                    0.0
                };
                let vessel_pixels = vessel_size_world * pixels_per_world_unit * 2.0;
                let needs_indicator = !has_parts || vessel_pixels < 5.0;

                if has_parts {
                    // Full part rendering for background vessels
                    let parts = vessel.parts.as_ref().unwrap();
                    let defs = part_defs.unwrap();
                    let render_scale = scale as f32;
                    let visual_rotation = vessel.rotation as f32 - std::f32::consts::FRAC_PI_2;
                    let cos_r = visual_rotation.cos();
                    let sin_r = visual_rotation.sin();

                    // First pass: parts
                    for part_data in parts {
                        if let Some(def) = defs.get(&part_data.definition_id) {
                            let local_x = part_data.local_x as f32 * render_scale;
                            let local_y = part_data.local_y as f32 * render_scale;
                            let rotated_x = local_x * cos_r - local_y * sin_r;
                            let rotated_y = local_x * sin_r + local_y * cos_r;
                            // Skip base disc for fairing half debris (shell-only)
                            let mut part_verts: Vec<Vertex> = Vec::new();
                            if part_data.fairing_half.is_none() {
                                crate::editor::generate_part_shape_vertices(
                                    &mut part_verts, def, 0.0, 0.0, 1.0,
                                    Some(&self.sprite_atlas),
                                    if part_data.is_solar_panel { Some(part_data.deploy_fraction) } else { None },
                                );
                            }

                            let base_index = all_vertices.len() as u32;
                            let scale_factor = render_scale;
                            let bg_part_rot = part_data.rotation as f32;
                            for vert in &part_verts {
                                let mut vx = vert.position[0] * scale_factor;
                                let mut vy = vert.position[1] * scale_factor;
                                // Apply part rotation
                                if bg_part_rot.abs() > 1e-6 {
                                    let pc = bg_part_rot.cos();
                                    let ps = bg_part_rot.sin();
                                    let px = vx * pc - vy * ps;
                                    let py = vx * ps + vy * pc;
                                    vx = px;
                                    vy = py;
                                }
                                let rx = vx * cos_r - vy * sin_r;
                                let ry = vx * sin_r + vy * cos_r;
                                all_vertices.push(Vertex {
                                    position: [rel_x + (rotated_x + rx), rel_y + (rotated_y + ry)],
                                    color: vert.color,
                                    uv: vert.uv,
                                });
                            }
                            let num_part_verts = part_verts.len() as u32;
                            for i in (0..num_part_verts).step_by(3) {
                                if i + 2 < num_part_verts {
                                    all_indices.push(base_index + i);
                                    all_indices.push(base_index + i + 1);
                                    all_indices.push(base_index + i + 2);
                                }
                            }
                        }
                    }

                    // Second pass: decoupler adapter fairings
                    for part_data in parts {
                        if let Some(decoupler_def) = defs.get(&part_data.definition_id) {
                            if decoupler_def.decoupler.is_none() {
                                continue;
                            }
                            let dec_x = part_data.local_x as f32;
                            let dec_y = part_data.local_y as f32;
                            let mut adapter_verts: Vec<Vertex> = Vec::new();
                            crate::editor::generate_flight_decoupler_adapter(
                                &mut adapter_verts, decoupler_def,
                                dec_x, dec_y, parts, defs, 1.0,
                            );
                            if !adapter_verts.is_empty() {
                                let base_index = all_vertices.len() as u32;
                                for vert in &adapter_verts {
                                    let vx = vert.position[0] * render_scale;
                                    let vy = vert.position[1] * render_scale;
                                    let rx = vx * cos_r - vy * sin_r;
                                    let ry = vx * sin_r + vy * cos_r;
                                    all_vertices.push(Vertex::new([rel_x + rx, rel_y + ry], vert.color));
                                }
                                let num_verts = adapter_verts.len() as u32;
                                for i in (0..num_verts).step_by(3) {
                                    if i + 2 < num_verts {
                                        all_indices.push(base_index + i);
                                        all_indices.push(base_index + i + 1);
                                        all_indices.push(base_index + i + 2);
                                    }
                                }
                            }
                        }
                    }

                    // Third pass: fairing shells
                    for part_data in parts {
                        let Some(ref shape) = part_data.fairing_shape else { continue };
                        let Some(fairing_def) = defs.get(&part_data.definition_id) else { continue };
                        if fairing_def.fairing.is_none() { continue; }

                        let px = part_data.local_x as f32;
                        let py = part_data.local_y as f32;
                        let hitbox_half_h = part_data.hitbox_half_h as f32;
                        let base_half_w = (fairing_def.width() / 2.0) as f32;
                        let mut shell_verts: Vec<Vertex> = Vec::new();
                        crate::editor::generate_flight_fairing_shell(
                            &mut shell_verts, shape,
                            px, py, hitbox_half_h, base_half_w, 1.0,
                            part_data.fairing_half,
                        );
                        if !shell_verts.is_empty() {
                            let base_index = all_vertices.len() as u32;
                            for vert in &shell_verts {
                                let vx = vert.position[0] * render_scale;
                                let vy = vert.position[1] * render_scale;
                                let rx = vx * cos_r - vy * sin_r;
                                let ry = vx * sin_r + vy * cos_r;
                                all_vertices.push(Vertex::new([rel_x + rx, rel_y + ry], vert.color));
                            }
                            let num_verts = shell_verts.len() as u32;
                            for i in (0..num_verts).step_by(3) {
                                if i + 2 < num_verts {
                                    all_indices.push(base_index + i);
                                    all_indices.push(base_index + i + 1);
                                    all_indices.push(base_index + i + 2);
                                }
                            }
                        }
                    }
                }

                // Triangle indicator when vessel is too small to see or has no parts
                if needs_indicator {
                    let icon_screen_size = 8.0f32;
                    let icon_world_size = icon_screen_size / pixels_per_world_unit;

                    let base_idx = all_vertices.len() as u32;
                    let (tri_verts, tri_idxs) = super::geometry::create_ship_triangle(
                        rel_x, rel_y,
                        icon_world_size,
                        std::f32::consts::FRAC_PI_2,
                        vessel.color,
                    );
                    for v in tri_verts {
                        all_vertices.push(v);
                    }
                    for idx in tri_idxs {
                        all_indices.push(base_idx + idx);
                    }
                }

                // Store screen position for click detection
                let ndc_x = rel_x * self.camera.zoom / self.camera.aspect_ratio;
                let ndc_y = rel_y * self.camera.zoom;
                let scale_factor = self.window.scale_factor() as f32;
                let screen_x = (ndc_x + 1.0) * 0.5 * self.size.width as f32 / scale_factor;
                let screen_y = (1.0 - ndc_y) * 0.5 * self.size.height as f32 / scale_factor;
                self.background_vessel_screen_positions.push((vessel.id, [screen_x, screen_y]));

                // Draw orbit line when this vessel's triangle indicator is showing
                if needs_indicator {
                  if let Some(ref orbit) = vessel.orbit {
                    let e = orbit.eccentricity;
                    if e < 1.0 && orbit.semi_major_axis > 0.0 {
                        let a = orbit.semi_major_axis;
                        let b = a * (1.0 - e * e).sqrt();
                        let c = a * e;
                        let arg_peri = orbit.argument_of_periapsis;
                        // Subtract camera from parent first for precision — both are galaxy-scale,
                        // their difference is solar-system-scale, so orbit geometry stays precise.
                        let pcam_x = orbit.parent_x - cam_x - off_x;
                        let pcam_y = orbit.parent_y - cam_y - off_y;
                        let center_x = pcam_x - c * arg_peri.cos();
                        let center_y = pcam_y - c * arg_peri.sin();

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
                            if len < 1e-20 { continue; }
                            let nx = -dy / len * line_width;
                            let ny = dx / len * line_width;

                            let next_px = center_x + next_rx;
                            let next_py = center_y + next_ry;
                            let base = all_vertices.len() as u32;
                            all_vertices.push(Vertex::new([(px + nx) as f32, (py + ny) as f32], orbit.color));
                            all_vertices.push(Vertex::new([(px - nx) as f32, (py - ny) as f32], orbit.color));
                            all_vertices.push(Vertex::new([(next_px - nx) as f32, (next_py - ny) as f32], orbit.color));
                            all_vertices.push(Vertex::new([(next_px + nx) as f32, (next_py + ny) as f32], orbit.color));
                            all_indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                        }
                    }
                  }
                }
            }
        }

        self.num_indices = all_indices.len() as u32;

        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&all_vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&all_indices));
    }

    /// Draw atmosphere rings around bodies that have atmospheres.
    fn add_atmosphere_vertices(
        &self,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)],
        scale: f64,
    ) {
        let cam_x = self.camera.body_center[0];
        let cam_y = self.camera.body_center[1];
        let off_x = self.camera.ship_offset[0];
        let off_y = self.camera.ship_offset[1];
        let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;

        // Atmosphere uses alpha channel to encode t (0=surface, 1=edge).
        // Fragment shader applies exp(-8*t) for non-linear falloff.
        for &(bx, by, radius, _, atmo_height, atmo_color, _) in bodies {
            if atmo_height <= 0.0 {
                continue;
            }

            // Negative alpha flags atmosphere for the shader's exp(-8*t) falloff.
            // Inner (surface): alpha = -1.0 (t=0), Outer (edge): alpha = -2.0 (t=1)
            let inner_color: [f32; 4] = [atmo_color[0], atmo_color[1], atmo_color[2], -1.0];
            let outer_color: [f32; 4] = [atmo_color[0], atmo_color[1], atmo_color[2], -2.0];

            let cx = bx * scale;
            let cy = by * scale;
            let r_inner = radius * scale;
            let r_outer = (radius + atmo_height) * scale;

            let outer_pixel_radius = r_outer as f32 * pixels_per_world_unit;
            if outer_pixel_radius < 1.0 {
                continue;
            }

            let circumference_pixels = 2.0 * std::f32::consts::PI * outer_pixel_radius;
            let raw_segments = (circumference_pixels / 3.0) as u32;

            if raw_segments <= 4096 {
                let segments = raw_segments.clamp(64, 4096) & !1;
                let base = all_vertices.len() as u32;

                for i in 0..segments {
                    let angle = (i as f64 / segments as f64) * std::f64::consts::TAU;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Subtract large values first to preserve precision at galaxy-scale distances
                    all_vertices.push(Vertex::new([((cx - cam_x - off_x) + r_inner * cos_a) as f32, ((cy - cam_y - off_y) + r_inner * sin_a) as f32], inner_color));
                    all_vertices.push(Vertex::new([((cx - cam_x - off_x) + r_outer * cos_a) as f32, ((cy - cam_y - off_y) + r_outer * sin_a) as f32], outer_color));
                }

                for i in 0..segments {
                    let i0 = base + i * 2;
                    let i1 = base + i * 2 + 1;
                    let i2 = base + ((i + 1) % segments) * 2;
                    let i3 = base + ((i + 1) % segments) * 2 + 1;

                    all_indices.push(i0);
                    all_indices.push(i2);
                    all_indices.push(i1);
                    all_indices.push(i1);
                    all_indices.push(i2);
                    all_indices.push(i3);
                }
            } else {
                let arc_segments = 4096u32;

                let dx = (cam_x + off_x) - cx;
                let dy = (cam_y + off_y) - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let cam_angle = dy.atan2(dx);

                let half_h = 1.0f64 / self.camera.zoom as f64;
                let half_w = self.camera.aspect_ratio as f64 * half_h;
                let view_diag = (half_w * half_w + half_h * half_h).sqrt();

                let visible_half = if dist > 1e-10 {
                    (view_diag / dist).min(1.0).asin()
                } else {
                    std::f64::consts::PI
                };
                let arc_half = visible_half.max(0.005 * std::f64::consts::TAU);

                let base = all_vertices.len() as u32;

                for i in 0..=arc_segments {
                    let t = i as f64 / arc_segments as f64;
                    let angle = cam_angle - arc_half + t * 2.0 * arc_half;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Subtract large values first to preserve precision at galaxy-scale distances
                    all_vertices.push(Vertex::new([((cx - cam_x - off_x) + r_inner * cos_a) as f32, ((cy - cam_y - off_y) + r_inner * sin_a) as f32], inner_color));
                    all_vertices.push(Vertex::new([((cx - cam_x - off_x) + r_outer * cos_a) as f32, ((cy - cam_y - off_y) + r_outer * sin_a) as f32], outer_color));
                }

                for i in 0..arc_segments {
                    let i0 = base + i * 2;
                    let i1 = base + i * 2 + 1;
                    let i2 = base + (i + 1) * 2;
                    let i3 = base + (i + 1) * 2 + 1;

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

    /// Draw accretion disc rings around bodies that have them (e.g., Sgr A*)
    fn add_accretion_disc_vertices(
        &self,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)],
        accretion_discs: &[Option<crate::bodies::AccretionDisc>],
        scale: f64,
    ) {
        let cam_x = self.camera.body_center[0];
        let cam_y = self.camera.body_center[1];
        let off_x = self.camera.ship_offset[0];
        let off_y = self.camera.ship_offset[1];
        let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;

        for &(bx, by, _radius, _, _, _, body_idx) in bodies {
            let disc = match accretion_discs.get(body_idx) {
                Some(Some(d)) => d,
                _ => continue,
            };

            let cx = bx * scale;
            let cy = by * scale;
            let r_inner = disc.inner_radius * scale;
            let r_outer = disc.outer_radius * scale;

            let outer_pixel_radius = r_outer as f32 * pixels_per_world_unit;
            if outer_pixel_radius < 1.0 {
                continue;
            }

            // Use concentric ring strips with color gradient and atmosphere-style fade
            let num_rings = 16u32;
            let circumference_pixels = 2.0 * std::f32::consts::PI * outer_pixel_radius;
            let segments = ((circumference_pixels / 3.0) as u32).clamp(64, 4096) & !1;

            for ring in 0..num_rings {
                let t0 = ring as f64 / num_rings as f64;
                let t1 = (ring + 1) as f64 / num_rings as f64;
                let ring_r_inner = r_inner + (r_outer - r_inner) * t0;
                let ring_r_outer = r_inner + (r_outer - r_inner) * t1;

                let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;

                // Non-linear color mix: t^1.5 produces white → orange → red transition
                let mix0 = (t0 as f32).powf(1.5);
                let mix1 = (t1 as f32).powf(1.5);

                // Brightness: exp(-6*t³) fades to the same blackness as atmospheres
                // (exp(-6) ≈ 0.0025 at the outer edge), but with a gentler initial
                // falloff so the white→orange→red gradient remains visible
                let bright0 = (-6.0_f32 * (t0 as f32).powi(3)).exp();
                let bright1 = (-6.0_f32 * (t1 as f32).powi(3)).exp();

                let color_inner = [
                    lerp(disc.color_inner[0], disc.color_outer[0], mix0) * bright0,
                    lerp(disc.color_inner[1], disc.color_outer[1], mix0) * bright0,
                    lerp(disc.color_inner[2], disc.color_outer[2], mix0) * bright0,
                    bright0,
                ];
                let color_outer = [
                    lerp(disc.color_inner[0], disc.color_outer[0], mix1) * bright1,
                    lerp(disc.color_inner[1], disc.color_outer[1], mix1) * bright1,
                    lerp(disc.color_inner[2], disc.color_outer[2], mix1) * bright1,
                    bright1,
                ];

                let base = all_vertices.len() as u32;

                for i in 0..segments {
                    let angle = (i as f64 / segments as f64) * std::f64::consts::TAU;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Subtract large values first to preserve precision at galaxy-scale distances
                    all_vertices.push(Vertex::new(
                        [((cx - cam_x - off_x) + ring_r_inner * cos_a) as f32, ((cy - cam_y - off_y) + ring_r_inner * sin_a) as f32],
                        color_inner,
                    ));
                    all_vertices.push(Vertex::new(
                        [((cx - cam_x - off_x) + ring_r_outer * cos_a) as f32, ((cy - cam_y - off_y) + ring_r_outer * sin_a) as f32],
                        color_outer,
                    ));
                }

                for i in 0..segments {
                    let i0 = base + i * 2;
                    let i1 = base + i * 2 + 1;
                    let i2 = base + ((i + 1) % segments) * 2;
                    let i3 = base + ((i + 1) % segments) * 2 + 1;

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

    /// Render galaxy star field with post-process blur.
    /// 1. Rasterize solid squares (raw RON colors) into a 200×200 CPU pixel buffer
    /// 2. Apply separable gaussian blur (5-tap kernel, 2 passes)
    /// 3. Emit blurred pixels as vertex-colored quads
    fn add_galaxy_texture_quad(
        &self,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        in_galaxy_view: bool,
        scale: f64,
    ) {
        if !in_galaxy_view {
            return;
        }
        let layer = match self.body_texture_map.galaxy_layer {
            Some(l) => l,
            None => return,
        };

        // Galaxy image spans 100,000 ly centered on Sgr A* (body 0 at origin)
        let half = 60_000.0 * crate::bodies::LIGHT_YEAR * scale;
        let cam_x = self.camera.body_center[0];
        let cam_y = self.camera.body_center[1];
        let off_x = self.camera.ship_offset[0];
        let off_y = self.camera.ship_offset[1];

        let x0 = (-half - cam_x - off_x) as f32;
        let x1 = (half - cam_x - off_x) as f32;
        let y0 = (-half - cam_y - off_y) as f32;
        let y1 = (half - cam_y - off_y) as f32;

        let base = all_vertices.len() as u32;

        // UV: y-flip so image top maps to +y world. Use epsilon to avoid (0,0) which
        // the shader treats as "no texture" (solid color fallback).
        let e = 0.001;
        all_vertices.push(Vertex::textured([x0, y0], [e, 1.0], layer));
        all_vertices.push(Vertex::textured([x1, y0], [1.0, 1.0], layer));
        all_vertices.push(Vertex::textured([x1, y1], [1.0, e], layer));
        all_vertices.push(Vertex::textured([x0, y1], [e, e], layer));

        all_indices.push(base);
        all_indices.push(base + 1);
        all_indices.push(base + 2);

        all_indices.push(base);
        all_indices.push(base + 2);
        all_indices.push(base + 3);
    }

    /// Helper to add body vertices (extracted for reuse)
    fn add_body_vertices(
        &mut self,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)],
        scale: f64,
    ) {
        // Store body data for hit testing
        self.bodies.clear();

        // Calculate world units per pixel for indicator sizing
        let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;
        let indicator_screen_radius = 16.0f32;
        let indicator_world_radius = (indicator_screen_radius / pixels_per_world_unit) as f64;
        let min_body_pixels = 5.0f32;

        let cam_x = self.camera.body_center[0];
        let cam_y = self.camera.body_center[1];
        let off_x = self.camera.ship_offset[0];
        let off_y = self.camera.ship_offset[1];

        for (x, y, radius, color, _atmo_height, _atmo_color, body_idx) in bodies {
            let rel_x = ((*x * scale) - cam_x - off_x) as f32;
            let rel_y = ((*y * scale) - cam_y - off_y) as f32;
            let r = (*radius * scale) as f32;

            let cx = *x * scale;
            let cy = *y * scale;
            let r_f64 = *radius * scale;

            // Bodies with radius=0 are hidden (e.g., planets/moons in galaxy view)
            // Push empty BodyData to keep indices aligned but skip rendering/hit testing
            if *radius <= 0.0 {
                self.bodies.push(BodyData {
                    x: cx,
                    y: cy,
                    radius: 0.0,
                    indicator_radius: 0.0,
                });
                continue;
            }

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
                let texture_layer = self.body_texture_map.layer_for_body(*body_idx);

                let draw_pixel_radius = draw_r * pixels_per_world_unit;
                let circumference_pixels = 2.0 * std::f32::consts::PI * draw_pixel_radius;
                let raw_segments = (circumference_pixels / 3.0) as u32;

                if raw_segments <= 4096 {
                    // Full circle: polygon triangle fan
                    let segments = raw_segments.clamp(64, 4096) & !1;

                    if let Some(layer) = texture_layer {
                        all_vertices.push(Vertex::textured([rel_x, rel_y], [0.5, 0.5], layer));
                        for i in 0..segments {
                            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                            let u = 0.5 + 0.5 * angle.cos();
                            let v = 0.5 - 0.5 * angle.sin();
                            all_vertices.push(Vertex::textured(
                                [rel_x + draw_r * angle.cos(), rel_y + draw_r * angle.sin()],
                                [u, v],
                                layer,
                            ));
                        }
                    } else {
                        all_vertices.push(Vertex::new([rel_x, rel_y], *color));
                        for i in 0..segments {
                            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                            all_vertices.push(Vertex::new([rel_x + draw_r * angle.cos(), rel_y + draw_r * angle.sin()], *color));
                        }
                    }

                    for i in 0..segments {
                        all_indices.push(base_index);
                        all_indices.push(base_index + i + 1);
                        all_indices.push(base_index + ((i + 1) % segments) + 1);
                    }
                } else {
                    // Arc mode: 4096 segments on the visible ~1% of circumference
                    let arc_segments = 4096u32;

                    // Direction from body center to camera (f64 precision)
                    let dx = (cam_x + off_x) - cx;
                    let dy = (cam_y + off_y) - cy;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let cam_angle = dy.atan2(dx);

                    // Viewport diagonal in world units
                    let half_h = 1.0f64 / self.camera.zoom as f64;
                    let half_w = self.camera.aspect_ratio as f64 * half_h;
                    let view_diag = (half_w * half_w + half_h * half_h).sqrt();

                    // Half-angle of visible arc from body center
                    let visible_half = if dist > 1e-10 {
                        (view_diag / dist).min(1.0).asin()
                    } else {
                        std::f64::consts::PI
                    };

                    // At least 1% of circumference (0.5% each side)
                    let arc_half = visible_half.max(0.005 * std::f64::consts::TAU);

                    if let Some(layer) = texture_layer {
                        // Center vertex with UV center
                        all_vertices.push(Vertex::textured([rel_x, rel_y], [0.5, 0.5], layer));

                        for i in 0..=arc_segments {
                            let t = i as f64 / arc_segments as f64;
                            let angle = cam_angle - arc_half + t * 2.0 * arc_half;
                            // Subtract large values first to preserve precision at galaxy-scale distances
                            let vx = (cx - cam_x - off_x) + r_f64 * angle.cos();
                            let vy = (cy - cam_y - off_y) + r_f64 * angle.sin();
                            let u = 0.5 + 0.5 * (angle.cos() as f32);
                            let v = 0.5 - 0.5 * (angle.sin() as f32);
                            all_vertices.push(Vertex::textured(
                                [vx as f32, vy as f32],
                                [u, v],
                                layer,
                            ));
                        }
                    } else {
                        all_vertices.push(Vertex::new([rel_x, rel_y], *color));

                        for i in 0..=arc_segments {
                            let t = i as f64 / arc_segments as f64;
                            let angle = cam_angle - arc_half + t * 2.0 * arc_half;
                            // Subtract large values first to preserve precision at galaxy-scale distances
                            let vx = (cx - cam_x - off_x) + r_f64 * angle.cos();
                            let vy = (cy - cam_y - off_y) + r_f64 * angle.sin();
                            all_vertices.push(Vertex::new([vx as f32, vy as f32], *color));
                        }
                    }

                    // Triangle fan
                    for i in 0..arc_segments {
                        all_indices.push(base_index);
                        all_indices.push(base_index + 1 + i);
                        all_indices.push(base_index + 2 + i);
                    }
                }
            }

            if needs_indicator {
                let base_index = all_vertices.len() as u32;
                let ring_outer = indicator_world_radius as f32;
                let ring_inner = (indicator_world_radius * 0.7) as f32;
                let ring_segments = 64u32;

                // Use body color for ring, but apply brightness floor for very dark bodies (e.g., black holes)
                let ring_color = if color[0] + color[1] + color[2] < 0.1 {
                    [0.7, 0.5, 0.3, 1.0] // Warm amber fallback
                } else {
                    *color
                };

                for i in 0..ring_segments {
                    let angle = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    all_vertices.push(Vertex::new([rel_x + ring_outer * cos_a, rel_y + ring_outer * sin_a], ring_color));
                    all_vertices.push(Vertex::new([rel_x + ring_inner * cos_a, rel_y + ring_inner * sin_a], [ring_color[0] * 0.3, ring_color[1] * 0.3, ring_color[2] * 0.3, ring_color[3] * 0.5]));
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

    /// Draw the launchpad on Earth's surface when in ship view.
    fn add_launchpad_vertices(
        &self,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)],
        scale: f64,
        ship: &ShipRenderData,
    ) {
        use crate::game::{LAUNCHPAD_BODY_INDEX, LAUNCHPAD_SURFACE_ANGLE,
                          LAUNCHPAD_HEIGHT, LAUNCHPAD_TOP_WIDTH, LAUNCHPAD_BOTTOM_WIDTH};
        let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;
        let ship_pixels = ship.size as f32 * pixels_per_world_unit * 2.0;

        // Only draw launchpad in ship view
        if ship_pixels < 5.0 {
            return;
        }

        // Find the launchpad body
        let Some((bx, by, radius, _, _, _, _)) = bodies.get(LAUNCHPAD_BODY_INDEX) else { return };

        // Compute body center relative to camera in f64 first, preserving precision.
        // At galaxy-scale distances (body position ~2.46e11 world units), the launchpad
        // dimensions (~6e-8 world units) are below f64 ULP if computed in absolute coords.
        // By subtracting body_center and ship_offset first, we keep all values near zero.
        let cam_x = self.camera.body_center[0];
        let cam_y = self.camera.body_center[1];
        let off_x = self.camera.ship_offset[0];
        let off_y = self.camera.ship_offset[1];
        let rel_cx = bx * scale - cam_x - off_x;
        let rel_cy = by * scale - cam_y - off_y;
        let r = radius * scale;

        let lp_angle = LAUNCHPAD_SURFACE_ANGLE;
        let lp_height = LAUNCHPAD_HEIGHT * scale;
        let lp_top_half = (LAUNCHPAD_TOP_WIDTH * 0.5) * scale;
        let lp_bot_half = (LAUNCHPAD_BOTTOM_WIDTH * 0.5) * scale;

        // Surface point at launchpad center (relative to camera)
        let sx = rel_cx + r * lp_angle.cos();
        let sy = rel_cy + r * lp_angle.sin();

        // Radial outward and tangent directions
        let rad_x = lp_angle.cos();
        let rad_y = lp_angle.sin();
        let tan_x = -rad_y;
        let tan_y = rad_x;

        // 4 corners relative to camera
        let bl_x = sx - lp_bot_half * tan_x;
        let bl_y = sy - lp_bot_half * tan_y;
        let br_x = sx + lp_bot_half * tan_x;
        let br_y = sy + lp_bot_half * tan_y;
        let tl_x = sx - lp_top_half * tan_x + lp_height * rad_x;
        let tl_y = sy - lp_top_half * tan_y + lp_height * rad_y;
        let tr_x = sx + lp_top_half * tan_x + lp_height * rad_x;
        let tr_y = sy + lp_top_half * tan_y + lp_height * rad_y;

        let lp_color: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
        let base = all_vertices.len() as u32;

        all_vertices.push(Vertex::new([bl_x as f32, bl_y as f32], lp_color));
        all_vertices.push(Vertex::new([br_x as f32, br_y as f32], lp_color));
        all_vertices.push(Vertex::new([tl_x as f32, tl_y as f32], lp_color));
        all_vertices.push(Vertex::new([tr_x as f32, tr_y as f32], lp_color));

        // Two triangles for the trapezoid
        all_indices.push(base);
        all_indices.push(base + 1);
        all_indices.push(base + 2);
        all_indices.push(base + 1);
        all_indices.push(base + 3);
        all_indices.push(base + 2);
    }

    /// Update geometry with multiple bodies (legacy method without orbits)
    /// scale: world units per meter (e.g., 1e-9 means 1 billion meters = 1 world unit)
    pub fn update_bodies(&mut self, bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)], scale: f64) {
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
        let cam_x = self.camera.body_center[0];
        let cam_y = self.camera.body_center[1];
        let off_x = self.camera.ship_offset[0];
        let off_y = self.camera.ship_offset[1];

        for (x, y, radius, color, _atmo_height, _atmo_color, body_idx) in bodies {
            // Calculate position relative to camera in f64 first, then convert to f32
            // This preserves precision for small bodies far from origin
            let rel_x = ((*x * scale) - cam_x - off_x) as f32;
            let rel_y = ((*y * scale) - cam_y - off_y) as f32;
            let r = (*radius * scale) as f32;

            // For hit testing, we still need absolute world coordinates (f64 for precision)
            let cx = *x * scale;
            let cy = *y * scale;
            let r_f64 = *radius * scale;

            // Bodies with radius=0 are hidden (e.g., planets/moons in galaxy view)
            // Push empty BodyData to keep indices aligned but skip rendering/hit testing
            if *radius <= 0.0 {
                self.bodies.push(BodyData {
                    x: cx,
                    y: cy,
                    radius: 0.0,
                    indicator_radius: 0.0,
                });
                continue;
            }

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
                let draw_r = r;
                let texture_layer = self.body_texture_map.layer_for_body(*body_idx);
                let draw_pixel_radius = draw_r * pixels_per_world_unit;
                let circumference_pixels = 2.0 * std::f32::consts::PI * draw_pixel_radius;
                let raw_segments = (circumference_pixels / 3.0) as u32;

                if raw_segments <= 16384 {
                    // Full circle - planet small enough on screen
                    let segments = if texture_layer.is_some() { 64u32.max(raw_segments.min(256)) } else { 4u32 };

                    if let Some(layer) = texture_layer {
                        all_vertices.push(Vertex::textured([rel_x, rel_y], [0.5, 0.5], layer));
                        for i in 0..segments {
                            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                            let u = 0.5 + 0.5 * angle.cos();
                            let v = 0.5 - 0.5 * angle.sin();
                            all_vertices.push(Vertex::textured(
                                [rel_x + draw_r * angle.cos(), rel_y + draw_r * angle.sin()],
                                [u, v],
                                layer,
                            ));
                        }
                    } else {
                        all_vertices.push(Vertex::new([rel_x, rel_y], *color));
                        for i in 0..segments {
                            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                            all_vertices.push(Vertex::new([rel_x + draw_r * angle.cos(), rel_y + draw_r * angle.sin()], *color));
                        }
                    }

                    for i in 0..segments {
                        all_indices.push(base_index);
                        all_indices.push(base_index + i + 1);
                        all_indices.push(base_index + ((i + 1) % segments) + 1);
                    }
                } else {
                    // Zoomed in close: segments over just the visible arc
                    let dist = (rel_x * rel_x + rel_y * rel_y).sqrt();
                    let cam_angle = (-rel_y).atan2(-rel_x);

                    let aspect = self.size.width as f32 / self.size.height as f32;
                    let half_h = 1.0 / self.camera.zoom;
                    let half_w = aspect * half_h;
                    let viewport_diag = (half_w * half_w + half_h * half_h).sqrt();

                    let half_angle = if dist > 1e-6 {
                        ((viewport_diag / dist).min(1.0)).asin() * 1.5
                    } else {
                        std::f32::consts::PI
                    };
                    let half_angle = half_angle.min(std::f32::consts::PI);

                    let arc_segments = if texture_layer.is_some() { 256u32 } else { 4u32 };

                    if let Some(layer) = texture_layer {
                        all_vertices.push(Vertex::textured([rel_x, rel_y], [0.5, 0.5], layer));
                        for i in 0..=arc_segments {
                            let t = i as f32 / arc_segments as f32;
                            let angle = cam_angle - half_angle + t * 2.0 * half_angle;
                            let u = 0.5 + 0.5 * angle.cos();
                            let v = 0.5 - 0.5 * angle.sin();
                            all_vertices.push(Vertex::textured(
                                [rel_x + draw_r * angle.cos(), rel_y + draw_r * angle.sin()],
                                [u, v],
                                layer,
                            ));
                        }
                    } else {
                        all_vertices.push(Vertex::new([rel_x, rel_y], *color));
                        for i in 0..=arc_segments {
                            let t = i as f32 / arc_segments as f32;
                            let angle = cam_angle - half_angle + t * 2.0 * half_angle;
                            all_vertices.push(Vertex::new([rel_x + draw_r * angle.cos(), rel_y + draw_r * angle.sin()], *color));
                        }
                    }

                    for i in 0..arc_segments {
                        all_indices.push(base_index);
                        all_indices.push(base_index + 1 + i);
                        all_indices.push(base_index + 2 + i);
                    }
                }
            }

            // Draw indicator ring if body is too small
            if needs_indicator {
                let base_index = all_vertices.len() as u32;
                // Cast to f32 for vertex positions
                let ring_outer = indicator_world_radius as f32;
                let ring_inner = (indicator_world_radius * 0.7) as f32;
                let ring_segments = 4u32; // Smooth indicator rings

                // Use body color for ring, but apply brightness floor for very dark bodies (e.g., black holes)
                let ring_color = if color[0] + color[1] + color[2] < 0.1 {
                    [0.7, 0.5, 0.3, 1.0] // Warm amber fallback
                } else {
                    *color
                };

                // Create ring vertices (inner and outer circles, relative to camera)
                for i in 0..ring_segments {
                    let angle = (i as f32 / ring_segments as f32) * std::f32::consts::TAU;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    // Outer vertex
                    all_vertices.push(Vertex::new([rel_x + ring_outer * cos_a, rel_y + ring_outer * sin_a], ring_color));
                    // Inner vertex
                    all_vertices.push(Vertex::new([rel_x + ring_inner * cos_a, rel_y + ring_inner * sin_a], [ring_color[0] * 0.3, ring_color[1] * 0.3, ring_color[2] * 0.3, ring_color[3] * 0.5]));
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
            // Skip hidden bodies (radius=0, indicator_radius=0)
            if body.radius <= 0.0 && body.indicator_radius <= 0.0 {
                continue;
            }

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
            // Skip hidden bodies (radius=0, indicator_radius=0)
            if body.radius <= 0.0 && body.indicator_radius <= 0.0 {
                continue;
            }

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

    /// Find background vessel at screen position, returns vessel ID if within click range (20px)
    pub fn background_vessel_at_screen_pos(&self, screen_x: f32, screen_y: f32) -> Option<u64> {
        let threshold = 20.0f32;
        let mut closest: Option<(u64, f32)> = None;

        for &(id, pos) in &self.background_vessel_screen_positions {
            let dx = screen_x - pos[0];
            let dy = screen_y - pos[1];
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < threshold {
                if closest.is_none() || dist < closest.unwrap().1 {
                    closest = Some((id, dist));
                }
            }
        }

        closest.map(|(id, _)| id)
    }

    /// Focus camera on a body by index and start tracking it
    pub fn focus_on_body(&mut self, index: usize) {
        if let Some(body) = self.bodies.get(index) {
            self.camera.focus_on([body.x, body.y]); // Both are now f64
            self.tracked_body = Some(index);
            self.tracked_vessel = None; // Stop tracking any vessel
        }
    }

    /// Update camera to follow tracked body using current positions
    pub fn update_tracking(&mut self, positions: &[[f64; 2]], scale: f64) {
        if let Some(index) = self.tracked_body {
            if let Some(pos) = positions.get(index) {
                // Set camera position directly to body position (in f64 for precision)
                self.camera.position[0] = pos[0] * scale;
                self.camera.position[1] = pos[1] * scale;
                // When tracking a body (not ship), camera is at body center with no offset
                self.camera.body_center = self.camera.position;
                self.camera.ship_offset = [0.0, 0.0];
            }
        }
    }

    /// Render the editor scene
    pub fn render_editor(
        &mut self,
        vertices: &[Vertex],
        egui_callback: impl FnOnce(&egui::Context),
    ) -> Result<(), wgpu::SurfaceError> {
        // Update camera buffer before rendering
        self.update_camera_buffer();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build egui UI
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, egui_callback);

        self.egui_state.handle_platform_output(&self.window, full_output.platform_output);

        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        // Create LOCAL buffers for editor - don't modify the shared buffers
        let editor_vertex_buffer;
        let editor_index_buffer;
        let editor_num_indices;

        if !vertices.is_empty() {
            editor_vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Editor Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let indices: Vec<u32> = (0..vertices.len() as u32).collect();
            editor_index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Editor Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            editor_num_indices = indices.len() as u32;
        } else {
            // Empty buffers
            editor_vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Editor Vertex Buffer Empty"),
                size: 64,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            });
            editor_index_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Editor Index Buffer Empty"),
                size: 64,
                usage: wgpu::BufferUsages::INDEX,
                mapped_at_creation: false,
            });
            editor_num_indices = 0;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Editor Render Encoder"),
            });

        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_descriptor);

        // Render editor geometry
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Editor Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if editor_num_indices > 0 {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.body_texture_bind_group, &[]);
                render_pass.set_bind_group(2, &self.sprite_atlas.bind_group, &[]);
                render_pass.set_vertex_buffer(0, editor_vertex_buffer.slice(..));
                render_pass.set_index_buffer(editor_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..editor_num_indices, 0, 0..1);
            }
        }

        // Render egui
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Editor Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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

        Ok(())
    }

    /// Update the editor camera for rendering
    pub fn set_editor_camera(&mut self, offset: [f64; 2], zoom: f32) {
        self.camera.position = offset;
        self.camera.zoom = zoom;
        self.camera.rotation = 0.0;
        self.update_camera_buffer();
    }

    /// Get the egui context for direct UI access
    pub fn egui_context(&self) -> &egui::Context {
        &self.egui_ctx
    }

    /// Render the main menu: planets + centered menu buttons + time warp panel.
    /// Returns a MainMenuAction if the user clicked a button.
    pub fn render_main_menu(
        &mut self,
        warp_levels: &[f64],
        current_warp_index: usize,
        date_str: &str,
        egui_callback: impl FnOnce(&egui::Context) -> MainMenuAction,
    ) -> Result<(usize, MainMenuAction), wgpu::SurfaceError> {
        self.update_camera_buffer();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut menu_action = MainMenuAction::None;
        let mut new_warp_index = current_warp_index;

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Time warp panel at top
            egui::TopBottomPanel::top("time_warp_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Time Warp:");
                    for (i, &warp) in warp_levels.iter().enumerate() {
                        let label = if warp >= 1_000_000_000.0 {
                            format!("{}B", (warp / 1_000_000_000.0) as i32)
                        } else if warp >= 1_000_000.0 {
                            format!("{}M", (warp / 1_000_000.0) as i32)
                        } else if warp >= 1000.0 {
                            format!("{}K", (warp / 1000.0) as i32)
                        } else {
                            format!("{}x", warp as i32)
                        };
                        let is_selected = i == current_warp_index;
                        if ui.selectable_label(is_selected, &label).clicked() {
                            new_warp_index = i;
                        }
                    }
                    ui.separator();
                    let current_warp = warp_levels[current_warp_index];
                    ui.label(format!("Current: {}x", current_warp as i64));

                    ui.separator();
                    ui.label(date_str);
                });
            });

            menu_action = egui_callback(ctx);
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

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main Menu Render Encoder"),
        });
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_descriptor);

        // Geometry pass (planets/orbits already in self.vertex_buffer)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Menu Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.body_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.sprite_atlas.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Egui pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Menu Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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

        Ok((new_warp_index, menu_action))
    }

    /// Render the title screen: planets + egui overlay (no time warp bar).
    pub fn render_title_screen(
        &mut self,
        egui_callback: impl FnOnce(&egui::Context) -> TitleScreenAction,
    ) -> Result<TitleScreenAction, wgpu::SurfaceError> {
        self.update_camera_buffer();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut action = TitleScreenAction::None;

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // No time warp panel — just the callback
            action = egui_callback(ctx);
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

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Title Screen Render Encoder"),
        });
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_descriptor);

        // Geometry pass (planets/orbits already in self.vertex_buffer)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Title Screen Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.body_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.sprite_atlas.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Egui pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Title Screen Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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

        Ok(action)
    }

    /// Render tracking station: planets + time warp panel + vessel list.
    /// Returns the new warp index, pause action, and tracking station action.
    pub fn render_tracking_station(
        &mut self,
        body_names: &[String],
        warp_levels: &[f64],
        current_warp_index: usize,
        paused: bool,
        date_str: &str,
        vessels: &[TrackingVesselData],
        active_vessel_id: u64,
        body_info: &[BodyInfoData],
    ) -> Result<(usize, PauseAction, TrackingStationAction), wgpu::SurfaceError> {
        self.update_camera_buffer();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut new_warp_index = current_warp_index;
        let mut pause_action = PauseAction::None;
        let mut ts_action = TrackingStationAction::None;
        let hovered = self.hovered_body;
        let bodies_copy = self.bodies.clone();
        let size = self.size;
        let camera_pos = self.camera.position;
        let camera_zoom = self.camera.zoom;
        let camera_rotation = self.camera.rotation;
        let aspect_ratio = self.camera.aspect_ratio;
        let scale_factor = self.window.scale_factor() as f32;

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Time warp panel at top
            egui::TopBottomPanel::top("time_warp_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Time Warp:");
                    for (i, &warp) in warp_levels.iter().enumerate() {
                        let label = if warp >= 1_000_000_000.0 {
                            format!("{}B", (warp / 1_000_000_000.0) as i32)
                        } else if warp >= 1_000_000.0 {
                            format!("{}M", (warp / 1_000_000.0) as i32)
                        } else if warp >= 1000.0 {
                            format!("{}K", (warp / 1000.0) as i32)
                        } else {
                            format!("{}x", warp as i32)
                        };
                        let is_selected = i == current_warp_index;
                        if ui.selectable_label(is_selected, &label).clicked() {
                            new_warp_index = i;
                        }
                    }
                    ui.separator();
                    let current_warp = warp_levels[current_warp_index];
                    ui.label(format!("Current: {}x", current_warp as i64));

                    ui.separator();
                    ui.label(date_str);
                });
            });

            // Vessels sidebar
            if !vessels.is_empty() {
                egui::SidePanel::left("vessels_panel")
                    .default_width(180.0)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.heading(egui::RichText::new("Vessels").size(16.0).color(egui::Color32::WHITE));
                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for vessel in vessels {
                                let is_active = vessel.id == active_vessel_id;
                                let body_name = body_names.get(vessel.soi_body)
                                    .cloned()
                                    .unwrap_or_else(|| "Unknown".to_string());

                                ui.horizontal(|ui| {
                                    // Color indicator
                                    let color = egui::Color32::from_rgba_unmultiplied(
                                        (vessel.color[0] * 255.0) as u8,
                                        (vessel.color[1] * 255.0) as u8,
                                        (vessel.color[2] * 255.0) as u8,
                                        255,
                                    );
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(8.0, 8.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(rect.center(), 4.0, color);

                                    ui.vertical(|ui| {
                                        let name_text = if is_active {
                                            egui::RichText::new(&vessel.name)
                                                .color(egui::Color32::from_rgb(100, 255, 100))
                                                .size(13.0)
                                        } else if vessel.is_debris {
                                            egui::RichText::new(&vessel.name)
                                                .color(egui::Color32::from_rgb(140, 140, 140))
                                                .size(13.0)
                                        } else {
                                            egui::RichText::new(&vessel.name)
                                                .color(egui::Color32::WHITE)
                                                .size(13.0)
                                        };
                                        if ui.add(egui::Label::new(name_text).sense(egui::Sense::click())).clicked() {
                                            ts_action = TrackingStationAction::FocusVessel(vessel.id);
                                        }
                                        ui.label(egui::RichText::new(format!("SOI: {}", body_name))
                                            .size(11.0)
                                            .color(egui::Color32::GRAY));
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if !vessel.is_debris {
                                            if ui.small_button("Fly").clicked() {
                                                ts_action = TrackingStationAction::FlyVessel(vessel.id);
                                            }
                                        }
                                        let delete_btn = egui::Button::new(
                                            egui::RichText::new("X").color(egui::Color32::from_rgb(200, 80, 80))
                                        ).small();
                                        if ui.add(delete_btn).on_hover_text("Delete vessel").clicked() {
                                            ts_action = TrackingStationAction::DeleteVessel(vessel.id);
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        });
                    });
            }

            // Body info right panel (shown when a body is tracked)
            if let Some(idx) = self.tracked_body {
                if let Some(info) = body_info.get(idx) {
                    egui::SidePanel::right("body_info_panel")
                        .default_width(220.0)
                        .resizable(false)
                        .show(ctx, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.heading(egui::RichText::new(&info.name).size(18.0).color(egui::Color32::WHITE));
                                if !info.description.is_empty() {
                                    ui.label(egui::RichText::new(&info.description)
                                        .size(12.0)
                                        .italics()
                                        .color(egui::Color32::from_rgb(160, 160, 160)));
                                }
                                ui.separator();

                                // Physical properties
                                ui.label(egui::RichText::new("Physical Properties").size(13.0).color(egui::Color32::from_rgb(200, 200, 200)));
                                ui.add_space(2.0);
                                ui.label(format!("Radius: {}", format_distance(info.radius_m)));
                                ui.label(format!("Surface gravity: {:.2} m/s\u{b2}", info.surface_gravity_ms2));
                                ui.label(format!("Mass: {}", format_mass(info.mass_kg)));

                                // Atmosphere
                                ui.add_space(4.0);
                                if let Some(pressure) = info.atmosphere_pressure_pa {
                                    ui.label(egui::RichText::new("Atmosphere").size(13.0).color(egui::Color32::from_rgb(200, 200, 200)));
                                    ui.add_space(2.0);
                                    ui.label(format!("Pressure: {}", format_pressure(pressure)));
                                    if let Some(height) = info.atmosphere_height_m {
                                        ui.label(format!("Height: {}", format_distance(height)));
                                    }
                                } else {
                                    ui.label(egui::RichText::new("No atmosphere")
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(120, 120, 120)));
                                }

                                // Orbit (skip for root body / Sun)
                                if let Some(sma) = info.orbit_semi_major_axis_m {
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.label(egui::RichText::new("Orbit").size(13.0).color(egui::Color32::from_rgb(200, 200, 200)));
                                    ui.add_space(2.0);
                                    ui.label(format!("Semi-major axis: {}", format_distance(sma)));
                                    if let Some(ecc) = info.orbit_eccentricity {
                                        ui.label(format!("Eccentricity: {:.4}", ecc));
                                    }
                                    if let Some(period) = info.orbit_period_s {
                                        ui.label(format!("Period: {}", format_duration(period)));
                                    }
                                }
                            });
                        });
                }
            }

            // Bottom panel: Tracking Station label
            egui::TopBottomPanel::bottom("tracking_station_label").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(egui::RichText::new("Tracking Station").size(14.0).color(egui::Color32::from_rgb(180, 180, 180)));
                });
            });

            // Hovered body labels (same logic as flight)
            if let Some(idx) = hovered {
                if let Some(body) = bodies_copy.get(idx) {
                    if let Some(name) = body_names.get(idx) {
                        let rel_x = (body.x - camera_pos[0]) as f32;
                        let rel_y = (body.y - camera_pos[1]) as f32;
                        let cos_r = camera_rotation.cos();
                        let sin_r = camera_rotation.sin();
                        let rot_x = rel_x * cos_r - rel_y * sin_r;
                        let rot_y = rel_x * sin_r + rel_y * cos_r;
                        let ndc_x = rot_x * camera_zoom / aspect_ratio;
                        let ndc_y = rot_y * camera_zoom;
                        let screen_x = (ndc_x + 1.0) * 0.5 * size.width as f32 / scale_factor;
                        let screen_y = (1.0 - ndc_y) * 0.5 * size.height as f32 / scale_factor;
                        let label_y = screen_y - 20.0;

                        egui::Area::new(egui::Id::new("body_label"))
                            .fixed_pos(egui::pos2(screen_x, label_y))
                            .pivot(egui::Align2::CENTER_BOTTOM)
                            .show(ctx, |ui| {
                                ui.label(egui::RichText::new(name).color(egui::Color32::WHITE).size(14.0));
                            });
                    }
                }
            }

            // Pause overlay
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
                                        pause_action = PauseAction::MainMenu;
                                    }
                                });
                            });
                    });
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

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Tracking Station Render Encoder"),
        });
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_descriptor);

        // Geometry pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tracking Station Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.body_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.sprite_atlas.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Egui pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tracking Station Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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

        Ok((new_warp_index, pause_action, ts_action))
    }
}

/// Blackbody glow color for a given temperature.
/// Returns (r, g, b) in 0.0-1.0 following an approximate electromagnetic
/// spectrum: dark red at 500K, cherry red ~1000K, orange ~2000K, bright
/// yellow at 4000K. Below 500K returns None (no glow).
fn blackbody_color(temp_k: f64) -> Option<[f32; 3]> {
    if temp_k < 500.0 {
        return None;
    }
    // t: 0.0 at 500K, 1.0 at 4000K
    let t = ((temp_k - 500.0) / 3500.0).min(1.0) as f32;

    // Red: starts dim (0.3) at 500K, reaches full quickly
    let r = (0.3 + 0.7 * (t * 2.0).min(1.0)).min(1.0);
    // Green: stays 0 until ~1000K, then rises to ~0.85 at 4000K
    let g = if t < 0.15 { 0.0 } else { 0.85 * ((t - 0.15) / 0.85).powf(1.5) };
    // Blue: stays 0 (no blue in this range — yellow is the limit)
    let b = 0.0_f32;

    Some([r, g, b])
}

/// Apply heat tinting to a color based on temperature (Kelvin).
/// Below 500K: no effect. 500K-4000K: blackbody glow from dark red to bright yellow.
fn apply_heat_tint(color: [f32; 4], temperature: f64) -> [f32; 4] {
    let glow = match blackbody_color(temperature) {
        Some(g) => g,
        None => return color,
    };
    // Blend factor: how much the glow overrides the base color.
    // At 500K the glow is subtle, by ~1500K it dominates.
    let t = ((temperature - 500.0) / 3500.0).min(1.0) as f32;
    let blend = (t * 1.5).min(1.0);
    [
        color[0] * (1.0 - blend) + glow[0] * blend,
        color[1] * (1.0 - blend) + glow[1] * blend,
        color[2] * (1.0 - blend) + glow[2] * blend,
        color[3],
    ]
}
