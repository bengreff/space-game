use super::{FuelType, Propellant, PartDefinitions, VesselBlueprint};
use std::collections::HashMap;

/// A vessel in flight - runtime representation with physics
#[derive(Debug, Clone)]
pub struct FlightVessel {
    // Physics state (same as Ship)
    pub rel_position: [f64; 2],
    pub rel_velocity: [f64; 2],
    pub rotation: f64,
    pub rotational_velocity: f64,
    pub soi_body: usize,

    // Part tree
    pub parts: Vec<FlightPart>,
    pub root_part_index: usize,

    // Aggregated properties (calculated from parts)
    pub total_mass: f64,
    pub dry_mass: f64,
    pub center_of_mass: [f64; 2],
    pub max_thrust_vac: f64,
    pub max_thrust_asl: f64,
    pub moment_of_inertia: f64,
    pub torque: f64,  // From reaction wheels

    // Flight state
    pub throttle: f64,
    pub on_rails: bool,
    pub stages: Vec<Vec<usize>>,
    pub current_stage: usize,
}

/// A part in flight
#[derive(Debug, Clone)]
pub struct FlightPart {
    pub definition_id: String,
    pub local_position: [f64; 2],  // Relative to vessel center of mass
    pub rotation: f64,
    pub hitbox_half_extents: [f64; 2],

    // Resources currently in this part
    pub resources: HashMap<String, f64>,
    pub max_resources: HashMap<String, f64>,

    // Engine state (if this is an engine)
    pub engine_active: bool,
    pub engine_enabled: bool,  // User toggle: if false, engine won't fire even with fuel
    pub engine_thrust_vac: f64,
    pub engine_thrust_asl: f64,
    pub engine_isp_vac: f64,
    pub engine_isp_asl: f64,
    pub is_throttleable: bool,
    pub propellant_type: Option<Propellant>,
    pub mass_flow_rate: f64, // kg/s total (fuel+oxidizer) at full vacuum thrust

    // Gimbal state (if engine has gimbal)
    pub gimbal_angle: f64,      // Current gimbal deflection (radians)
    pub gimbal_range_rad: f64,  // Maximum gimbal deflection (radians, from engine data)

    // State
    pub destroyed: bool,
    pub decoupled: bool,
    pub crossfeed_enabled: bool, // Whether fuel can flow through this decoupler
}

