use super::formatting::apply_heat_tint;
use super::types::{
    BodyData, OrbitRenderData, ShipRenderData, TrackingVesselData, Vertex, HYPERBOLIC_RENDER_MARGIN,
    orbit_segments,
};
use super::state::RenderState;

/// Pre-computed render data for a procedural star dot.
/// Built in main.rs from GalaxyState, consumed by scene.rs.
#[derive(Clone)]
pub struct StarRenderData {
    pub x: f64,   // world position in meters
    pub y: f64,
    pub color: [f32; 3],
    pub luminosity: f32,    // solar luminosities (determines dot size)
    pub radius_m: f64,      // physical radius in meters (for real circle rendering)
    pub temperature: f32,   // Kelvin
    pub mass_solar: f64,    // solar masses
    pub star_type: &'static str,  // display string e.g. "G-type Main Sequence"
    // Decomposed catalog ID (avoids per-frame String allocation)
    pub catalog_prefix: &'static str, // e.g. "G", "WD", "RG"
    pub sector_x: u16,
    pub sector_y: u16,
    pub sector_index: u32,
    // Pre-computed rendering values (avoids per-frame log/ln)
    pub alpha: f32,       // brightness alpha from luminosity
    pub lum_factor: f32,  // hexagon size factor from luminosity
    // Galactic orbital characteristics
    pub semi_major_axis_m: f64,  // galactic orbit semi-major axis (meters)
    pub eccentricity: f32,       // galactic orbit eccentricity
    pub arg_periapsis: f32,      // galactic orbit argument of periapsis (radians)
    pub orbital_period_s: f64,   // galactic orbital period (seconds)
    // Catalog star fields (None/0 for procedural stars)
    pub catalog_name: Option<&'static str>,
    pub catalog_index: u16,
    pub num_catalog_stars: u8,  // number of stars in catalog system (0 for procedural)
}

impl StarRenderData {
    /// Format the display name: real name for catalog stars, procedural ID otherwise.
    /// Multi-star catalog systems get " System" suffix.
    pub fn format_name(&self) -> String {
        if let Some(name) = self.catalog_name {
            if self.num_catalog_stars > 1 {
                format!("{} System", name)
            } else {
                name.to_string()
            }
        } else {
            format!("{}-{:04}-{:04}-{:04}", self.catalog_prefix, self.sector_x, self.sector_y, self.sector_index)
        }
    }

    /// Check if this star matches the given sector identity.
    pub fn matches_id(&self, sector_x: u16, sector_y: u16, sector_index: u32) -> bool {
        self.sector_x == sector_x && self.sector_y == sector_y && self.sector_index == sector_index
    }
}

impl RenderState {
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

                let segments = orbit_segments(a, self.camera.zoom, self.size.height as f32);
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
        self.update_bodies_orbits_ship_and_vessels(bodies, orbits, ship, scale, part_defs, &[], &[], false, &[]);
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
        procedural_stars: &[StarRenderData],
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
            self.vessel_monoprop_fraction = s.monoprop_fraction;
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
            self.vessel_stage_burn_times = s.stage_burn_times.clone().unwrap_or_default();
            self.ship_soi_surface_gravity = s.soi_surface_gravity;
            self.ship_g_force = s.g_force;
            self.ship_temperature = s.temperature;
            self.ship_heat_fraction = s.heat_fraction;
            self.ship_heat_flux = s.heat_flux;
            self.ship_below_landing_altitude = s.below_landing_altitude;
            self.ship_velocity_direction = s.velocity_direction;
            self.ship_speed_fraction_c = s.speed_fraction_c;
            self.ship_lorentz_gamma = s.lorentz_gamma;
            self.ship_proper_time = s.proper_time;
            self.ship_mission_time = s.mission_time;
            self.ship_is_relativistic = s.is_relativistic;
            self.ship_grav_time_factor = s.grav_time_factor;
            self.ship_orbits_root = s.orbits_root;
            self.ship_has_control = s.has_control;
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

