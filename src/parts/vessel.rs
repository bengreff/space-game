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

    // Flight state
    pub throttle: f64,
    pub on_rails: bool,
    pub stages: Vec<Vec<usize>>,
    pub current_stage: usize,

    // Ejection force from last decoupler firing (kN), consumed by handle_post_decouple
    pub last_decouple_force: f64,
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

    // RCS state (if this is an RCS thruster)
    pub rcs_thrust: f64,         // kN (0 if not an RCS part)
    pub rcs_isp: f64,            // seconds
    pub rcs_mass_flow_rate: f64, // kg/s at full thrust

    // State
    pub destroyed: bool,
    pub decoupled: bool,
    pub crossfeed_enabled: bool, // Whether fuel can flow through this decoupler

    // Thermal state
    pub temperature: f64,         // Kelvin
    pub max_heat_tolerance: f64,  // Kelvin (from PartDefinition)

    // Fairing shell shape (if this is a fairing base)
    pub fairing_shape: Option<crate::parts::FairingShape>,
    // Which half to render (None = full shell, Some = debris half)
    pub fairing_half: Option<crate::parts::FairingHalf>,

    // Electricity (stored separately from resources to avoid mass calculation interference)
    pub electricity: f64,      // Current stored Wh
    pub max_electricity: f64,  // Capacity Wh
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

            // Create flight part with fuel loaded from blueprint state
            let mut resources = def.resources.clone();
            let mut max_resources = def.resources.clone();

            if let Some(ref tank) = def.tank {
                let fill = if bp_part.fill_fraction > 0.0 {
                    bp_part.fill_fraction
                } else if bp_part.tank_filled {
                    1.0
                } else {
                    0.0
                };
                if fill > 0.0 && bp_part.fuel_type != FuelType::Empty {
                    let (ox_kg, fuel_kg) = tank.propellant_capacity(bp_part.fuel_type);
                    if let Some(fuel_name) = bp_part.fuel_type.fuel_resource_name() {
                        if ox_kg > 0.0 {
                            resources.insert("oxygen".to_string(), ox_kg * fill);
                            max_resources.insert("oxygen".to_string(), ox_kg);
                        }
                        resources.insert(fuel_name.to_string(), fuel_kg * fill);
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

            // Electricity from battery data (starts fully charged)
            let max_elec = def.battery.as_ref().map(|b| b.capacity_wh).unwrap_or(0.0);

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
                rcs_thrust: 0.0,
                rcs_isp: 0.0,
                rcs_mass_flow_rate: 0.0,
                destroyed: false,
                decoupled: false,
                crossfeed_enabled: bp_part.crossfeed_enabled,
                temperature: 300.0,
                max_heat_tolerance: def.max_heat_tolerance,
                fairing_shape: bp_part.fairing_shape.clone(),
                fairing_half: None,
                electricity: max_elec,
                max_electricity: max_elec,
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

            // Set RCS data if this is an RCS thruster
            if let Some(ref rcs) = def.rcs {
                flight_part.rcs_thrust = rcs.thrust;
                flight_part.rcs_isp = rcs.isp;
                let g0 = 9.80665;
                flight_part.rcs_mass_flow_rate = if rcs.isp > 0.0 {
                    (rcs.thrust * 1000.0) / (g0 * rcs.isp)
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
            // Use actual part mass (dry + fuel), not def.wet_mass() which lacks loaded fuel
            let base_mass = def.mass;
            let resource_mass: f64 = parts[i].resources.values().sum::<f64>() * 0.001;
            let part_mass = base_mass + resource_mass;
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
            throttle: 0.0,
            on_rails: false,
            stages,
            current_stage: 0,
            last_decouple_force: 0.0,
        })
    }

    /// Recalculate mass, center of mass, and moment of inertia
    /// (call after resource consumption or staging)
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

        // Recalculate moment of inertia with current masses
        let mut moi = 0.0;
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            let def = match part_defs.get(&part.definition_id) {
                Some(d) => d,
                None => continue,
            };
            let base_mass = def.mass;
            let resource_mass: f64 = part.resources.values().sum::<f64>() * 0.001;
            let part_mass = base_mass + resource_mass;

            let r_sq = part.local_position[0].powi(2) + part.local_position[1].powi(2);
            moi += part_mass * r_sq;

            let w = def.width();
            let h = def.height();
            moi += part_mass * (w * w + h * h) / 12.0;
        }
        self.moment_of_inertia = moi.max(0.1);
    }

    /// Recenter all part local_positions around the new center of mass after decoupling.
    /// Returns the COM offset in vessel-local coordinates (before rotation) so the caller
    /// can shift the ship's world position to keep the vessel in the same physical location.
    pub fn recenter_on_com(&mut self, part_defs: &PartDefinitions) -> [f64; 2] {
        // First, recalculate mass to get the current COM
        self.recalculate_mass(part_defs);

        let com = self.center_of_mass;
        if com[0].abs() < 1e-9 && com[1].abs() < 1e-9 {
            return [0.0, 0.0]; // Already centered
        }

        // Shift all part positions so COM is at origin
        for part in &mut self.parts {
            part.local_position[0] -= com[0];
            part.local_position[1] -= com[1];
        }

        // Reset COM to origin and recalculate MOI with new positions
        self.center_of_mass = [0.0, 0.0];
        let mut moi = 0.0;
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            let def = match part_defs.get(&part.definition_id) {
                Some(d) => d,
                None => continue,
            };
            let base_mass = def.mass;
            let resource_mass: f64 = part.resources.values().sum::<f64>() * 0.001;
            let part_mass = base_mass + resource_mass;

            let r_sq = part.local_position[0].powi(2) + part.local_position[1].powi(2);
            moi += part_mass * r_sq;
            let w = def.width();
            let h = def.height();
            moi += part_mass * (w * w + h * h) / 12.0;
        }
        self.moment_of_inertia = moi.max(0.1);

        // Return the old COM offset so the ship position can be corrected
        com
    }

    /// Check if an engine is covered by a non-decoupled decoupler directly below it.
    /// A covered engine has a decoupler whose top edge is near/touching the engine's bottom
    /// edge and whose horizontal extent overlaps the engine's.
    pub fn is_engine_covered(&self, engine_idx: usize, part_defs: &PartDefinitions) -> bool {
        let engine = &self.parts[engine_idx];
        let engine_bottom = engine.local_position[1] - engine.hitbox_half_extents[1];
        let engine_left = engine.local_position[0] - engine.hitbox_half_extents[0];
        let engine_right = engine.local_position[0] + engine.hitbox_half_extents[0];

        for (i, part) in self.parts.iter().enumerate() {
            if i == engine_idx || part.destroyed || part.decoupled {
                continue;
            }
            let Some(def) = part_defs.get(&part.definition_id) else { continue };
            if def.decoupler.is_none() {
                continue;
            }
            let decoupler_top = part.local_position[1] + part.hitbox_half_extents[1];
            let decoupler_left = part.local_position[0] - part.hitbox_half_extents[0];
            let decoupler_right = part.local_position[0] + part.hitbox_half_extents[0];

            // Vertical adjacency: decoupler top near engine bottom
            if (decoupler_top - engine_bottom).abs() < 0.3
                && engine_left < decoupler_right
                && engine_right > decoupler_left
            {
                return true;
            }
        }
        false
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

            // Engines covered by a decoupler below cannot fire
            if self.is_engine_covered(i, part_defs) {
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

    /// Max thrust at a given atmospheric pressure (0.0 = vacuum, 1.0 = sea level)
    /// Does NOT apply throttle — returns max capability.
    pub fn active_thrust_at_pressure(&self, pressure: f64) -> f64 {
        self.parts.iter()
            .filter(|p| p.engine_active && !p.destroyed && !p.decoupled)
            .map(|p| p.engine_thrust_vac * (1.0 - pressure) + p.engine_thrust_asl * pressure)
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

    /// Compute total available RCS torque (kN·m) from all non-decoupled RCS thrusters
    /// that have monopropellant available in their fuel zone.
    /// Torque = sum of (thrust * lever_arm) where lever_arm = distance from COM (min 0.25m).
    pub fn compute_rcs_torque(&self, part_defs: &PartDefinitions) -> f64 {
        let zones = self.compute_fuel_zones(part_defs);
        let mut torque = 0.0;

        for (i, part) in self.parts.iter().enumerate() {
            if part.destroyed || part.decoupled || part.rcs_thrust <= 0.0 {
                continue;
            }

            // Check if monopropellant is available in this part's fuel zone
            let zone = zones[i];
            if zone == usize::MAX {
                continue;
            }
            let monoprop_available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter_map(|(_, p)| p.resources.get("monopropellant"))
                .sum();
            if monoprop_available < 0.001 {
                continue;
            }

            // Lever arm = distance from COM (min 0.25m to always provide some torque)
            let lever_arm = (part.local_position[0].powi(2) + part.local_position[1].powi(2))
                .sqrt()
                .max(0.25);
            torque += part.rcs_thrust * lever_arm;
        }

        torque
    }

    /// Compute total available RCS translation force (kN) from all non-decoupled RCS thrusters
    /// that have monopropellant available in their fuel zone.
    pub fn compute_rcs_translation_force(&self, part_defs: &PartDefinitions) -> f64 {
        let zones = self.compute_fuel_zones(part_defs);
        let mut force = 0.0;

        for (i, part) in self.parts.iter().enumerate() {
            if part.destroyed || part.decoupled || part.rcs_thrust <= 0.0 {
                continue;
            }

            let zone = zones[i];
            if zone == usize::MAX {
                continue;
            }
            let monoprop_available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter_map(|(_, p)| p.resources.get("monopropellant"))
                .sum();
            if monoprop_available < 0.001 {
                continue;
            }

            force += part.rcs_thrust;
        }

        force
    }

    /// Consume monopropellant from RCS thrusters during translation.
    /// `translate` is [forward, right] in vessel-local frame, -1..1 each.
    pub fn consume_rcs_translation_fuel(&mut self, dt: f64, translate: [f64; 2], part_defs: &PartDefinitions) {
        let mag = (translate[0].powi(2) + translate[1].powi(2)).sqrt();
        if mag < 0.001 {
            return;
        }

        let zones = self.compute_fuel_zones(part_defs);
        let mut zone_demands: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();

        for (i, part) in self.parts.iter().enumerate() {
            if part.destroyed || part.decoupled || part.rcs_thrust <= 0.0 {
                continue;
            }
            let zone = zones[i];
            if zone == usize::MAX {
                continue;
            }
            let monoprop_available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter_map(|(_, p)| p.resources.get("monopropellant"))
                .sum();
            if monoprop_available < 0.001 {
                continue;
            }

            // Scale consumption by translation magnitude (capped at 1)
            let consumption = part.rcs_mass_flow_rate * mag.min(1.0) * dt;
            *zone_demands.entry(zone).or_insert(0.0) += consumption;
        }

        for (&zone, &drain_amount) in &zone_demands {
            let available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter_map(|(_, p)| p.resources.get("monopropellant"))
                .sum();
            if drain_amount > 0.0 && available > 0.0 {
                let drain_frac = (drain_amount / available).min(1.0);
                for (j, part) in self.parts.iter_mut().enumerate() {
                    if part.destroyed || part.decoupled || zones[j] != zone {
                        continue;
                    }
                    if let Some(mp) = part.resources.get_mut("monopropellant") {
                        *mp -= *mp * drain_frac;
                        if *mp < 0.001 {
                            *mp = 0.0;
                        }
                    }
                }
            }
        }
    }

    /// Consume monopropellant from RCS thrusters during rotation.
    /// `direction` is +1.0 (CCW), -1.0 (CW), or 0.0 (no rotation).
    /// Returns actual torque available (kN·m).
    pub fn consume_rcs_fuel(&mut self, dt: f64, direction: f64, part_defs: &PartDefinitions) -> f64 {
        if direction.abs() < 0.001 {
            return 0.0;
        }

        let zones = self.compute_fuel_zones(part_defs);
        let mut total_torque = 0.0;

        // Collect RCS fuel demands per zone
        let mut zone_demands: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();

        for (i, part) in self.parts.iter().enumerate() {
            if part.destroyed || part.decoupled || part.rcs_thrust <= 0.0 {
                continue;
            }
            let zone = zones[i];
            if zone == usize::MAX {
                continue;
            }
            let monoprop_available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter_map(|(_, p)| p.resources.get("monopropellant"))
                .sum();
            if monoprop_available < 0.001 {
                continue;
            }

            let lever_arm = (part.local_position[0].powi(2) + part.local_position[1].powi(2))
                .sqrt()
                .max(0.25);
            total_torque += part.rcs_thrust * lever_arm;

            // Fuel consumption: mass_flow_rate * dt
            let consumption = part.rcs_mass_flow_rate * dt;
            *zone_demands.entry(zone).or_insert(0.0) += consumption;
        }

        // Drain monopropellant from tanks in each zone
        for (&zone, &drain_amount) in &zone_demands {
            let available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter_map(|(_, p)| p.resources.get("monopropellant"))
                .sum();
            if drain_amount > 0.0 && available > 0.0 {
                let drain_frac = (drain_amount / available).min(1.0);
                for (j, part) in self.parts.iter_mut().enumerate() {
                    if part.destroyed || part.decoupled || zones[j] != zone {
                        continue;
                    }
                    if let Some(mp) = part.resources.get_mut("monopropellant") {
                        *mp -= *mp * drain_frac;
                        if *mp < 0.001 {
                            *mp = 0.0;
                        }
                    }
                }
            }
        }

        total_torque
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

        // Phase 3: Drain resources from tanks within each zone,
        // prioritizing tanks behind crossfeed decouplers (asparagus/onion staging)
        let priorities = self.compute_drain_priorities(part_defs);

        // Drain oxygen per zone (lowest priority tanks first)
        for (&zone, &ox_drain) in &zone_ox_drains {
            // Find min priority among tanks in this zone that have oxygen
            let min_pri = self.parts.iter().enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter(|(_, p)| p.resources.get("oxygen").copied().unwrap_or(0.0) > 0.0)
                .map(|(j, _)| priorities[j])
                .min()
                .unwrap_or(usize::MAX);

            let ox_available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone && priorities[*j] == min_pri)
                .filter_map(|(_, p)| p.resources.get("oxygen"))
                .sum();

            if ox_drain > 0.0 && ox_available > 0.0 {
                let drain_frac = (ox_drain / ox_available).min(1.0);
                for (j, part) in self.parts.iter_mut().enumerate() {
                    if part.destroyed || part.decoupled || zones[j] != zone || priorities[j] != min_pri {
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

        // Drain each fuel type per zone (lowest priority tanks first)
        for (&(zone, fuel_name), &drain_amount) in &zone_fuel_drains {
            let min_pri = self.parts.iter().enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone)
                .filter(|(_, p)| p.resources.get(fuel_name).copied().unwrap_or(0.0) > 0.0)
                .map(|(j, _)| priorities[j])
                .min()
                .unwrap_or(usize::MAX);

            let available: f64 = self.parts.iter()
                .enumerate()
                .filter(|(j, p)| !p.destroyed && !p.decoupled && zones[*j] == zone && priorities[*j] == min_pri)
                .filter_map(|(_, p)| p.resources.get(fuel_name))
                .sum();

            if drain_amount > 0.0 && available > 0.0 {
                let drain_frac = (drain_amount / available).min(1.0);
                for (j, part) in self.parts.iter_mut().enumerate() {
                    if part.destroyed || part.decoupled || zones[j] != zone || priorities[j] != min_pri {
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
    const PROPELLANT_RESOURCES: &'static [&'static str] = &["oxygen", "rp1", "methane", "hydrogen", "monopropellant"];

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

    /// Get fuel accessible to currently active engines (current_kg, max_kg).
    /// Uses fuel zones: only counts fuel in zones that contain an active engine.
    pub fn get_stage_fuel(&self, part_defs: &PartDefinitions) -> (f64, f64) {
        let zones = self.compute_fuel_zones(part_defs);

        // Find zone IDs that contain at least one active, non-destroyed, non-decoupled engine
        let mut active_zones = std::collections::HashSet::new();
        for (i, part) in self.parts.iter().enumerate() {
            if part.destroyed || part.decoupled {
                continue;
            }
            if part.engine_active && part.engine_enabled && part.engine_thrust_vac > 0.0 {
                if zones[i] != usize::MAX {
                    active_zones.insert(zones[i]);
                }
            }
        }

        if active_zones.is_empty() {
            return (0.0, 0.0);
        }

        let mut current = 0.0;
        let mut max = 0.0;
        for (i, part) in self.parts.iter().enumerate() {
            if !active_zones.contains(&zones[i]) {
                continue;
            }
            for &name in Self::PROPELLANT_RESOURCES {
                if let Some(&amount) = part.resources.get(name) {
                    current += amount;
                }
                if let Some(&amount) = part.max_resources.get(name) {
                    max += amount;
                }
            }
        }

        (current, max)
    }

    /// Total electricity stored across all non-decoupled battery parts (Wh)
    pub fn total_electricity(&self) -> f64 {
        self.parts.iter()
            .filter(|p| !p.destroyed && !p.decoupled && p.max_electricity > 0.0)
            .map(|p| p.electricity)
            .sum()
    }

    /// Total electricity capacity across all non-decoupled battery parts (Wh)
    pub fn max_electricity(&self) -> f64 {
        self.parts.iter()
            .filter(|p| !p.destroyed && !p.decoupled)
            .map(|p| p.max_electricity)
            .sum()
    }

    /// Electricity fraction (0.0–1.0), or None if no batteries
    pub fn electricity_fraction(&self) -> Option<f64> {
        let max = self.max_electricity();
        if max > 0.0 {
            Some(self.total_electricity() / max)
        } else {
            None
        }
    }

    /// Update power: generate from solar/RTG/alternators, consume from pods.
    /// Distributes net charge/drain proportionally across battery parts.
    /// `sun_distance_m` is ship-to-Sun distance in meters.
    /// Returns (generation_watts, consumption_watts).
    pub fn update_power(&mut self, dt: f64, sun_distance_m: f64, part_defs: &PartDefinitions) -> (f64, f64) {
        const AU_M: f64 = 1.496e11; // 1 AU in meters

        let mut generation = 0.0_f64;
        let mut consumption = 0.0_f64;

        // Calculate generation and consumption from active (non-decoupled, non-destroyed) parts
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            let Some(def) = part_defs.get(&part.definition_id) else { continue };

            // Solar panels: inverse-square scaling from 1 AU
            if let Some(ref solar) = def.solar_panel {
                let distance_ratio = AU_M / sun_distance_m.max(1.0);
                generation += solar.output_1au * distance_ratio * distance_ratio;
            }

            // RTG: constant output
            if let Some(ref rtg) = def.rtg {
                generation += rtg.output_watts;
            }

            // Engine alternators: only when engine is active
            if let Some(ref engine) = def.engine {
                if engine.alternator_power > 0.0 && part.engine_active {
                    generation += engine.alternator_power;
                }
            }

            // Pod power draw
            if let Some(ref pod) = def.pod {
                if pod.power_draw > 0.0 {
                    consumption += pod.power_draw;
                }
            }
        }

        // Net energy change in Wh for this timestep
        let net_wh = (generation - consumption) * dt / 3600.0;

        // Distribute across battery parts proportionally
        let max_elec = self.max_electricity();
        if max_elec > 0.0 && net_wh.abs() > 1e-12 {
            for part in &mut self.parts {
                if part.destroyed || part.decoupled || part.max_electricity <= 0.0 {
                    continue;
                }
                let fraction = part.max_electricity / max_elec;
                let part_delta = net_wh * fraction;
                part.electricity = (part.electricity + part_delta).clamp(0.0, part.max_electricity);
            }
        }

        (generation, consumption)
    }

    /// Get the bounding half-width of the vessel (max extent from COM in X)
    pub fn bounding_half_width(&self) -> f64 {
        let mut max_extent = 0.0f64;
        for part in &self.parts {
            if part.destroyed || part.decoupled {
                continue;
            }
            let right = part.local_position[0] + part.hitbox_half_extents[0];
            let left = part.local_position[0] - part.hitbox_half_extents[0];
            max_extent = max_extent.max(right.abs()).max(left.abs());
        }
        max_extent.max(0.5)
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

    /// Compute drain priority for each part.
    /// Tanks behind crossfeed-enabled decouplers in earlier stages get lower
    /// priority values (drain first). Tanks always reachable from root get
    /// `usize::MAX` (drain last). Used for asparagus/onion staging.
    fn compute_drain_priorities(&self, part_defs: &PartDefinitions) -> Vec<usize> {
        use std::collections::VecDeque;

        let n = self.parts.len();
        let connections = self.find_weld_connections(part_defs);

        // Find root (same logic as decouple_disconnected)
        let root = if self.root_part_index < n
            && !self.parts[self.root_part_index].destroyed
            && !self.parts[self.root_part_index].decoupled
        {
            self.root_part_index
        } else {
            self.parts.iter().enumerate()
                .filter(|(_, p)| !p.destroyed && !p.decoupled)
                .find(|(_, p)| part_defs.get(&p.definition_id).map(|d| d.pod.is_some()).unwrap_or(false))
                .map(|(i, _)| i)
                .or_else(|| self.parts.iter().enumerate()
                    .find(|(_, p)| !p.destroyed && !p.decoupled)
                    .map(|(i, _)| i))
                .unwrap_or(0)
        };

        let mut priorities = vec![usize::MAX; n];

        // For each crossfeed-enabled decoupler in each stage, BFS from root
        // without that decoupler. Parts unreachable get priority = min(current, stage_idx).
        for (stage_idx, stage) in self.stages.iter().enumerate() {
            for &part_idx in stage {
                if part_idx >= n { continue; }
                if self.parts[part_idx].destroyed || self.parts[part_idx].decoupled { continue; }
                let is_crossfeed_decoupler = part_defs.get(&self.parts[part_idx].definition_id)
                    .map(|d| d.decoupler.is_some()).unwrap_or(false)
                    && self.parts[part_idx].crossfeed_enabled;
                if !is_crossfeed_decoupler { continue; }

                // BFS from root, skipping this decoupler
                let mut reached = vec![false; n];
                let mut queue = VecDeque::new();
                if root != part_idx {
                    reached[root] = true;
                    queue.push_back(root);
                }
                while let Some(idx) = queue.pop_front() {
                    for &neighbor in &connections[idx] {
                        if neighbor == part_idx || reached[neighbor] { continue; }
                        if self.parts[neighbor].destroyed || self.parts[neighbor].decoupled { continue; }
                        reached[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }

                // Unreachable parts get priority = min(current, stage_idx)
                for i in 0..n {
                    if !reached[i] && i != part_idx
                        && !self.parts[i].destroyed && !self.parts[i].decoupled
                    {
                        priorities[i] = priorities[i].min(stage_idx);
                    }
                }
            }
        }

        priorities
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

    /// Compute fuel zones using simulated decoupled state (for delta-v calculation).
    /// Same algorithm as compute_fuel_zones but uses a provided decoupled vec
    /// instead of reading from part state.
    fn compute_fuel_zones_simulated(
        &self,
        part_defs: &PartDefinitions,
        sim_decoupled: &[bool],
    ) -> Vec<usize> {
        use std::collections::VecDeque;

        let n = self.parts.len();
        // Build adjacency from weld hitbox overlap, skipping simulated-decoupled parts
        let mut connections = vec![Vec::new(); n];
        for i in 0..n {
            if sim_decoupled[i] { continue; }
            let Some(def_i) = part_defs.get(&self.parts[i].definition_id) else { continue };
            let weld_hw_i = def_i.weld_hitbox_width() / 2.0;
            let weld_hh_i = def_i.weld_hitbox_height() / 2.0;
            for j in (i + 1)..n {
                if sim_decoupled[j] { continue; }
                let Some(def_j) = part_defs.get(&self.parts[j].definition_id) else { continue };
                let weld_hw_j = def_j.weld_hitbox_width() / 2.0;
                let weld_hh_j = def_j.weld_hitbox_height() / 2.0;
                let dx = (self.parts[i].local_position[0] - self.parts[j].local_position[0]).abs();
                let dy = (self.parts[i].local_position[1] - self.parts[j].local_position[1]).abs();
                if dx < weld_hw_i + weld_hw_j && dy < weld_hh_i + weld_hh_j {
                    connections[i].push(j);
                    connections[j].push(i);
                }
            }
        }

        // Flood-fill zones; non-crossfeed decouplers are barriers
        let mut zones = vec![usize::MAX; n];
        let mut current_zone = 0;
        for start in 0..n {
            if zones[start] != usize::MAX || sim_decoupled[start] { continue; }
            let mut queue = VecDeque::new();
            zones[start] = current_zone;
            queue.push_back(start);
            while let Some(idx) = queue.pop_front() {
                let is_barrier = {
                    let def = part_defs.get(&self.parts[idx].definition_id);
                    def.map(|d| d.decoupler.is_some()).unwrap_or(false)
                        && !self.parts[idx].crossfeed_enabled
                };
                if is_barrier && idx != start { continue; }
                for &neighbor in &connections[idx] {
                    if zones[neighbor] != usize::MAX { continue; }
                    zones[neighbor] = current_zone;
                    queue.push_back(neighbor);
                }
            }
            current_zone += 1;
        }
        zones
    }

    /// Compute drain priorities using simulated decoupled state (for delta-v calculation).
    /// Same algorithm as compute_drain_priorities but uses sim_decoupled instead of part state.
    fn compute_drain_priorities_simulated(
        &self,
        part_defs: &PartDefinitions,
        sim_decoupled: &[bool],
    ) -> Vec<usize> {
        use std::collections::VecDeque;

        let n = self.parts.len();

        // Build adjacency (same as compute_fuel_zones_simulated)
        let mut connections = vec![Vec::new(); n];
        for i in 0..n {
            if sim_decoupled[i] { continue; }
            let Some(def_i) = part_defs.get(&self.parts[i].definition_id) else { continue };
            let weld_hw_i = def_i.weld_hitbox_width() / 2.0;
            let weld_hh_i = def_i.weld_hitbox_height() / 2.0;
            for j in (i + 1)..n {
                if sim_decoupled[j] { continue; }
                let Some(def_j) = part_defs.get(&self.parts[j].definition_id) else { continue };
                let weld_hw_j = def_j.weld_hitbox_width() / 2.0;
                let weld_hh_j = def_j.weld_hitbox_height() / 2.0;
                let dx = (self.parts[i].local_position[0] - self.parts[j].local_position[0]).abs();
                let dy = (self.parts[i].local_position[1] - self.parts[j].local_position[1]).abs();
                if dx < weld_hw_i + weld_hw_j && dy < weld_hh_i + weld_hh_j {
                    connections[i].push(j);
                    connections[j].push(i);
                }
            }
        }

        // Find root
        let root = if self.root_part_index < n && !sim_decoupled[self.root_part_index] {
            self.root_part_index
        } else {
            (0..n).filter(|&i| !sim_decoupled[i])
                .find(|&i| part_defs.get(&self.parts[i].definition_id).map(|d| d.pod.is_some()).unwrap_or(false))
                .or_else(|| (0..n).find(|&i| !sim_decoupled[i]))
                .unwrap_or(0)
        };

        let mut priorities = vec![usize::MAX; n];

        for (stage_idx, stage) in self.stages.iter().enumerate() {
            for &part_idx in stage {
                if part_idx >= n || sim_decoupled[part_idx] { continue; }
                let is_crossfeed_decoupler = part_defs.get(&self.parts[part_idx].definition_id)
                    .map(|d| d.decoupler.is_some()).unwrap_or(false)
                    && self.parts[part_idx].crossfeed_enabled;
                if !is_crossfeed_decoupler { continue; }

                // BFS from root, skipping this decoupler
                let mut reached = vec![false; n];
                let mut queue = VecDeque::new();
                if root != part_idx && !sim_decoupled[root] {
                    reached[root] = true;
                    queue.push_back(root);
                }
                while let Some(idx) = queue.pop_front() {
                    for &neighbor in &connections[idx] {
                        if neighbor == part_idx || reached[neighbor] { continue; }
                        if sim_decoupled[neighbor] { continue; }
                        reached[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }

                for i in 0..n {
                    if !reached[i] && i != part_idx && !sim_decoupled[i] {
                        priorities[i] = priorities[i].min(stage_idx);
                    }
                }
            }
        }

        priorities
    }

    /// Check if an engine is covered by a decoupler using simulated decoupled state.
    /// Used by calculate_stage_delta_v to determine which engines can fire after staging.
    fn is_engine_covered_simulated(
        &self,
        engine_idx: usize,
        part_defs: &PartDefinitions,
        sim_decoupled: &[bool],
    ) -> bool {
        let engine = &self.parts[engine_idx];
        let engine_bottom = engine.local_position[1] - engine.hitbox_half_extents[1];
        let engine_left = engine.local_position[0] - engine.hitbox_half_extents[0];
        let engine_right = engine.local_position[0] + engine.hitbox_half_extents[0];

        for (i, part) in self.parts.iter().enumerate() {
            if i == engine_idx || sim_decoupled[i] {
                continue;
            }
            let Some(def) = part_defs.get(&part.definition_id) else { continue };
            if def.decoupler.is_none() {
                continue;
            }
            let decoupler_top = part.local_position[1] + part.hitbox_half_extents[1];
            let decoupler_left = part.local_position[0] - part.hitbox_half_extents[0];
            let decoupler_right = part.local_position[0] + part.hitbox_half_extents[0];

            if (decoupler_top - engine_bottom).abs() < 0.3
                && engine_left < decoupler_right
                && engine_right > decoupler_left
            {
                return true;
            }
        }
        false
    }

    /// Stefan-Boltzmann constant (W/m^2/K^4)
    const STEFAN_BOLTZMANN: f64 = 5.670374419e-8;

    /// Sutton-Graves convective heating constant for N₂/O₂ atmosphere
    const SUTTON_GRAVES_K: f64 = 1.7415e-4;

    /// Update per-part temperatures from aerodynamic heating and radiative cooling.
    /// `airspeed_dir_local` is the unit vector of airspeed in vessel-local coordinates
    /// (i.e., already rotated by -vessel_rotation).
    pub fn update_part_temperatures(
        &mut self,
        dt: f64,
        density: f64,
        airspeed: f64,
        airspeed_dir_local: [f64; 2],
        part_defs: &PartDefinitions,
    ) {
        if airspeed < 1.0 || density < 1e-15 {
            // No significant heating — just radiative cooling
            for part in &mut self.parts {
                if part.destroyed || part.decoupled { continue; }
                let def = match part_defs.get(&part.definition_id) {
                    Some(d) => d,
                    None => continue,
                };
                if part.temperature > 300.0 {
                    let surface_area = 2.0 * (def.width() + def.height());
                    let q_out = def.emissivity * Self::STEFAN_BOLTZMANN * (part.temperature.powi(4) - 300.0_f64.powi(4)) * surface_area;
                    let mass_kg = (def.mass + part.resources.values().sum::<f64>() * 0.001) * 1000.0;
                    if mass_kg > 0.0 {
                        let d_temp = q_out / (mass_kg * def.specific_heat) * dt;
                        part.temperature = (part.temperature - d_temp).max(300.0);
                    }
                }
            }
            return;
        }

        // --- 1D interval occlusion along the airspeed axis ---
        // Project each part onto the velocity axis to determine ordering,
        // then compute how much of each part's perpendicular cross-section is exposed.

        // Velocity direction in local frame
        let vx = airspeed_dir_local[0];
        let vy = airspeed_dir_local[1];
        // Perpendicular axis
        let px = -vy;
        let py = vx;

        // For each non-destroyed/decoupled part, compute:
        // - projection_along: position projected onto velocity axis (front = most negative)
        // - perp_interval: [min, max] of the part projected onto perpendicular axis
        // - cross_section_width: width of the part perpendicular to velocity
        struct PartProjection {
            idx: usize,
            proj_along: f64,  // how far along velocity axis (more positive = more forward)
            perp_min: f64,
            perp_max: f64,
            surface_area: f64,
        }

        let mut projections: Vec<PartProjection> = Vec::new();

        for (i, part) in self.parts.iter().enumerate() {
            if part.destroyed || part.decoupled { continue; }
            let def = match part_defs.get(&part.definition_id) {
                Some(d) => d,
                None => continue,
            };

            let half_w = def.width() / 2.0;
            let half_h = def.height() / 2.0;
            let cx = part.local_position[0];
            let cy = part.local_position[1];

            // Project center onto velocity axis
            let proj_along = cx * vx + cy * vy;

            // Part corners projected onto both axes to find extent
            let corners = [
                (cx - half_w, cy - half_h),
                (cx + half_w, cy - half_h),
                (cx + half_w, cy + half_h),
                (cx - half_w, cy + half_h),
            ];

            let mut perp_min = f64::MAX;
            let mut perp_max = f64::MIN;
            for &(cx2, cy2) in &corners {
                let pp = cx2 * px + cy2 * py;
                perp_min = perp_min.min(pp);
                perp_max = perp_max.max(pp);
            }

            let surface_area = 2.0 * (def.width() + def.height());

            projections.push(PartProjection {
                idx: i,
                proj_along,
                perp_min,
                perp_max,
                surface_area,
            });
        }

        // Compute fairing-shielded parts.
        // A part is shielded if it falls within a non-decoupled fairing's envelope.
        let mut fairing_shielded: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for (fi, fpart) in self.parts.iter().enumerate() {
            if fpart.destroyed || fpart.decoupled { continue; }
            let Some(ref shape) = fpart.fairing_shape else { continue; };
            if shape.vertices.is_empty() { continue; }
            let Some(fdef) = part_defs.get(&fpart.definition_id) else { continue; };
            if fdef.fairing.is_none() { continue; }

            let gs = crate::parts::GRID_SQUARE_SIZE;
            let fairing_cx = fpart.local_position[0];
            let fairing_base_top = fpart.local_position[1] + fdef.hitbox_height() / 2.0;
            let base_half_w = fdef.width() / 2.0;

            // Build segment list: [(y_bottom, half_w_bottom, y_top, half_w_top), ...]
            let mut segs: Vec<(f64, f64, f64, f64)> = Vec::new();
            let mut prev_y = fairing_base_top;
            let mut prev_hw = base_half_w;
            for &(hw_grid, y_off_grid) in &shape.vertices {
                let seg_y = fairing_base_top + y_off_grid * gs;
                let seg_hw = hw_grid * gs;
                segs.push((prev_y, prev_hw, seg_y, seg_hw));
                prev_y = seg_y;
                prev_hw = seg_hw;
            }

            // Check each part
            for (pi, ppart) in self.parts.iter().enumerate() {
                if pi == fi || ppart.destroyed || ppart.decoupled { continue; }
                let Some(pdef) = part_defs.get(&ppart.definition_id) else { continue; };
                let part_cx = ppart.local_position[0];
                let part_cy = ppart.local_position[1];
                let part_half_h = pdef.hitbox_height() / 2.0;
                let part_half_w = pdef.hitbox_width() / 2.0;
                let part_bottom = part_cy - part_half_h;
                let part_top = part_cy + part_half_h;

                // Part must be between base top and fairing tip Y
                if part_bottom < fairing_base_top - 0.01 { continue; }
                let tip_y = segs.last().map(|s| s.2).unwrap_or(fairing_base_top);
                if part_top > tip_y + 0.01 { continue; }

                // Check if the part's width fits within the fairing width at the part's center Y
                let mut shielded = true;
                let check_y = part_cy;
                let mut envelope_hw = 0.0_f64;
                let mut found_seg = false;
                for &(y_bot, hw_bot, y_top, hw_top) in &segs {
                    if check_y >= y_bot - 0.01 && check_y <= y_top + 0.01 {
                        let t = if (y_top - y_bot).abs() > 0.001 {
                            (check_y - y_bot) / (y_top - y_bot)
                        } else {
                            0.5
                        };
                        envelope_hw = hw_bot + (hw_top - hw_bot) * t;
                        found_seg = true;
                        break;
                    }
                }
                if !found_seg { shielded = false; }
                if shielded {
                    let dx = (part_cx - fairing_cx).abs();
                    if dx + part_half_w > envelope_hw + 0.01 {
                        shielded = false;
                    }
                }
                if shielded {
                    fairing_shielded.insert(pi);
                }
            }
        }

        // Sort by projection along velocity axis (most forward = most positive first)
        // The leading edge has the largest dot product with the velocity direction
        projections.sort_by(|a, b| b.proj_along.partial_cmp(&a.proj_along).unwrap());

        // Track occluded perpendicular intervals (sorted, non-overlapping)
        let mut occluded: Vec<(f64, f64)> = Vec::new();

        for proj in &projections {
            // Calculate exposed fraction of this part's perpendicular interval
            let total_width = proj.perp_max - proj.perp_min;
            if total_width <= 0.0 { continue; }

            let exposed_width = exposed_interval_width(proj.perp_min, proj.perp_max, &occluded);
            let part = &self.parts[proj.idx];
            let def = match part_defs.get(&part.definition_id) {
                Some(d) => d,
                None => continue,
            };

            // Exposed area for heating (zero if fairing-shielded)
            let exposed_area = if fairing_shielded.contains(&proj.idx) {
                0.0
            } else {
                exposed_width * 0.5 // depth approximation (GRID_SQUARE_SIZE)
            };

            // Sutton-Graves heat input: q_in = K * sqrt(density) * airspeed^3 * exposed_area
            let q_in = Self::SUTTON_GRAVES_K * density.sqrt() * airspeed.powi(3) * exposed_area;

            // Radiative cooling (net radiation): q_out = emissivity * sigma * (T^4 - T_ambient^4) * surface_area
            let q_out = def.emissivity * Self::STEFAN_BOLTZMANN * (part.temperature.powi(4) - 300.0_f64.powi(4)) * proj.surface_area;

            let mass_kg = (def.mass + part.resources.values().sum::<f64>() * 0.001) * 1000.0;
            if mass_kg > 0.0 {
                let d_temp = (q_in - q_out) / (mass_kg * def.specific_heat) * dt;
                self.parts[proj.idx].temperature = (self.parts[proj.idx].temperature + d_temp).max(300.0);
            }

            // Add this part to occluded intervals
            insert_interval(&mut occluded, proj.perp_min, proj.perp_max);
        }
    }

    /// Check for parts exceeding their heat tolerance and destroy them.
    /// Returns indices of destroyed parts (for staging cleanup).
    pub fn destroy_overheated_parts(&mut self) -> Vec<usize> {
        let mut destroyed = Vec::new();
        for (i, part) in self.parts.iter_mut().enumerate() {
            if part.destroyed || part.decoupled { continue; }
            if part.temperature >= part.max_heat_tolerance {
                part.destroyed = true;
                destroyed.push(i);
                log::info!("Part {} destroyed by overheating at {}K (tolerance: {}K)",
                    part.definition_id, part.temperature as i32, part.max_heat_tolerance as i32);
            }
        }
        destroyed
    }

    /// After parts are destroyed, check if the vessel has split into disconnected components.
    /// Uses BFS from root_part_index through weld connections. Any non-destroyed,
    /// non-decoupled parts unreachable from root form debris vessels.
    /// Returns a list of (debris_vessel, com_offset_local) for each disconnected component.
    pub fn check_and_split(&mut self, part_defs: &PartDefinitions) -> Vec<(FlightVessel, [f64; 2])> {
        use std::collections::VecDeque;

        let n = self.parts.len();
        let connections = self.find_weld_connections(part_defs);

        // BFS from root to find all reachable parts
        let mut reachable = vec![false; n];

        // If root part is destroyed, find the first non-destroyed non-decoupled part as new root
        if self.root_part_index < n && (self.parts[self.root_part_index].destroyed || self.parts[self.root_part_index].decoupled) {
            // Try to find a pod first, then any part
            let new_root = self.parts.iter().enumerate()
                .filter(|(_, p)| !p.destroyed && !p.decoupled)
                .find(|(_, p)| {
                    part_defs.get(&p.definition_id).map(|d| d.pod.is_some()).unwrap_or(false)
                })
                .map(|(i, _)| i)
                .or_else(|| {
                    self.parts.iter().enumerate()
                        .find(|(_, p)| !p.destroyed && !p.decoupled)
                        .map(|(i, _)| i)
                });
            match new_root {
                Some(r) => self.root_part_index = r,
                None => return Vec::new(), // All parts destroyed
            }
        }

        let root = self.root_part_index;
        if root >= n || self.parts[root].destroyed || self.parts[root].decoupled {
            return Vec::new();
        }

        let mut queue = VecDeque::new();
        reachable[root] = true;
        queue.push_back(root);
        while let Some(idx) = queue.pop_front() {
            for &neighbor in &connections[idx] {
                if !reachable[neighbor] && !self.parts[neighbor].destroyed && !self.parts[neighbor].decoupled {
                    reachable[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }

        // Find disconnected components (non-reachable, non-destroyed, non-decoupled parts)
        let unreachable_indices: Vec<usize> = (0..n)
            .filter(|&i| !reachable[i] && !self.parts[i].destroyed && !self.parts[i].decoupled)
            .collect();

        if unreachable_indices.is_empty() {
            return Vec::new();
        }

        // Group unreachable parts into connected components
        let mut visited = vec![false; n];
        let mut components: Vec<Vec<usize>> = Vec::new();

        for &start in &unreachable_indices {
            if visited[start] { continue; }
            let mut component = Vec::new();
            let mut q = VecDeque::new();
            visited[start] = true;
            q.push_back(start);
            while let Some(idx) = q.pop_front() {
                component.push(idx);
                for &neighbor in &connections[idx] {
                    if !visited[neighbor] && !self.parts[neighbor].destroyed && !self.parts[neighbor].decoupled {
                        if unreachable_indices.contains(&neighbor) {
                            visited[neighbor] = true;
                            q.push_back(neighbor);
                        }
                    }
                }
            }
            components.push(component);
        }

        // Create debris vessels for each component
        let mut result = Vec::new();
        for component in &components {
            // Calculate COM of this component
            let mut debris_mass = 0.0;
            let mut debris_com = [0.0f64, 0.0];
            for &i in component {
                let part = &self.parts[i];
                let base_mass = part_defs.get(&part.definition_id)
                    .map(|d| d.mass).unwrap_or(0.0);
                let resource_mass: f64 = part.resources.values().sum::<f64>() * 0.001;
                let pm = base_mass + resource_mass;
                debris_mass += pm;
                debris_com[0] += part.local_position[0] * pm;
                debris_com[1] += part.local_position[1] * pm;
            }
            if debris_mass <= 0.0 { continue; }
            debris_com[0] /= debris_mass;
            debris_com[1] /= debris_mass;

            // Clone parts, shift to local COM, clear state
            let mut debris_parts: Vec<FlightPart> = component.iter().map(|&i| {
                let mut part = self.parts[i].clone();
                part.local_position[0] -= debris_com[0];
                part.local_position[1] -= debris_com[1];
                part
            }).collect();

            // Disable engines on debris
            for part in &mut debris_parts {
                part.engine_active = false;
                part.engine_enabled = false;
            }

            // Calculate MOI
            let mut moi = 0.0;
            for part in &debris_parts {
                let base_mass = part_defs.get(&part.definition_id)
                    .map(|d| d.mass).unwrap_or(0.0);
                let resource_mass: f64 = part.resources.values().sum::<f64>() * 0.001;
                let pm = base_mass + resource_mass;
                let r_sq = part.local_position[0].powi(2) + part.local_position[1].powi(2);
                moi += pm * r_sq;
                let (w, h) = part_defs.get(&part.definition_id)
                    .map(|d| (d.width(), d.height()))
                    .unwrap_or((1.0, 1.0));
                moi += pm * (w * w + h * h) / 12.0;
            }
            moi = moi.max(0.1);

            let dry_mass = debris_parts.iter()
                .map(|p| part_defs.get(&p.definition_id).map(|d| d.mass).unwrap_or(0.0))
                .sum();

            let debris_vessel = FlightVessel {
                rel_position: [0.0, 0.0],
                rel_velocity: [0.0, 0.0],
                rotation: self.rotation,
                rotational_velocity: 0.0,
                soi_body: self.soi_body,
                parts: debris_parts,
                root_part_index: 0,
                total_mass: debris_mass,
                dry_mass,
                center_of_mass: [0.0, 0.0],
                max_thrust_vac: 0.0,
                max_thrust_asl: 0.0,
                moment_of_inertia: moi,
                throttle: 0.0,
                on_rails: false,
                stages: Vec::new(),
                current_stage: 0,
                last_decouple_force: 0.0,
            };

            result.push((debris_vessel, debris_com));

            // Mark source parts as destroyed in the parent vessel
            for &i in component {
                self.parts[i].destroyed = true;
            }
        }

        result
    }

    /// Calculate per-stage delta-v (vacuum) using the Tsiolkovsky rocket equation.
    /// Simulates staging sequentially: decouplers fire, engines activate.
    /// Fuel zones (divided by non-crossfeed decouplers) determine which fuel
    /// is accessible to active engines in each stage.
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

            // 3. Compute fuel zones and drain priorities with simulated decoupled state
            let zones = self.compute_fuel_zones_simulated(part_defs, &decoupled);
            let sim_priorities = self.compute_drain_priorities_simulated(part_defs, &decoupled);

            // 4. Find zones containing active (non-decoupled, non-covered) engines
            let engine_zones: std::collections::HashSet<usize> = (0..self.parts.len())
                .filter(|&i| !decoupled[i] && engines_enabled[i] && zones[i] != usize::MAX
                    && !self.is_engine_covered_simulated(i, part_defs, &decoupled))
                .map(|i| zones[i])
                .collect();

            // 5. Calculate wet mass of ALL remaining parts, but only count
            //    fuel from min-priority tanks in engine zones as burnable
            let mut zone_min_priority: HashMap<usize, usize> = HashMap::new();
            for i in 0..self.parts.len() {
                if decoupled[i] || fuel_remaining[i] <= 0.0 { continue; }
                if zones[i] != usize::MAX && engine_zones.contains(&zones[i]) {
                    let entry = zone_min_priority.entry(zones[i]).or_insert(usize::MAX);
                    *entry = (*entry).min(sim_priorities[i]);
                }
            }

            let mut wet_mass = 0.0;
            let mut burnable_fuel = 0.0;
            for i in 0..self.parts.len() {
                if decoupled[i] { continue; }
                let base_mass = part_defs.get(&self.parts[i].definition_id)
                    .map(|d| d.mass).unwrap_or(0.0);
                wet_mass += base_mass + fuel_remaining[i];

                if zones[i] != usize::MAX && engine_zones.contains(&zones[i]) {
                    let zmp = zone_min_priority.get(&zones[i]).copied().unwrap_or(usize::MAX);
                    if sim_priorities[i] == zmp {
                        burnable_fuel += fuel_remaining[i];
                    }
                }
            }

            // 6. Calculate thrust-weighted average Isp (skip covered engines)
            let mut total_thrust = 0.0;
            let mut weighted_isp = 0.0;
            for i in 0..self.parts.len() {
                if decoupled[i] || !engines_enabled[i] { continue; }
                if self.is_engine_covered_simulated(i, part_defs, &decoupled) { continue; }
                total_thrust += self.parts[i].engine_thrust_vac;
                weighted_isp += self.parts[i].engine_thrust_vac * self.parts[i].engine_isp_vac;
            }
            let isp = if total_thrust > 0.0 { weighted_isp / total_thrust } else { 0.0 };

            // 7. Δv = Isp * g0 * ln(wet / dry)
            let dry_mass = wet_mass - burnable_fuel;
            let dv = if isp > 0.0 && dry_mass > 0.0 && wet_mass > dry_mass {
                isp * g0 * (wet_mass / dry_mass).ln()
            } else {
                0.0
            };
            stage_dvs.push(dv);

            // 8. Consume only min-priority burnable fuel (in engine zones)
            for i in 0..self.parts.len() {
                if decoupled[i] { continue; }
                if zones[i] != usize::MAX && engine_zones.contains(&zones[i]) {
                    let zmp = zone_min_priority.get(&zones[i]).copied().unwrap_or(usize::MAX);
                    if sim_priorities[i] == zmp {
                        fuel_remaining[i] = 0.0;
                    }
                }
            }
        }

        stage_dvs
    }

    /// Extract decoupled fairing shells into two half-shell debris vessels.
    /// The base disc stays on the vessel; only the shell splits off.
    /// Must be called BEFORE extract_decoupled_parts().
    /// Returns Vec of (debris_vessel, com_offset_local, FairingHalf).
    pub fn extract_fairing_halves(&mut self, part_defs: &PartDefinitions) -> Vec<(FlightVessel, [f64; 2], crate::parts::FairingHalf)> {
        use crate::parts::FairingHalf;
        let mut result = Vec::new();

        // Find decoupled fairing parts that have a shell to split
        let fairing_indices: Vec<usize> = self.parts.iter()
            .enumerate()
            .filter(|(_, p)| p.decoupled && !p.destroyed && p.fairing_shape.is_some() && p.fairing_half.is_none())
            .map(|(i, _)| i)
            .collect();

        for idx in fairing_indices {
            let part = &self.parts[idx];
            let com_offset = part.local_position;
            let base_mass = part_defs.get(&part.definition_id).map(|d| d.mass).unwrap_or(0.0);
            // Shell is ~20% of total fairing mass, split between two halves
            let half_shell_mass = (base_mass * 0.1).max(0.001);

            for &half in &[FairingHalf::Left, FairingHalf::Right] {
                let mut debris_part = part.clone();
                debris_part.local_position = [0.0, 0.0];
                debris_part.decoupled = false;
                debris_part.fairing_half = Some(half);

                let moi = {
                    let (w, h) = part_defs.get(&debris_part.definition_id)
                        .map(|d| (d.width(), d.height()))
                        .unwrap_or((1.0, 1.0));
                    (half_shell_mass * (w * w + h * h) / 12.0).max(0.1)
                };

                let debris_vessel = FlightVessel {
                    rel_position: [0.0, 0.0],
                    rel_velocity: [0.0, 0.0],
                    rotation: self.rotation,
                    rotational_velocity: 0.0,
                    soi_body: self.soi_body,
                    parts: vec![debris_part],
                    root_part_index: 0,
                    total_mass: half_shell_mass,
                    dry_mass: half_shell_mass,
                    center_of_mass: [0.0, 0.0],
                    max_thrust_vac: 0.0,
                    max_thrust_asl: 0.0,
                    moment_of_inertia: moi,
                    throttle: 0.0,
                    on_rails: false,
                    stages: Vec::new(),
                    current_stage: 0,
                    last_decouple_force: 0.0,
                };

                result.push((debris_vessel, com_offset, half));
            }

            // Keep the base disc on the vessel: un-decouple and strip the shell
            self.parts[idx].decoupled = false;
            self.parts[idx].fairing_shape = None;
        }

        result
    }

    /// Extract decoupled parts into a new FlightVessel (debris).
    /// Call after activate_next_stage() or manual decouple marks parts as `decoupled = true`.
    /// Returns (debris_vessel, com_offset_local) if any decoupled parts were extracted.
    /// The com_offset_local is in vessel-local coordinates (before rotation) so the caller
    /// can place the debris ship at the correct world position.
    pub fn extract_decoupled_parts(&mut self, part_defs: &PartDefinitions) -> Option<(FlightVessel, [f64; 2])> {
        let decoupled_indices: Vec<usize> = self.parts.iter()
            .enumerate()
            .filter(|(_, p)| p.decoupled && !p.destroyed)
            .map(|(i, _)| i)
            .collect();

        if decoupled_indices.is_empty() {
            return None;
        }

        // Compute COM of the decoupled parts (in the current vessel's local frame)
        let mut debris_mass = 0.0;
        let mut debris_com = [0.0f64, 0.0];
        for &i in &decoupled_indices {
            let part = &self.parts[i];
            let base_mass = part_defs.get(&part.definition_id)
                .map(|d| d.mass).unwrap_or(0.0);
            let resource_mass: f64 = part.resources.values().sum::<f64>() * 0.001;
            let pm = base_mass + resource_mass;
            debris_mass += pm;
            debris_com[0] += part.local_position[0] * pm;
            debris_com[1] += part.local_position[1] * pm;
        }
        if debris_mass <= 0.0 {
            return None;
        }
        debris_com[0] /= debris_mass;
        debris_com[1] /= debris_mass;

        // Clone decoupled parts, shift them relative to their own COM, clear decoupled flag
        let mut debris_parts: Vec<FlightPart> = decoupled_indices.iter().map(|&i| {
            let mut part = self.parts[i].clone();
            part.local_position[0] -= debris_com[0];
            part.local_position[1] -= debris_com[1];
            part.decoupled = false;
            part
        }).collect();

        // Mark originals as destroyed so they aren't re-extracted on future staging events
        for &i in &decoupled_indices {
            self.parts[i].destroyed = true;
        }

        // Disable all engines on debris (no staging system)
        for part in &mut debris_parts {
            part.engine_active = false;
            part.engine_enabled = false;
        }

        // Calculate moment of inertia for debris
        let mut moi = 0.0;
        for part in &debris_parts {
            let base_mass = part_defs.get(&part.definition_id)
                .map(|d| d.mass).unwrap_or(0.0);
            let resource_mass: f64 = part.resources.values().sum::<f64>() * 0.001;
            let pm = base_mass + resource_mass;
            let r_sq = part.local_position[0].powi(2) + part.local_position[1].powi(2);
            moi += pm * r_sq;
            let (w, h) = part_defs.get(&part.definition_id)
                .map(|d| (d.width(), d.height()))
                .unwrap_or((1.0, 1.0));
            moi += pm * (w * w + h * h) / 12.0;
        }
        moi = moi.max(0.1);

        let dry_mass = debris_parts.iter()
            .map(|p| part_defs.get(&p.definition_id).map(|d| d.mass).unwrap_or(0.0))
            .sum();

        let debris_vessel = FlightVessel {
            rel_position: [0.0, 0.0], // Will be set by caller
            rel_velocity: [0.0, 0.0],
            rotation: self.rotation,
            rotational_velocity: 0.0,
            soi_body: self.soi_body,
            parts: debris_parts,
            root_part_index: 0,
            total_mass: debris_mass,
            dry_mass,
            center_of_mass: [0.0, 0.0],
            max_thrust_vac: 0.0,
            max_thrust_asl: 0.0,
            moment_of_inertia: moi,
            throttle: 0.0,
            on_rails: false,
            stages: Vec::new(),   // Debris can't stage
            current_stage: 0,
            last_decouple_force: 0.0,
        };

        Some((debris_vessel, debris_com))
    }

    /// Activate next stage (enables engines and fires decouplers in that stage)
    pub fn activate_next_stage(&mut self, part_defs: &PartDefinitions) -> bool {
        if self.current_stage >= self.stages.len() {
            return false;
        }

        let stage_parts = self.stages[self.current_stage].clone();
        self.last_decouple_force = 0.0;

        for &part_idx in &stage_parts {
            if part_idx >= self.parts.len() || self.parts[part_idx].decoupled {
                continue;
            }

            // Enable engines in this stage
            if self.parts[part_idx].propellant_type.is_some() {
                self.parts[part_idx].engine_enabled = true;
            }

            // Fire fairings: decouple just the fairing base (parts inside stay)
            let def = part_defs.get(&self.parts[part_idx].definition_id);
            if let Some(def) = def {
                if let Some(ref fairing_data) = def.fairing {
                    if fairing_data.ejection_force > self.last_decouple_force {
                        self.last_decouple_force = fairing_data.ejection_force;
                    }
                    // Mark the fairing base as decoupled — parts above stay connected
                    self.parts[part_idx].decoupled = true;
                }
            }

            // Fire decouplers: decouple all parts below the decoupler
            let def = part_defs.get(&self.parts[part_idx].definition_id);
            if let Some(def) = def {
                if let Some(ref dec_data) = def.decoupler {
                    // Store max ejection force from decouplers in this stage
                    if dec_data.ejection_force > self.last_decouple_force {
                        self.last_decouple_force = dec_data.ejection_force;
                    }

                    // Mark the decoupler itself as decoupled
                    self.parts[part_idx].decoupled = true;

                    if !dec_data.is_radial {
                        // Stack decoupler: Y-based decoupling + adapter/fairing logic
                        let dec_x = self.parts[part_idx].local_position[0];
                        let decoupler_bottom = self.parts[part_idx].local_position[1]
                            - def.hitbox_height() / 2.0;
                        let decoupler_top = self.parts[part_idx].local_position[1]
                            + def.hitbox_height() / 2.0;
                        let dec_half_w = def.width() / 2.0;

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

                        // Also decouple parts beside the adapter/fairing.
                        // The adapter zone spans from the visual ring top to the
                        // tank/pod bottom above. (The ring is shorter than the hitbox;
                        // the gap between ring top and hitbox top is the adapter space.)
                        let ring_top = self.parts[part_idx].local_position[1]
                            - def.hitbox_height() / 2.0 + def.height();

                        // Find the closest aligned tank/pod above to get fairing width
                        let mut adapter_top = decoupler_top;
                        let mut tank_half_w = dec_half_w;
                        let mut best_dist = f64::MAX;
                        for j in 0..self.parts.len() {
                            if self.parts[j].decoupled || self.parts[j].destroyed { continue; }
                            let Some(jdef) = part_defs.get(&self.parts[j].definition_id) else { continue };
                            if jdef.tank.is_none() && jdef.pod.is_none() { continue; }
                            if (self.parts[j].local_position[0] - dec_x).abs() > 0.01 { continue; }
                            let j_bottom = self.parts[j].local_position[1] - jdef.hitbox_height() / 2.0;
                            if j_bottom < ring_top - 0.01 { continue; }
                            let dist = j_bottom - ring_top;
                            if dist < best_dist {
                                best_dist = dist;
                                adapter_top = j_bottom;
                                tank_half_w = jdef.width() / 2.0;
                            }
                        }

                        // Decouple parts beside the fairing. A part qualifies if its
                        // center Y is in the adapter zone and it's off the center axis
                        // but within reach of the fairing edge.
                        let max_half_w = dec_half_w.max(tank_half_w);
                        let margin = crate::parts::GRID_SQUARE_SIZE;
                        for j in 0..self.parts.len() {
                            if j == part_idx || self.parts[j].decoupled || self.parts[j].destroyed {
                                continue;
                            }
                            let jy = self.parts[j].local_position[1];
                            let jx = self.parts[j].local_position[0];
                            if jy < ring_top - 0.01 || jy > adapter_top + 0.01 { continue; }
                            let dx = (jx - dec_x).abs();
                            // Off center axis, but close enough to be on the fairing
                            if dx > 0.1 && dx < max_half_w + margin {
                                self.parts[j].decoupled = true;
                            }
                        }
                    }
                    // Radial decouplers: only mark self as decoupled (done above).
                    // The BFS in decouple_disconnected() handles disconnecting
                    // parts that are only reachable through the decoupled radial.
                }
            }
        }

        // After Y-based decoupling, also decouple any parts that are no longer
        // connected to the vessel root (e.g., RCS blocks side-mounted on a decoupler).
        self.decouple_disconnected(part_defs);

        self.current_stage += 1;
        true
    }

    /// Mark any non-decoupled parts that are disconnected from the root as decoupled.
    /// Uses BFS through weld connections (which skip already-decoupled parts).
    fn decouple_disconnected(&mut self, part_defs: &PartDefinitions) {
        use std::collections::VecDeque;

        let n = self.parts.len();
        let connections = self.find_weld_connections(part_defs);

        // Find a valid root (prefer current root, then any pod, then any part)
        let root = if self.root_part_index < n
            && !self.parts[self.root_part_index].destroyed
            && !self.parts[self.root_part_index].decoupled
        {
            self.root_part_index
        } else {
            match self.parts.iter().enumerate()
                .filter(|(_, p)| !p.destroyed && !p.decoupled)
                .find(|(_, p)| part_defs.get(&p.definition_id).map(|d| d.pod.is_some()).unwrap_or(false))
                .map(|(i, _)| i)
                .or_else(|| self.parts.iter().enumerate()
                    .find(|(_, p)| !p.destroyed && !p.decoupled)
                    .map(|(i, _)| i))
            {
                Some(r) => r,
                None => return,
            }
        };

        // BFS from root
        let mut reachable = vec![false; n];
        let mut queue = VecDeque::new();
        reachable[root] = true;
        queue.push_back(root);
        while let Some(idx) = queue.pop_front() {
            for &neighbor in &connections[idx] {
                if !reachable[neighbor] && !self.parts[neighbor].destroyed && !self.parts[neighbor].decoupled {
                    reachable[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }

        // Mark unreachable parts as decoupled
        for i in 0..n {
            if !reachable[i] && !self.parts[i].destroyed && !self.parts[i].decoupled {
                self.parts[i].decoupled = true;
            }
        }
    }
}

/// Calculate the exposed (non-occluded) width of an interval [min, max]
/// given a set of sorted, non-overlapping occluded intervals.
fn exposed_interval_width(min: f64, max: f64, occluded: &[(f64, f64)]) -> f64 {
    let mut exposed = max - min;
    for &(occ_min, occ_max) in occluded {
        // Skip intervals entirely outside our range
        if occ_max <= min || occ_min >= max { continue; }
        // Clamp to our interval
        let overlap_min = occ_min.max(min);
        let overlap_max = occ_max.min(max);
        exposed -= overlap_max - overlap_min;
    }
    exposed.max(0.0)
}

/// Insert an interval into a sorted, non-overlapping interval list, merging overlaps.
fn insert_interval(intervals: &mut Vec<(f64, f64)>, min: f64, max: f64) {
    let mut new_min = min;
    let mut new_max = max;
    let mut i = 0;
    while i < intervals.len() {
        if intervals[i].1 < new_min {
            i += 1;
            continue;
        }
        if intervals[i].0 > new_max {
            break;
        }
        // Overlapping or adjacent — merge
        new_min = new_min.min(intervals[i].0);
        new_max = new_max.max(intervals[i].1);
        intervals.remove(i);
    }
    intervals.insert(i, (new_min, new_max));
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
            rcs_thrust: 0.0,
            rcs_isp: 0.0,
            rcs_mass_flow_rate: 0.0,
            destroyed: false,
            decoupled: false,
            crossfeed_enabled: false,
            temperature: 300.0,
            max_heat_tolerance: 2000.0,
            fairing_shape: None,
            fairing_half: None,
            electricity: 0.0,
            max_electricity: 0.0,
        }],
        root_part_index: 0,
        total_mass: 2.0,
        dry_mass: 2.0,
        center_of_mass: [0.0, 0.0],
        max_thrust_vac: 200.0,
        max_thrust_asl: 150.0,
        moment_of_inertia: 1.0,
        throttle: 0.0,
        on_rails: false,
        stages: Vec::new(),
        current_stage: 0,
        last_decouple_force: 0.0,
    }
}
