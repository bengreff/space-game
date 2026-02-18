use super::{PartDefinitions, VesselBlueprint};
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
    pub engine_thrust_vac: f64,
    pub engine_thrust_asl: f64,
    pub engine_isp_vac: f64,
    pub engine_isp_asl: f64,
    pub is_throttleable: bool,

    // State
    pub destroyed: bool,
    pub decoupled: bool,
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

            let part_mass = def.wet_mass();
            total_mass += part_mass;
            dry_mass += def.mass;

            // Weight center of mass by part mass
            center_of_mass[0] += bp_part.position[0] * part_mass;
            center_of_mass[1] += bp_part.position[1] * part_mass;

            // Sum thrust from engines
            if let Some(ref engine) = def.engine {
                max_thrust_vac += engine.thrust_vac;
                max_thrust_asl += engine.thrust_asl;
            }

            // Sum torque from pods
            if let Some(ref pod) = def.pod {
                torque += pod.torque;
            }

            // Create flight part
            let mut flight_part = FlightPart {
                definition_id: bp_part.definition_id.clone(),
                local_position: bp_part.position,
                rotation: bp_part.rotation,
                hitbox_half_extents: [def.width() / 2.0, def.height() / 2.0],
                resources: def.resources.clone(),
                max_resources: def.resources.clone(),
                engine_active: false,
                engine_thrust_vac: 0.0,
                engine_thrust_asl: 0.0,
                engine_isp_vac: 0.0,
                engine_isp_asl: 0.0,
                is_throttleable: true,
                destroyed: false,
                decoupled: false,
            };

            // Set engine data if this is an engine
            if let Some(ref engine) = def.engine {
                flight_part.engine_active = true;
                flight_part.engine_thrust_vac = engine.thrust_vac;
                flight_part.engine_thrust_asl = engine.thrust_asl;
                flight_part.engine_isp_vac = engine.isp_vac;
                flight_part.engine_isp_asl = engine.isp_asl;
                flight_part.is_throttleable = engine.throttleable;
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

    /// Consume fuel and return actual thrust achieved
    pub fn consume_fuel(&mut self, dt: f64, pressure: f64) -> f64 {
        if self.throttle <= 0.0 {
            return 0.0;
        }

        let isp = self.get_isp(pressure);
        if isp <= 0.0 {
            return 0.0;
        }

        // Calculate fuel consumption rate (mass flow rate)
        // F = m_dot * v_e = m_dot * g0 * Isp
        // m_dot = F / (g0 * Isp)
        let g0 = 9.80665; // Standard gravity
        let thrust = self.get_thrust(pressure);
        let mass_flow_rate = thrust / (g0 * isp);
        let fuel_needed = mass_flow_rate * dt;

        // Try to consume fuel from tanks
        let fuel_available = self.get_total_fuel();
        let fuel_consumed = fuel_needed.min(fuel_available);

        if fuel_consumed > 0.0 {
            self.consume_fuel_amount(fuel_consumed);
        }

        // Return actual thrust achieved
        if fuel_needed > 0.0 {
            thrust * (fuel_consumed / fuel_needed)
        } else {
            0.0
        }
    }

    /// Get total fuel available
    fn get_total_fuel(&self) -> f64 {
        let mut total = 0.0;
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            if let Some(&fuel) = part.resources.get("liquid_fuel") {
                total += fuel;
            }
            if let Some(&fuel) = part.resources.get("solid_fuel") {
                total += fuel;
            }
        }
        total
    }

    /// Consume a specific amount of fuel from all tanks
    fn consume_fuel_amount(&mut self, amount: f64) {
        let mut remaining = amount;

        for part in &mut self.parts {
            if part.destroyed || part.decoupled || remaining <= 0.0 {
                continue;
            }

            // Consume liquid fuel first
            if let Some(fuel) = part.resources.get_mut("liquid_fuel") {
                let consumed = (*fuel).min(remaining);
                *fuel -= consumed;
                remaining -= consumed;
            }

            // Then solid fuel
            if let Some(fuel) = part.resources.get_mut("solid_fuel") {
                let consumed = (*fuel).min(remaining);
                *fuel -= consumed;
                remaining -= consumed;
            }
        }
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

    /// Activate next stage
    pub fn activate_next_stage(&mut self) -> bool {
        if self.current_stage >= self.stages.len() {
            return false;
        }

        let stage_parts = self.stages[self.current_stage].clone();

        // Decouple parts in this stage
        for &part_idx in &stage_parts {
            if part_idx < self.parts.len() {
                self.parts[part_idx].decoupled = true;
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
            engine_thrust_vac: 200.0,
            engine_thrust_asl: 150.0,
            engine_isp_vac: 300.0,
            engine_isp_asl: 250.0,
            is_throttleable: true,
            destroyed: false,
            decoupled: false,
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