        // Draw procedural stars (on top of galaxy texture, behind sector grid)
        self.current_procedural_stars = procedural_stars.to_vec();
        // Clear hovered_star since screen positions are rebuilt each frame.
        self.hovered_star = None;
        // Re-resolve focused star by sector ID in the current star list
        if let Some((sx, sy, si)) = self.focused_star_id {
            if let Some(new_idx) = self.current_procedural_stars.iter()
                .position(|s| s.matches_id(sx, sy, si))
            {
                self.focused_star = Some(new_idx);
                let star = &self.current_procedural_stars[new_idx];
                self.focused_star_world_pos = Some([star.x, star.y]);
            } else {
                // Star not in current visible set — clear index but keep world pos for tracking.
                // This prevents the info panel from showing a wrong star.
                self.focused_star = None;
            }
        }
        Self::add_procedural_stars_impl(
            &self.camera, self.size, &mut self.procedural_star_screen_positions,
            &mut all_vertices, &mut all_indices, &self.current_procedural_stars, scale,
            self.focused_star,
        );

        // Near Sgr A* (within 1000 ly), show galactic orbits for all catalog stars
        // and the focused star (even if procedural). Only when not in galaxy view
        // and the star's dot is sub-pixel — orbits disappear as you zoom in.
        if !in_galaxy_view {
            const SGR_A_RADIUS_M: f64 = 1000.0 * 9.461e15; // 1000 ly

            for (i, star) in self.current_procedural_stars.iter().enumerate() {
                let dist_from_sgr_a = (star.x * star.x + star.y * star.y).sqrt();
                let near_sgr_a = dist_from_sgr_a < SGR_A_RADIUS_M;
                let is_focused = Some(i) == self.focused_star;

                // Within 1000 ly: catalog stars + focused star
                // Outside: focused star only
                if near_sgr_a {
                    if !is_focused && star.catalog_index == 0 { continue; }
                } else {
                    if !is_focused { continue; }
                }

                // Sub-pixel check using physical stellar radius (matches the
                // hexagon-vs-circle transition in add_procedural_stars_impl).
                // Orbit disappears when you zoom in close enough that the star's
                // physical disk exceeds 1 pixel.
                let radius_px = star.radius_m * scale
                    * self.camera.zoom as f64 * self.size.height as f64 * 0.5;
                if radius_px >= 1.0 { continue; }

                Self::add_galactic_orbit_line(
                    &self.camera, &mut all_vertices, &mut all_indices,
                    star, scale, self.size.height as f32,
                );
            }
        }

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