impl FlightVessel {
    /// Create a flight vessel from a blueprint
    pub fn from_blueprint(
        blueprint: &VesselBlueprint,
        part_defs: &PartDefinitions,
        spawn_position: [f64; 2],
        spawn_velocity: [f64; 2],
        soi_body: usize,
    ) -> Result<Self, String> {
        blueprint.validate()?;

        let mut parts = Vec::new();
        let mut total_mass = 0.0;
        let mut dry_mass = 0.0;
        let mut center_of_mass = [0.0, 0.0];
        let mut max_thrust_vac = 0.0;
        let mut max_thrust_asl = 0.0;
        let mut torque = 0.0;

        // First pass: create parts and calculate total mass
        for bp_part in &blueprint.parts {
            let def = part_defs.get(&bp_part.definition_id)
                .ok_or_else(|| format!("Unknown part: {}", bp_part.definition_id))?;

            dry_mass += def.mass;

            // Sum thrust from engines
            if let Some(ref engine) = def.engine {
                max_thrust_vac += engine.thrust_vac;
                max_thrust_asl += engine.thrust_asl;
            }

            // Sum torque from pods
            if let Some(ref pod) = def.pod {
                torque += pod.torque;
            }

            // Create flight part with fuel loaded from blueprint state
            let mut resources = def.resources.clone();
            let mut max_resources = def.resources.clone();

            if let Some(ref tank) = def.tank {
                if bp_part.tank_filled && bp_part.fuel_type != FuelType::Empty {
                    let (ox_kg, fuel_kg) = tank.propellant_capacity(bp_part.fuel_type);
                    if let Some(fuel_name) = bp_part.fuel_type.fuel_resource_name() {
                        resources.insert("oxygen".to_string(), ox_kg);
                        resources.insert(fuel_name.to_string(), fuel_kg);
                        max_resources.insert("oxygen".to_string(), ox_kg);
                        max_resources.insert(fuel_name.to_string(), fuel_kg);
                    }
                }
            }

            // Calculate part mass including fuel
            let resource_mass_kg: f64 = resources.values().sum();
            let part_mass = def.mass + resource_mass_kg * 0.001; // kg -> tonnes
            total_mass += part_mass;

            // Weight center of mass by part mass
            center_of_mass[0] += bp_part.position[0] * part_mass;
            center_of_mass[1] += bp_part.position[1] * part_mass;

            // Create flight part
            let mut flight_part = FlightPart {
                definition_id: bp_part.definition_id.clone(),
                local_position: bp_part.position,
                rotation: bp_part.rotation,
                hitbox_half_extents: [def.width() / 2.0, def.height() / 2.0],
                resources,
                max_resources,
                engine_active: false,
                engine_enabled: false,
                engine_thrust_vac: 0.0,
                engine_thrust_asl: 0.0,
                engine_isp_vac: 0.0,
                engine_isp_asl: 0.0,
                is_throttleable: true,
                propellant_type: None,
                mass_flow_rate: 0.0,
                gimbal_angle: 0.0,
                gimbal_range_rad: 0.0,
                destroyed: false,
                decoupled: false,
                crossfeed_enabled: bp_part.crossfeed_enabled,
            };

            // Set engine data if this is an engine
            if let Some(ref engine) = def.engine {
                flight_part.engine_active = false;
                flight_part.engine_thrust_vac = engine.thrust_vac;
                flight_part.engine_thrust_asl = engine.thrust_asl;
                flight_part.engine_isp_vac = engine.isp_vac;
                flight_part.engine_isp_asl = engine.isp_asl;
                flight_part.is_throttleable = engine.throttleable;
                flight_part.propellant_type = Some(engine.propellant);
                flight_part.gimbal_range_rad = engine.gimbal_range.to_radians();
                // m_dot = F / (g0 * Isp), F in Newtons = kN * 1000
                let g0 = 9.80665;
                flight_part.mass_flow_rate = if engine.isp_vac > 0.0 {
                    (engine.thrust_vac * 1000.0) / (g0 * engine.isp_vac)
                } else {
                    0.0
                };
            }

            parts.push(flight_part);
        }

        // Normalize center of mass
        if total_mass > 0.0 {
            center_of_mass[0] /= total_mass;
            center_of_mass[1] /= total_mass;
        }

        // Second pass: shift part positions relative to center of mass
        for part in &mut parts {
            part.local_position[0] -= center_of_mass[0];
            part.local_position[1] -= center_of_mass[1];
        }

        // Calculate moment of inertia (simplified: sum of m*r^2)
        let mut moment_of_inertia = 0.0;
        for (i, bp_part) in blueprint.parts.iter().enumerate() {
            let def = part_defs.get(&bp_part.definition_id).unwrap();
            let part_mass = def.wet_mass();
            let r_sq = parts[i].local_position[0].powi(2) + parts[i].local_position[1].powi(2);
            moment_of_inertia += part_mass * r_sq;

            // Add part's own moment of inertia (approximate as rectangle)
            let w = def.width();
            let h = def.height();
            moment_of_inertia += part_mass * (w * w + h * h) / 12.0;
        }

        // Minimum moment of inertia for stability
        moment_of_inertia = moment_of_inertia.max(0.1);

        // Convert blueprint stages to part indices
        let stages = blueprint.stages.clone();

        Ok(FlightVessel {
            rel_position: spawn_position,
            rel_velocity: spawn_velocity,
            rotation: 0.0,
            rotational_velocity: 0.0,
            soi_body,
            parts,
            root_part_index: blueprint.root_part_index,
            total_mass,
            dry_mass,
            center_of_mass: [0.0, 0.0], // Now at origin after shifting
            max_thrust_vac,
            max_thrust_asl,
            moment_of_inertia,
            torque,
            throttle: 0.0,
            on_rails: false,
            stages,
            current_stage: 0,
        })
    }

    /// Recalculate mass and center of mass (call after resource consumption or staging)
    pub fn recalculate_mass(&mut self, part_defs: &PartDefinitions) {
        self.total_mass = 0.0;
        let mut com = [0.0, 0.0];

        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }

            let def = part_defs.get(&part.definition_id);
            let base_mass = def.map(|d| d.mass).unwrap_or(0.0);
            let resource_mass: f64 = part.resources.values().sum::<f64>() * 0.001;
            let part_mass = base_mass + resource_mass;

            self.total_mass += part_mass;
            com[0] += part.local_position[0] * part_mass;
            com[1] += part.local_position[1] * part_mass;
        }

        if self.total_mass > 0.0 {
            self.center_of_mass[0] = com[0] / self.total_mass;
            self.center_of_mass[1] = com[1] / self.total_mass;
        }
    }

    /// Update engine_active flags based on fuel availability within each fuel zone.
    /// Engines without their required propellant type are deactivated.
    pub fn update_engine_states(&mut self, part_defs: &PartDefinitions) {
        let zones = self.compute_fuel_zones(part_defs);

        for i in 0..self.parts.len() {
            if self.parts[i].destroyed || self.parts[i].decoupled {
                continue;
            }
            let propellant = match self.parts[i].propellant_type {
                Some(p) => p,
                None => continue,
            };

            // User-disabled engines are always inactive
            if !self.parts[i].engine_enabled {
                self.parts[i].engine_active = false;
                continue;
            }

            let fuel_type = propellant.fuel_type();
            let fuel_name = match fuel_type.fuel_resource_name() {
                Some(n) => n,
                None => {
                    self.parts[i].engine_active = false;
                    continue;
                }
            };

            let engine_zone = zones[i];
            let fuel_available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == engine_zone)
                .filter_map(|(_, p)| p.resources.get(fuel_name))
                .sum();
            let ox_available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == engine_zone)
                .filter_map(|(_, p)| p.resources.get("oxygen"))
                .sum();

            self.parts[i].engine_active = fuel_available > 0.001 && ox_available > 0.001;
        }
    }

    /// Max vacuum thrust from currently active (fueled) engines
    pub fn active_thrust_vac(&self) -> f64 {
        self.parts.iter()
            .filter(|p| p.engine_active && !p.destroyed && !p.decoupled)
            .map(|p| p.engine_thrust_vac)
            .sum()
    }

    /// Max sea-level thrust from currently active (fueled) engines
    pub fn active_thrust_asl(&self) -> f64 {
        self.parts.iter()
            .filter(|p| p.engine_active && !p.destroyed && !p.decoupled)
            .map(|p| p.engine_thrust_asl)
            .sum()
    }

    /// Set gimbal angles on all engines based on rotation input.
    /// When rotating left, engines gimbal to create positive (CCW) torque.
    /// When rotating right, engines gimbal to create negative (CW) torque.
    pub fn update_gimbal(&mut self, rotate_left: bool, rotate_right: bool) {
        for part in &mut self.parts {
            if part.destroyed || part.decoupled || part.gimbal_range_rad <= 0.0 {
                continue;
            }
            if !part.engine_active {
                part.gimbal_angle = 0.0;
                continue;
            }
            if rotate_left {
                part.gimbal_angle = part.gimbal_range_rad;
            } else if rotate_right {
                part.gimbal_angle = -part.gimbal_range_rad;
            } else {
                part.gimbal_angle = 0.0;
            }
        }
    }

    /// Compute net torque from gimbaled engines (kN·m).
    ///
    /// In the vessel's local frame, the structural axis is Y (parts stacked
    /// vertically in the editor) while the physics thrust axis is X. The
    /// nominal (un-gimbaled) engine thrust is along the vessel axis and
    /// produces no torque for centered engines. Gimbal deflection θ rotates
    /// the thrust vector, creating torque:
    ///
    ///   τ = F · [px·(cos θ - 1) - py·sin θ]
    ///
    /// where F = engine_thrust_vac × throttle (kN), and (px, py) is the
    /// engine's local position relative to COM (meters).
    pub fn compute_gimbal_torque(&self) -> f64 {
        let mut torque = 0.0;
        for part in &self.parts {
            if part.destroyed || part.decoupled || !part.engine_active {
                continue;
            }
            if part.gimbal_angle.abs() < 1e-6 || part.gimbal_range_rad <= 0.0 {
                continue;
            }
            let thrust = part.engine_thrust_vac * self.throttle;
            if thrust <= 0.0 {
                continue;
            }
            let px = part.local_position[0];
            let py = part.local_position[1];
            let theta = part.gimbal_angle;
            torque += thrust * (px * (theta.cos() - 1.0) - py * theta.sin());
        }
        torque
    }

    /// Get thrust at a given atmospheric pressure (0.0 = vacuum, 1.0 = sea level)
    pub fn get_thrust(&self, pressure: f64) -> f64 {
        let mut thrust = 0.0;
        for part in &self.parts {
            if part.engine_active && !part.destroyed && !part.decoupled {
                thrust += part.engine_thrust_vac * (1.0 - pressure)
                    + part.engine_thrust_asl * pressure;
            }
        }
        thrust * self.throttle
    }

    /// Get current specific impulse at given pressure
    pub fn get_isp(&self, pressure: f64) -> f64 {
        let mut total_thrust = 0.0;
        let mut weighted_isp = 0.0;

        for part in &self.parts {
            if part.engine_active && !part.destroyed && !part.decoupled {
                let thrust = part.engine_thrust_vac * (1.0 - pressure)
                    + part.engine_thrust_asl * pressure;
                let isp = part.engine_isp_vac * (1.0 - pressure)
                    + part.engine_isp_asl * pressure;

                total_thrust += thrust;
                weighted_isp += thrust * isp;
            }
        }

        if total_thrust > 0.0 {
            weighted_isp / total_thrust
        } else {
            0.0
        }
    }

    /// Consume fuel per-engine and return actual thrust achieved.
    /// Each engine drains oxidizer and its specific fuel type at the tank ratio,
    /// only from tanks in the same fuel zone.
    /// Engines without available fuel are deactivated.
    pub fn consume_fuel(&mut self, dt: f64, pressure: f64, part_defs: &PartDefinitions) -> f64 {
        // Always update which engines have fuel
        self.update_engine_states(part_defs);

        if self.throttle <= 0.0 {
            return 0.0;
        }

        let zones = self.compute_fuel_zones(part_defs);
        let g0 = 9.80665;

        // Phase 1: Collect per-engine fuel demands
        // (engine_index, fuel_resource_name, ox_needed_kg, fuel_needed_kg, engine_thrust_at_pressure, zone)
        let mut engine_demands: Vec<(usize, &'static str, f64, f64, f64, usize)> = Vec::new();

        for (i, part) in self.parts.iter().enumerate() {
            if !part.engine_active || part.destroyed || part.decoupled {
                continue;
            }
            let propellant = match part.propellant_type {
                Some(p) => p,
                None => continue,
            };
            let fuel_type = propellant.fuel_type();
            let fuel_name = match fuel_type.fuel_resource_name() {
                Some(n) => n,
                None => continue,
            };

            // Ox:fuel ratio from the tank chemistry
            let (ox_per_sq, fuel_per_sq) = fuel_type.propellant_per_grid_square();
            let total_per_sq = ox_per_sq + fuel_per_sq;
            if total_per_sq <= 0.0 {
                continue;
            }
            let ox_ratio = ox_per_sq / total_per_sq;
            let fuel_ratio = fuel_per_sq / total_per_sq;

            // Engine thrust and ISP at current pressure
            let engine_thrust = part.engine_thrust_vac * (1.0 - pressure)
                + part.engine_thrust_asl * pressure;
            let engine_isp = part.engine_isp_vac * (1.0 - pressure)
                + part.engine_isp_asl * pressure;
            if engine_isp <= 0.0 {
                continue;
            }

            // Mass flow rate at current throttle: m_dot = F / (g0 * Isp)
            let mass_flow = (engine_thrust * 1000.0) / (g0 * engine_isp);
            let total_consumption = mass_flow * self.throttle * dt;

            engine_demands.push((
                i,
                fuel_name,
                total_consumption * ox_ratio,
                total_consumption * fuel_ratio,
                engine_thrust,
                zones[i],
            ));
        }

        if engine_demands.is_empty() {
            return 0.0;
        }

        // Phase 2: Sum drain amounts per (zone, resource)
        let mut zone_ox_drains: HashMap<usize, f64> = HashMap::new();
        let mut zone_fuel_drains: HashMap<(usize, &str), f64> = HashMap::new();
        let mut total_thrust = 0.0;

        for &(_, fuel_name, ox_needed, fuel_needed, engine_thrust, zone) in &engine_demands {
            *zone_ox_drains.entry(zone).or_insert(0.0) += ox_needed;
            *zone_fuel_drains.entry((zone, fuel_name)).or_insert(0.0) += fuel_needed;
            total_thrust += engine_thrust * self.throttle;
        }

        // Phase 3: Drain resources from tanks within each zone
        // Drain oxygen per zone
        for (&zone, &ox_drain) in &zone_ox_drains {
            let ox_available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter_map(|(_, p)| p.resources.get("oxygen"))
                .sum();

            if ox_drain > 0.0 && ox_available > 0.0 {
                let drain_frac = (ox_drain / ox_available).min(1.0);
                for (j, part) in self.parts.iter_mut().enumerate() {
                    if part.destroyed || part.decoupled || zones[j] != zone {
                        continue;
                    }
                    if let Some(ox) = part.resources.get_mut("oxygen") {
                        *ox -= *ox * drain_frac;
                        if *ox < 0.001 {
                            *ox = 0.0;
                        }
                    }
                }
            }
        }

        // Drain each fuel type per zone
        for (&(zone, fuel_name), &drain_amount) in &zone_fuel_drains {
            let available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter_map(|(_, p)| p.resources.get(fuel_name))
                .sum();

            if drain_amount > 0.0 && available > 0.0 {
                let drain_frac = (drain_amount / available).min(1.0);
                for (j, part) in self.parts.iter_mut().enumerate() {
                    if part.destroyed || part.decoupled || zones[j] != zone {
                        continue;
                    }
                    if let Some(fuel) = part.resources.get_mut(fuel_name) {
                        *fuel -= *fuel * drain_frac;
                        if *fuel < 0.001 {
                            *fuel = 0.0;
                        }
                    }
                }
            }
        }

        total_thrust
    }

    /// Resource names that count as propellant
    const PROPELLANT_RESOURCES: &'static [&'static str] = &["oxygen", "rp1", "methane", "hydrogen"];

    /// Get total fuel available (kg)
    pub fn get_total_fuel(&self) -> f64 {
        let mut total = 0.0;
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            for &name in Self::PROPELLANT_RESOURCES {
                if let Some(&amount) = part.resources.get(name) {
                    total += amount;
                }
            }
        }
        total
    }

    /// Get max total fuel capacity (kg)
    pub fn get_max_fuel(&self) -> f64 {
        let mut total = 0.0;
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            for &name in Self::PROPELLANT_RESOURCES {
                if let Some(&amount) = part.max_resources.get(name) {
                    total += amount;
                }
            }
        }
        total
    }

    /// Get the bounding half-height of the vessel (max extent from COM in Y)
    pub fn bounding_half_height(&self) -> f64 {
        let mut max_extent = 0.0f64;
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            let top = part.local_position[1] + part.hitbox_half_extents[1];
            let bottom = part.local_position[1] - part.hitbox_half_extents[1];
            max_extent = max_extent.max(top.abs()).max(bottom.abs());
        }
        max_extent.max(1.0)
    }

    /// Distance from COM to the bottom of the vessel (most negative Y extent).
    /// Used for placing the vessel on a surface so the engine touches the ground.
    pub fn bottom_extent(&self) -> f64 {
        let mut min_y = 0.0f64;
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            let bottom = part.local_position[1] - part.hitbox_half_extents[1];
            min_y = min_y.min(bottom);
        }
        -min_y
    }

    /// Check if any part collides with terrain (sphere at origin with given radius).
    /// vessel_pos is the vessel center position relative to the body.
    /// vessel_rotation is the vessel's rotation angle.
    /// body_index is the SOI body index (used for launchpad collision).
    /// Returns Some(surface_angle) if collision detected.
    pub fn check_terrain_collision(
        &self,
        vessel_pos: [f64; 2],
        vessel_rotation: f64,
        body_radius: f64,
        body_index: usize,
    ) -> Option<f64> {
        use crate::game::{LAUNCHPAD_BODY_INDEX, LAUNCHPAD_SURFACE_ANGLE,
                          LAUNCHPAD_HEIGHT, LAUNCHPAD_TOP_WIDTH, LAUNCHPAD_BOTTOM_WIDTH};

        let cos_r = vessel_rotation.cos();
        let sin_r = vessel_rotation.sin();

        // Launchpad collision parameters
        let has_launchpad = body_index == LAUNCHPAD_BODY_INDEX;
        let lp_top_half = LAUNCHPAD_TOP_WIDTH * 0.5;
        let lp_bot_half = LAUNCHPAD_BOTTOM_WIDTH * 0.5;
        let lp_surface_radius = body_radius + LAUNCHPAD_HEIGHT;

        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }

            // Check 4 corners of the hitbox
            let hx = part.hitbox_half_extents[0];
            let hy = part.hitbox_half_extents[1];
            let corners = [
                [part.local_position[0] - hx, part.local_position[1] - hy],
                [part.local_position[0] + hx, part.local_position[1] - hy],
                [part.local_position[0] + hx, part.local_position[1] + hy],
                [part.local_position[0] - hx, part.local_position[1] + hy],
            ];

            for corner in &corners {
                // Rotate corner by vessel rotation
                let world_x = vessel_pos[0] + corner[0] * cos_r - corner[1] * sin_r;
                let world_y = vessel_pos[1] + corner[0] * sin_r + corner[1] * cos_r;

                let dist = (world_x * world_x + world_y * world_y).sqrt();

                // Check ground collision
                if dist < body_radius {
                    return Some(world_y.atan2(world_x));
                }

                // Check launchpad collision
                if has_launchpad && dist < lp_surface_radius {
                    let corner_angle = world_y.atan2(world_x);
                    let angle_diff = corner_angle - LAUNCHPAD_SURFACE_ANGLE;
                    let angle_diff = angle_diff - (angle_diff / std::f64::consts::TAU).round() * std::f64::consts::TAU;
                    // Linear interpolation of width from bottom to top
                    let height_frac = ((dist - body_radius) / LAUNCHPAD_HEIGHT).clamp(0.0, 1.0);
                    let half_width_at_height = lp_bot_half + (lp_top_half - lp_bot_half) * height_frac;
                    let half_angle = half_width_at_height / body_radius;
                    if angle_diff.abs() < half_angle {
                        return Some(corner_angle);
                    }
                }
            }
        }

        None
    }

    /// Find weld connections between parts whose welding hitboxes overlap.
    /// Returns adjacency list: for each part index, the set of part indices it is welded to.
    pub fn find_weld_connections(&self, part_defs: &PartDefinitions) -> Vec<Vec<usize>> {
        let n = self.parts.len();
        let mut connections = vec![Vec::new(); n];

        for i in 0..n {
            if self.parts[i].destroyed || self.parts[i].decoupled {
                continue;
            }
            let Some(def_i) = part_defs.get(&self.parts[i].definition_id) else {
                continue;
            };
            let weld_hw_i = def_i.weld_hitbox_width() / 2.0;
            let weld_hh_i = def_i.weld_hitbox_height() / 2.0;

            for j in (i + 1)..n {
                if self.parts[j].destroyed || self.parts[j].decoupled {
                    continue;
                }
                let Some(def_j) = part_defs.get(&self.parts[j].definition_id) else {
                    continue;
                };
                let weld_hw_j = def_j.weld_hitbox_width() / 2.0;
                let weld_hh_j = def_j.weld_hitbox_height() / 2.0;

                // Check AABB overlap of welding hitboxes
                let dx = (self.parts[i].local_position[0] - self.parts[j].local_position[0]).abs();
                let dy = (self.parts[i].local_position[1] - self.parts[j].local_position[1]).abs();

                if dx < weld_hw_i + weld_hw_j && dy < weld_hh_i + weld_hh_j {
                    connections[i].push(j);
                    connections[j].push(i);
                }
            }
        }

        connections
    }

    /// Compute fuel zones by flood-filling the weld adjacency graph.
    /// Non-crossfeed decouplers act as barriers: they are visited but don't
    /// propagate fuel flow to their neighbors.
    /// Returns a zone ID per part (usize::MAX for destroyed/decoupled parts).
    pub fn compute_fuel_zones(&self, part_defs: &PartDefinitions) -> Vec<usize> {
        use std::collections::VecDeque;

        let n = self.parts.len();
        let connections = self.find_weld_connections(part_defs);
        let mut zones = vec![usize::MAX; n];
        let mut current_zone = 0;

        for start in 0..n {
            if zones[start] != usize::MAX {
                continue;
            }
            if self.parts[start].destroyed || self.parts[start].decoupled {
                continue;
            }

            let mut queue = VecDeque::new();
            zones[start] = current_zone;
            queue.push_back(start);

            while let Some(idx) = queue.pop_front() {
                // If this part is a non-crossfeed decoupler, assign it to the
                // zone but don't propagate through it.
                let is_barrier = {
                    let def = part_defs.get(&self.parts[idx].definition_id);
                    def.map(|d| d.decoupler.is_some()).unwrap_or(false)
                        && !self.parts[idx].crossfeed_enabled
                };

                if is_barrier && idx != start {
                    // Barrier parts get visited (assigned a zone) but don't
                    // propagate to neighbors — they might be claimed by
                    // another zone's fill that starts from their other side.
                    continue;
                }

                for &neighbor in &connections[idx] {
                    if zones[neighbor] != usize::MAX {
                        continue;
                    }
                    if self.parts[neighbor].destroyed || self.parts[neighbor].decoupled {
                        continue;
                    }
                    zones[neighbor] = current_zone;
                    queue.push_back(neighbor);
                }
            }

            current_zone += 1;
        }

        zones
    }

    /// Calculate delta-v using the Tsiolkovsky rocket equation
    pub fn calculate_delta_v(&self) -> f64 {
        if self.dry_mass <= 0.0 || self.total_mass <= self.dry_mass {
            return 0.0;
        }
        let isp = self.get_isp(0.0); // Vacuum ISP
        if isp <= 0.0 {
            return 0.0;
        }
        let g0 = 9.80665;
        let ve = g0 * isp;
        ve * (self.total_mass / self.dry_mass).ln()
    }

    /// Check collision between a part and a point (for terrain collision)
    pub fn check_part_collision(&self, part_index: usize, world_point: [f64; 2]) -> bool {
        if part_index >= self.parts.len() {
            return false;
        }

        let part = &self.parts[part_index];
        if part.destroyed || part.decoupled {
            return false;
        }

        // Transform world point to part-local coordinates
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        // Vector from vessel to world point
        let dx = world_point[0] - self.rel_position[0];
        let dy = world_point[1] - self.rel_position[1];

        // Rotate to vessel frame
        let local_x = dx * cos_r + dy * sin_r;
        let local_y = -dx * sin_r + dy * cos_r;

        // Offset by part position
        let part_local_x = local_x - part.local_position[0];
        let part_local_y = local_y - part.local_position[1];

        // Check against hitbox (AABB)
        part_local_x.abs() <= part.hitbox_half_extents[0]
            && part_local_y.abs() <= part.hitbox_half_extents[1]
    }

    /// Get world position of a specific part
    pub fn get_part_world_position(&self, part_index: usize) -> [f64; 2] {
        if part_index >= self.parts.len() {
            return self.rel_position;
        }

        let part = &self.parts[part_index];
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        [
            self.rel_position[0] + part.local_position[0] * cos_r - part.local_position[1] * sin_r,
            self.rel_position[1] + part.local_position[0] * sin_r + part.local_position[1] * cos_r,
        ]
    }

    /// Calculate per-stage delta-v (vacuum) using the Tsiolkovsky rocket equation.
    /// Simulates staging sequentially: decouplers fire, engines activate, all fuel burns.
    pub fn calculate_stage_delta_v(&self, part_defs: &PartDefinitions) -> Vec<f64> {
        let g0 = 9.80665;
        let mut stage_dvs = Vec::new();

        // Track state across stages
        let mut decoupled: Vec<bool> = self.parts.iter().map(|p| p.destroyed || p.decoupled).collect();
        let mut engines_enabled: Vec<bool> = vec![false; self.parts.len()];

        // Track remaining fuel per part (in tonnes)
        let mut fuel_remaining: Vec<f64> = self.parts.iter()
            .map(|p| {
                if p.destroyed || p.decoupled { return 0.0; }
                p.resources.values().sum::<f64>() / 1000.0
            })
            .collect();

        for stage in &self.stages {
            // 1. Fire decouplers in this stage
            for &part_idx in stage {
                if part_idx >= self.parts.len() || decoupled[part_idx] { continue; }
                let Some(def) = part_defs.get(&self.parts[part_idx].definition_id) else { continue };
                if def.decoupler.is_some() {
                    let decoupler_bottom = self.parts[part_idx].local_position[1]
                        - def.hitbox_height() / 2.0;
                    decoupled[part_idx] = true;
                    for i in 0..self.parts.len() {
                        if decoupled[i] { continue; }
                        let Some(other_def) = part_defs.get(&self.parts[i].definition_id) else { continue };
                        let other_top = self.parts[i].local_position[1]
                            + other_def.hitbox_height() / 2.0;
                        if other_top <= decoupler_bottom + 0.01 {
                            decoupled[i] = true;
                        }
                    }
                }
            }

            // 2. Enable engines in this stage
            for &part_idx in stage {
                if part_idx >= self.parts.len() || decoupled[part_idx] { continue; }
                if self.parts[part_idx].propellant_type.is_some() {
                    engines_enabled[part_idx] = true;
                }
            }

            // 3. Calculate wet mass and fuel mass
            let mut wet_mass = 0.0;
            let mut fuel_mass = 0.0;
            for i in 0..self.parts.len() {
                if decoupled[i] { continue; }
                let base_mass = part_defs.get(&self.parts[i].definition_id)
                    .map(|d| d.mass).unwrap_or(0.0);
                wet_mass += base_mass + fuel_remaining[i];
                fuel_mass += fuel_remaining[i];
            }

            // 4. Calculate thrust-weighted average Isp
            let mut total_thrust = 0.0;
            let mut weighted_isp = 0.0;
            for i in 0..self.parts.len() {
                if decoupled[i] || !engines_enabled[i] { continue; }
                total_thrust += self.parts[i].engine_thrust_vac;
                weighted_isp += self.parts[i].engine_thrust_vac * self.parts[i].engine_isp_vac;
            }
            let isp = if total_thrust > 0.0 { weighted_isp / total_thrust } else { 0.0 };

            // 5. Δv = Isp * g0 * ln(wet / dry)
            let dry_mass = wet_mass - fuel_mass;
            let dv = if isp > 0.0 && dry_mass > 0.0 && wet_mass > dry_mass {
                isp * g0 * (wet_mass / dry_mass).ln()
            } else {
                0.0
            };
            stage_dvs.push(dv);

            // 6. Consume all fuel in remaining parts
            for i in 0..self.parts.len() {
                if !decoupled[i] {
                    fuel_remaining[i] = 0.0;
                }
            }
        }

        stage_dvs
    }

    /// Activate next stage (enables engines and fires decouplers in that stage)
    pub fn activate_next_stage(&mut self, part_defs: &PartDefinitions) -> bool {
        if self.current_stage >= self.stages.len() {
            return false;
        }

        let stage_parts = self.stages[self.current_stage].clone();

        for &part_idx in &stage_parts {
            if part_idx >= self.parts.len() || self.parts[part_idx].decoupled {
                continue;
            }

            // Enable engines in this stage
            if self.parts[part_idx].propellant_type.is_some() {
                self.parts[part_idx].engine_enabled = true;
            }

            // Fire decouplers: decouple all parts below the decoupler
            let def = part_defs.get(&self.parts[part_idx].definition_id);
            if let Some(def) = def {
                if def.decoupler.is_some() {
                    let decoupler_bottom = self.parts[part_idx].local_position[1]
                        - def.hitbox_height() / 2.0;

                    // Mark the decoupler itself as decoupled
                    self.parts[part_idx].decoupled = true;

                    // Mark all parts whose top edge is at or below the decoupler bottom
                    for i in 0..self.parts.len() {
                        if i == part_idx || self.parts[i].decoupled {
                            continue;
                        }
                        let other_def = part_defs.get(&self.parts[i].definition_id);
                        let other_top = if let Some(od) = other_def {
                            self.parts[i].local_position[1] + od.hitbox_height() / 2.0
                        } else {
                            self.parts[i].local_position[1] + self.parts[i].hitbox_half_extents[1]
                        };
                        if other_top <= decoupler_bottom + 0.01 {
                            self.parts[i].decoupled = true;
                        }
                    }
                }
            }
        }

        self.current_stage += 1;
        true
    }
}