                let segments = orbit_segments(a, self.camera.zoom, self.size.height as f32);
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
                    let full_segments = orbit_segments(a, self.camera.zoom, self.size.height as f32);
                    let num_segments = ((angle_span.abs() / std::f64::consts::TAU) * full_segments as f64).max(16.0) as u32;
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
                if let Some((parent_pos, orbit_off, dist)) = self.closest_approach_world_pos {
                    // Two-step precision: (parent - body_center) + (orbit_offset - ship_offset)
                    let ca_x = (parent_pos[0] - cam_x) + (orbit_off[0] - off_x);
                    let ca_y = (parent_pos[1] - cam_y) + (orbit_off[1] - off_y);
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

                // Draw target closest approach marker (yellow dot at target's position)
                if let Some((parent_pos, orbit_off, dist)) = self.target_closest_approach_world_pos {
                    // Two-step precision: (parent - body_center) + (orbit_offset - ship_offset)
                    let tca_x = (parent_pos[0] - cam_x) + (orbit_off[0] - off_x);
                    let tca_y = (parent_pos[1] - cam_y) + (orbit_off[1] - off_y);
                    let tca_color = [1.0, 1.0, 0.0, 0.9_f32];

                    let tca_base = all_vertices.len() as u32;
                    all_vertices.push(Vertex::new([tca_x as f32, tca_y as f32], tca_color));
                    for i in 0..marker_segments {
                        let angle = (i as f64 / marker_segments as f64) * std::f64::consts::TAU;
                        all_vertices.push(Vertex::new([
                            (tca_x + marker_radius * angle.cos()) as f32,
                            (tca_y + marker_radius * angle.sin()) as f32,
                        ], tca_color));
                    }
                    for i in 0..marker_segments {
                        all_indices.push(tca_base);
                        all_indices.push(tca_base + 1 + i);
                        all_indices.push(tca_base + 1 + (i + 1) % marker_segments);
                    }
                    self.target_closest_approach_marker = Some(([tca_x, tca_y], dist));
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

                    let num_points = orbit_segments(a_abs, self.camera.zoom, self.size.height as f32) as usize;
                    let mut points: Vec<(f64, f64)> = Vec::with_capacity(num_points);

                    for i in 0..num_points {
                        let t = i as f64 / (num_points - 1) as f64;
                        let ta = start_ta + t * (end_ta - start_ta);

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

                    let num_segments_draw = orbit_segments(a, self.camera.zoom, self.size.height as f32);
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

        // Draw launchpad on body surface (ship view only, not galaxy view)
        if let Some(ship_data) = ship.filter(|_| !in_galaxy_view) {
            self.add_launchpad_vertices(&mut all_vertices, &mut all_indices, bodies, scale, ship_data, self.earth_index);
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


                    let mut part_verts: Vec<Vertex> = Vec::with_capacity(256);
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
                            part_verts.clear();
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
        self.background_vessel_screen_positions.clear();
        if !background_vessels.is_empty() && !in_galaxy_view {
            let cam_x = self.camera.body_center[0];
            let cam_y = self.camera.body_center[1];
            let off_x = self.camera.ship_offset[0];
            let off_y = self.camera.ship_offset[1];
            let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;

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
                    let mut part_verts: Vec<Vertex> = Vec::with_capacity(256);
                    for part_data in parts {
                        if let Some(def) = defs.get(&part_data.definition_id) {
                            let local_x = part_data.local_x as f32 * render_scale;
                            let local_y = part_data.local_y as f32 * render_scale;
                            let rotated_x = local_x * cos_r - local_y * sin_r;
                            let rotated_y = local_x * sin_r + local_y * cos_r;
                            // Skip base disc for fairing half debris (shell-only)
                            part_verts.clear();
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

                // Draw orbit line for this vessel
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

                        let segments = orbit_segments(a, self.camera.zoom, self.size.height as f32);
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

        // Truncate to buffer capacity to avoid wgpu overrun
        let max_verts = self.vertex_buffer.size() as usize / std::mem::size_of::<Vertex>();
        let max_idx = self.index_buffer.size() as usize / std::mem::size_of::<u32>();
        all_vertices.truncate(max_verts);
        all_indices.truncate(max_idx);

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

        // Galaxy image spans 100,000 ly centered on Sgr A* (50,000 ly per side)
        let half = 50_000.0 * crate::bodies::LIGHT_YEAR * scale;
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

    /// Render procedural stars as small colored hexagons (or real circles when zoomed in).
    /// Also stores screen positions for hover/click hit testing.
    /// Static method to avoid borrow conflicts with self.current_procedural_stars.
    fn add_procedural_stars_impl(
        camera: &super::camera::Camera,
        size: winit::dpi::PhysicalSize<u32>,
        screen_positions: &mut Vec<(usize, [f32; 2])>,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        stars: &[StarRenderData],
        scale: f64,
        focused_star: Option<usize>,
    ) {
        screen_positions.clear();

        if stars.is_empty() {
            return;
        }

        // Precomputed unit circle offsets for 6-segment hexagon (indistinguishable
        // from circle at 2-5px). Avoids 12 trig calls per star.
        const OFFSETS: [(f32, f32); 6] = [
            (1.0, 0.0),        // 0°
            (0.5, 0.866025),   // 60°
            (-0.5, 0.866025),  // 120°
            (-1.0, 0.0),       // 180°
            (-0.5, -0.866025), // 240°
            (0.5, -0.866025),  // 300°
        ];

        // Reserve for on-screen stars only (off-screen culling skips most).
        // Estimate ~20% visible at typical zoom; cap to avoid over-allocation.
        let visible_estimate = (stars.len() / 5).min(500_000);
        all_vertices.reserve(visible_estimate * 7);
        all_indices.reserve(visible_estimate * 18);

        // Two-step camera subtraction for precision at galaxy scale.
        // body_center and ship_offset are kept separate in f64, subtracted
        // sequentially before casting to f32 — same pattern as body rendering.
        let cam_x = camera.body_center[0] as f64;
        let cam_y = camera.body_center[1] as f64;
        let off_x = camera.ship_offset[0] as f64;
        let off_y = camera.ship_offset[1] as f64;

        let pixel_ndc = 1.0 / (camera.zoom as f64 * size.height as f64 * 0.5);
        let screen_half_w = size.width as f32 * 0.5;
        let screen_half_h = size.height as f32 * 0.5;
        let aspect = camera.aspect_ratio;
        let zoom = camera.zoom;
        let cos_r = camera.rotation.cos();
        let sin_r = camera.rotation.sin();

        // Culling margins: generous enough to include stars whose hexagons
        // or circles extend past the viewport edge
        let cull_margin = 100.0f32;
        let screen_w = size.width as f32;
        let screen_h = size.height as f32;

        for (i, star) in stars.iter().enumerate() {
            let sx = (star.x * scale - cam_x - off_x) as f32;
            let sy = (star.y * scale - cam_y - off_y) as f32;

            // Convert to screen pixel coords (must include camera rotation)
            let rot_x = sx * cos_r - sy * sin_r;
            let rot_y = sx * sin_r + sy * cos_r;
            let ndc_x = rot_x * zoom / aspect;
            let ndc_y = rot_y * zoom;
            let screen_px = (ndc_x + 1.0) * screen_half_w;
            let screen_py = (1.0 - ndc_y) * screen_half_h;

            // Skip vertex generation entirely for off-screen stars
            if screen_px < -cull_margin || screen_px > screen_w + cull_margin
                || screen_py < -cull_margin || screen_py > screen_h + cull_margin
            {
                continue;
            }

            // Skip rendering AND hit-testing for focused multi-star catalog systems.
            // The companion star bodies (injected by inject_catalog_planets) replace this
            // dot visually — rendering both creates a duplicate at the barycenter.
            // Hit-testing must also be skipped: the invisible barycenter dot would otherwise
            // capture clicks, clearing tracked_body and re-centering the camera on star A.
            if focused_star == Some(i) && star.num_catalog_stars > 1 {
                continue;
            }

            // Store screen position for hit testing (hover/click)
            screen_positions.push((i, [screen_px, screen_py]));

            // Check if physical radius is large enough on screen for a real circle
            // radius_world is in camera-relative world units (shader applies zoom)
            let radius_world = star.radius_m * scale as f64;
            let radius_px = radius_world * zoom as f64 * size.height as f64 * 0.5;

            // Use pre-computed alpha from build_procedural_star_data
            let color = [star.color[0], star.color[1], star.color[2], star.alpha];

            if radius_px > 1.0 {
                // Draw a real circle with adaptive segment count
                // Use full opacity for real circles — dimming is only for distant dots
                let circle_color = [star.color[0], star.color[1], star.color[2], 1.0f32];
                let r_ndc = radius_world as f32;
                let circumference = 2.0 * std::f64::consts::PI * radius_px;
                let segments = (circumference / 3.0).clamp(16.0, 256.0) as usize;

                let base = all_vertices.len() as u32;
                // Center vertex
                all_vertices.push(Vertex::new([sx, sy], circle_color));
                for seg in 0..segments {
                    let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;
                    all_vertices.push(Vertex::new(
                        [sx + r_ndc * angle.cos(), sy + r_ndc * angle.sin()],
                        circle_color,
                    ));
                }
                for seg in 0..segments {
                    let next = if seg + 1 < segments { seg + 1 } else { 0 };
                    all_indices.push(base);
                    all_indices.push(base + 1 + seg as u32);
                    all_indices.push(base + 1 + next as u32);
                }
            } else {
                // Small hexagon dot — use pre-computed lum_factor
                let half = (pixel_ndc * star.lum_factor as f64 * 2.0) as f32;

                let base = all_vertices.len() as u32;
                all_vertices.push(Vertex::new([sx, sy], color));
                for &(ox, oy) in &OFFSETS {
                    all_vertices.push(Vertex::new([sx + half * ox, sy + half * oy], color));
                }
                all_indices.extend_from_slice(&[
                    base, base + 1, base + 2,
                    base, base + 2, base + 3,
                    base, base + 3, base + 4,
                    base, base + 4, base + 5,
                    base, base + 5, base + 6,
                    base, base + 6, base + 1,
                ]);
            }

            // Ring indicator for catalog stars — distinguishes them from procedural stars.
            // Same style as body indicators: 64-segment ring, 16px outer, 70% inner.
            if star.catalog_index > 0 {
                let ring_outer = (16.0 * pixel_ndc) as f32;
                let ring_inner = (16.0 * 0.7 * pixel_ndc) as f32;
                let ring_segments = 64u32;
                let ring_color = [star.color[0], star.color[1], star.color[2], 1.0];
                let ring_inner_color = [star.color[0] * 0.3, star.color[1] * 0.3, star.color[2] * 0.3, 0.5];

                let base = all_vertices.len() as u32;
                for seg in 0..ring_segments {
                    let angle = (seg as f32 / ring_segments as f32) * std::f32::consts::TAU;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    all_vertices.push(Vertex::new([sx + ring_outer * cos_a, sy + ring_outer * sin_a], ring_color));
                    all_vertices.push(Vertex::new([sx + ring_inner * cos_a, sy + ring_inner * sin_a], ring_inner_color));
                }

                for seg in 0..ring_segments {
                    let i0 = base + seg * 2;
                    let i1 = base + seg * 2 + 1;
                    let i2 = base + ((seg + 1) % ring_segments) * 2;
                    let i3 = base + ((seg + 1) % ring_segments) * 2 + 1;
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

    /// Draw the galactic orbit ellipse for the focused star.
    /// The orbit is around the galactic center (0,0) using the star's orbital elements.
    fn add_galactic_orbit_line(
        camera: &super::camera::Camera,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        star: &StarRenderData,
        scale: f64,
        screen_height: f32,
    ) {
        let a = star.semi_major_axis_m * scale;
        let e = star.eccentricity as f64;
        let arg_peri = star.arg_periapsis as f64;

        if a <= 0.0 || e >= 1.0 {
            return;
        }

        let b = a * (1.0 - e * e).sqrt();
        let c = a * e;
        // Orbit center (focus is at galactic center = 0,0)
        let center_x = -c * arg_peri.cos();
        let center_y = -c * arg_peri.sin();

        let cam_x = camera.body_center[0] as f64;
        let cam_y = camera.body_center[1] as f64;
        let off_x = camera.ship_offset[0] as f64;
        let off_y = camera.ship_offset[1] as f64;

        let line_width = 0.002 / camera.zoom as f64;
        let orbit_color = [star.color[0] * 0.4, star.color[1] * 0.4, star.color[2] * 0.4, 0.5];

        let segments = orbit_segments(a, camera.zoom, screen_height);
        let base = all_vertices.len() as u32;

        for i in 0..segments {
            let angle = (i as f64 / segments as f64) * std::f64::consts::TAU;
            let ex = a * angle.cos();
            let ey = b * angle.sin();
            // Rotate by argument of periapsis
            let wx = center_x + ex * arg_peri.cos() - ey * arg_peri.sin();
            let wy = center_y + ex * arg_peri.sin() + ey * arg_peri.cos();

            let rel_x = (wx - cam_x - off_x) as f32;
            let rel_y = (wy - cam_y - off_y) as f32;

            // Perpendicular direction for line thickness
            let next_angle = ((i + 1) as f64 / segments as f64) * std::f64::consts::TAU;
            let nex = a * next_angle.cos();
            let ney = b * next_angle.sin();
            let nwx = center_x + nex * arg_peri.cos() - ney * arg_peri.sin();
            let nwy = center_y + nex * arg_peri.sin() + ney * arg_peri.cos();

            let dx = (nwx - wx) as f32;
            let dy = (nwy - wy) as f32;
            let len = (dx * dx + dy * dy).sqrt().max(1e-20);
            let nx = -dy / len * line_width as f32;
            let ny = dx / len * line_width as f32;

            all_vertices.push(Vertex::new([rel_x + nx, rel_y + ny], orbit_color));
            all_vertices.push(Vertex::new([rel_x - nx, rel_y - ny], orbit_color));
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

        // Two-pass indicator rendering: defer the root body's (first body) indicator
        // to render AFTER all other bodies, so it appears on top (e.g. Sun over planets).
        let mut deferred_root_indicator: Option<(f32, f32, [f32; 4])> = None;

        for (body_array_idx, (x, y, radius, color, _atmo_height, _atmo_color, body_idx)) in bodies.iter().enumerate() {
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
                let ring_color = if color[0] + color[1] + color[2] < 0.1 {
                    [0.7, 0.5, 0.3, 1.0] // Warm amber fallback for black holes
                } else {
                    *color
                };

                // Defer the first body's indicator to render last (on top of all others)
                if body_array_idx == 0 {
                    deferred_root_indicator = Some((rel_x, rel_y, ring_color));
                } else {
                    Self::draw_ring_indicator(all_vertices, all_indices, rel_x, rel_y, indicator_world_radius, ring_color);
                }
            }
        }

        // Second pass: draw deferred root body indicator on top of everything
        if let Some((rel_x, rel_y, ring_color)) = deferred_root_indicator {
            Self::draw_ring_indicator(all_vertices, all_indices, rel_x, rel_y, indicator_world_radius, ring_color);
        }
    }

    /// Draw a ring indicator at the given position (shared by body indicator rendering).
    fn draw_ring_indicator(
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        rel_x: f32,
        rel_y: f32,
        indicator_world_radius: f64,
        ring_color: [f32; 4],
    ) {
        let base_index = all_vertices.len() as u32;
        let ring_outer = indicator_world_radius as f32;
        let ring_inner = (indicator_world_radius * 0.7) as f32;
        let ring_segments = 64u32;

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

    /// Draw the launchpad on Earth's surface when in ship view.
    fn add_launchpad_vertices(
        &self,
        all_vertices: &mut Vec<Vertex>,
        all_indices: &mut Vec<u32>,
        bodies: &[(f64, f64, f64, [f32; 4], f64, [f32; 3], usize)],
        scale: f64,
        ship: &ShipRenderData,
        earth_index: usize,
    ) {
        use crate::game::{LAUNCHPAD_SURFACE_ANGLE,
                          LAUNCHPAD_HEIGHT, LAUNCHPAD_TOP_WIDTH, LAUNCHPAD_BOTTOM_WIDTH};
        let pixels_per_world_unit = self.camera.zoom * self.size.height as f32 / 2.0;
        let ship_pixels = ship.size as f32 * pixels_per_world_unit * 2.0;

        // Only draw launchpad in ship view
        if ship_pixels < 5.0 {
            return;
        }

        // Find the launchpad body
        let Some((bx, by, radius, _, _, _, _)) = bodies.get(earth_index) else { return };

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
}