/// Create a default single-part vessel for testing
pub fn create_default_vessel(
    spawn_position: [f64; 2],
    spawn_velocity: [f64; 2],
    soi_body: usize,
) -> FlightVessel {
    FlightVessel {
        rel_position: spawn_position,
        rel_velocity: spawn_velocity,
        rotation: 0.0,
        rotational_velocity: 0.0,
        soi_body,
        parts: vec![FlightPart {
            definition_id: "default_pod".to_string(),
            local_position: [0.0, 0.0],
            rotation: 0.0,
            hitbox_half_extents: [5.0, 5.0],
            resources: HashMap::new(),
            max_resources: HashMap::new(),
            engine_active: true,
            engine_enabled: true,
            engine_thrust_vac: 200.0,
            engine_thrust_asl: 150.0,
            engine_isp_vac: 300.0,
            engine_isp_asl: 250.0,
            is_throttleable: true,
            propellant_type: None,
            mass_flow_rate: 0.0,
            gimbal_angle: 0.0,
            gimbal_range_rad: 0.0,
            destroyed: false,
            decoupled: false,
            crossfeed_enabled: false,
        }],
        root_part_index: 0,
        total_mass: 2.0,
        dry_mass: 2.0,
        center_of_mass: [0.0, 0.0],
        max_thrust_vac: 200.0,
        max_thrust_asl: 150.0,
        moment_of_inertia: 1.0,
        torque: 5.0,
        throttle: 0.0,
        on_rails: false,
        stages: Vec::new(),
        current_stage: 0,
    }
}
