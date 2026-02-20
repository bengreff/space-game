use crate::bodies::{SolarSystem, G, Orbit};
use crate::render::ManeuverNode;

mod orbit;
mod patched_conics;
mod soi;

/// Autopilot target direction for ship rotation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AutopilotTarget {
    #[default]
    Off,
    Prograde,
    Retrograde,
    RadialIn,
    RadialOut,
    ManeuverNode,
}

/// Ship size in meters (physics space) - fallback when no vessel loaded
pub const SHIP_SIZE: f64 = 10.0;

/// Maximum thrust acceleration in m/s² - fallback when no vessel loaded
pub const MAX_THRUST_ACCELERATION: f64 = 20.0;

/// Rotation acceleration in radians/second² (30 degrees/s/s) - fallback
pub const ROTATION_ACCEL: f64 = 30.0 * std::f64::consts::PI / 180.0;

/// Physics data derived from a FlightVessel, passed into Ship::update()
#[derive(Clone, Debug)]
pub struct VesselPhysicsData {
    pub total_mass: f64,       // tonnes
    pub max_thrust_vac: f64,   // kN
    pub max_thrust_asl: f64,   // kN
    pub vessel_height: f64,    // meters (half-height for collision)
    pub bottom_extent: f64,    // meters (COM to bottom, for surface placement)
    pub moment_of_inertia: f64,
    pub torque: f64,           // kN·m from reaction wheels
    pub gimbal_torque: f64,    // kN·m from gimbaled engines (signed: + = CCW)
    pub vessel_half_width: f64, // meters (half-width for cross-section)
}

/// Rotation drag (natural deceleration) in radians/second² (9 degrees/s/s)
pub const ROTATION_DRAG: f64 = 9.0 * std::f64::consts::PI / 180.0;

/// Maximum rotation speed in radians/second (for reference, kept for compatibility)
pub const ROTATION_SPEED: f64 = 2.0;

/// Throttle change rate per second (0-1 scale)
pub const THROTTLE_RATE: f64 = 0.25;

/// Time warp threshold for on-rails mode (max physics warp)
pub const RAILS_WARP_THRESHOLD: f64 = 10.0;

/// Maximum physics timestep for accurate integration (seconds)
const MAX_PHYSICS_DT: f64 = 0.01;

/// Aerodynamic drag coefficient (blunt body)
const DRAG_COEFFICIENT: f64 = 0.4;

/// Specific heat capacity of vessel structure (J/kg/K, aluminum)
const VESSEL_SPECIFIC_HEAT: f64 = 900.0;

/// Stefan-Boltzmann constant (W/m²/K⁴)
const STEFAN_BOLTZMANN: f64 = 5.670374419e-8;

/// Emissivity for radiative cooling
const VESSEL_EMISSIVITY: f64 = 0.8;

/// Temperature at which the vessel is destroyed (K)
pub const VESSEL_DESTRUCTION_TEMP: f64 = 2000.0;

/// Ambient / starting temperature (K)
pub const AMBIENT_TEMPERATURE: f64 = 300.0;

/// Heat transfer coefficient for convective heating (Sutton-Graves simplified)
const HEAT_COEFFICIENT: f64 = 1.0e-4;

/// Maximum number of SOI changes to predict in patched conics
pub const MAX_PATCHED_CONICS: usize = 1;

/// Number of samples for SOI intersection detection
pub(crate) const SOI_INTERSECTION_SAMPLES: usize = 200;

/// SOI exit detection threshold (fraction of SOI radius)
pub(crate) const SOI_EXIT_THRESHOLD: f64 = 0.99;

/// SOI entry detection threshold (fraction of SOI radius)
pub(crate) const SOI_ENTRY_THRESHOLD: f64 = 1.01;

/// Minimum time threshold for SOI intersection (seconds)
pub(crate) const MIN_INTERSECTION_TIME: f64 = 0.01;

/// Binary search iterations for precise SOI crossing
pub(crate) const BINARY_SEARCH_ITERATIONS: usize = 50;

/// Binary search iterations for SOI intersection refinement
pub(crate) const SOI_REFINE_ITERATIONS: usize = 10;

/// Small angle margin for hyperbolic orbit calculations
pub(crate) const HYPERBOLIC_ANGLE_MARGIN: f64 = 0.01;

/// Ship state - either flying through space or landed on a body
#[derive(Clone, Debug)]
pub enum ShipState {
    Flying,
    Landed { body_index: usize, surface_angle: f64 },
}

/// Keyboard input state for ship controls
#[derive(Clone, Debug, Default)]
pub struct ShipInput {
    pub throttle_up: bool,
    pub throttle_down: bool,
    pub throttle_full: bool,
    pub throttle_zero: bool,
    pub rotate_left: bool,
    pub rotate_right: bool,
}

/// Cached orbit with mean anomaly for on-rails propagation
#[derive(Clone, Debug)]
pub struct ShipOrbit {
    pub orbit: Orbit,
    pub mean_anomaly: f64,
    pub parent_idx: usize,
    pub retrograde: bool,
}

/// Full orbital information for UI display
#[derive(Clone, Debug)]
pub struct OrbitalInfo {
    pub orbit: Orbit,
    pub parent_idx: usize,
    pub apoapsis: f64,
    pub periapsis: f64,
    pub orbital_period: f64,
    pub time_to_apoapsis: f64,
    pub time_to_periapsis: f64,
    pub retrograde: bool,
}

/// A single segment of a patched conics trajectory
#[derive(Clone, Debug)]
pub struct PatchedConicSegment {
    pub orbit: Orbit,
    pub parent_idx: usize,
    pub retrograde: bool,
    pub start_true_anomaly: f64,
    pub end_true_anomaly: Option<f64>,
    pub start_time: f64,
    pub end_time: Option<f64>,
}

/// Full patched conics trajectory
#[derive(Clone, Debug)]
pub struct PatchedTrajectory {
    pub segments: Vec<PatchedConicSegment>,
}

/// A flyable spaceship
/// Position and velocity are stored RELATIVE to the current SOI body
#[derive(Clone, Debug)]
pub struct Ship {
    pub rel_position: [f64; 2],
    pub rel_velocity: [f64; 2],
    pub rotation: f64,
    pub rotational_velocity: f64,  // rad/s
    pub throttle: f64,
    pub state: ShipState,
    pub color: [f32; 4],
    pub soi_body: usize,
    pub on_rails: bool,
    pub(crate) cached_orbit: Option<ShipOrbit>,
    /// Cached patched trajectory to avoid recalculating every frame
    pub(crate) cached_trajectory: Option<PatchedTrajectory>,
    /// Frame counter when trajectory was last calculated
    pub(crate) trajectory_calc_frame: u64,
    /// SOI body when trajectory was calculated (for cache invalidation)
    pub(crate) trajectory_soi_body: usize,
    /// Current frame counter (incremented each update)
    pub(crate) frame_counter: u64,
    /// Current vessel temperature (Kelvin)
    pub temperature: f64,
    /// Current aerodynamic heat flux (W/m²) for HUD display
    pub heat_flux: f64,
}

impl Ship {
    /// Create a ship spawned on Earth's surface (Earth is at index 3)
    pub fn spawn_on_earth(solar_system: &SolarSystem) -> Self {
        let earth_index = 3;
        let earth = &solar_system.bodies[earth_index];
        let earth_radius = earth.radius;
        let earth_mass = earth.mass;

        // Spawn in Low Earth Orbit (LEO) - 400 km altitude (ISS orbit)
        let leo_altitude = 4.0e5;
        let orbital_radius = earth_radius + leo_altitude;

        // Position at top of orbit (angle = PI/2)
        let orbit_angle = std::f64::consts::FRAC_PI_2;
        let rel_position = [
            orbital_radius * orbit_angle.cos(),
            orbital_radius * orbit_angle.sin(),
        ];

        // Circular orbital velocity: v = sqrt(GM/r)
        let orbital_velocity = (G * earth_mass / orbital_radius).sqrt();

        // Velocity perpendicular to position (prograde direction)
        let vel_angle = orbit_angle + std::f64::consts::FRAC_PI_2;
        let rel_velocity = [
            orbital_velocity * vel_angle.cos(),
            orbital_velocity * vel_angle.sin(),
        ];

        let rotation = vel_angle;

        Self {
            rel_position,
            rel_velocity,
            rotation,
            rotational_velocity: 0.0,
            throttle: 0.0,
            state: ShipState::Flying,
            color: [1.0, 0.2, 0.2, 1.0],
            soi_body: earth_index,
            on_rails: false,
            cached_orbit: None,
            cached_trajectory: None,
            trajectory_calc_frame: 0,
            trajectory_soi_body: earth_index,
            frame_counter: 0,
            temperature: AMBIENT_TEMPERATURE,
            heat_flux: 0.0,
        }
    }

    /// Get absolute position (for rendering)
    pub fn absolute_position(&self, solar_system: &SolarSystem) -> [f64; 2] {
        let soi_pos = solar_system.body_position(self.soi_body);
        [
            soi_pos[0] + self.rel_position[0],
            soi_pos[1] + self.rel_position[1],
        ]
    }

    /// Get absolute velocity (for rendering/SOI transitions)
    pub fn absolute_velocity(&self, solar_system: &SolarSystem) -> [f64; 2] {
        let soi_vel = self.get_body_velocity(self.soi_body, solar_system);
        [
            soi_vel[0] + self.rel_velocity[0],
            soi_vel[1] + self.rel_velocity[1],
        ]
    }

    /// Check if the ship is currently inside an atmosphere
    pub fn in_atmosphere(&self, solar_system: &SolarSystem) -> bool {
        let soi_body = &solar_system.bodies[self.soi_body];
        if let Some(ref atmo) = soi_body.atmosphere {
            let dist = (self.rel_position[0].powi(2) + self.rel_position[1].powi(2)).sqrt();
            let altitude = dist - soi_body.radius;
            altitude >= 0.0 && altitude < atmo.visible_height()
        } else {
            false
        }
    }

    /// Check if the ship is below the landing altitude of the current SOI body.
    /// Works for both atmospheric and airless bodies.
    /// Returns true for negative altitudes (inside the body surface), which can
    /// happen for on-rails vessels whose Keplerian orbits pass through the body.
    pub fn below_landing_altitude(&self, solar_system: &SolarSystem) -> bool {
        let soi_body = &solar_system.bodies[self.soi_body];
        let dist = (self.rel_position[0].powi(2) + self.rel_position[1].powi(2)).sqrt();
        let altitude = dist - soi_body.radius;
        altitude < soi_body.landing_altitude()
    }

    /// Check if the ship's orbit periapsis dips below the planet's surface.
    /// Used together with position checks to delete vessels that will
    /// inevitably hit the surface.
    pub fn periapsis_below_surface(&self, solar_system: &SolarSystem) -> bool {
        if matches!(self.state, ShipState::Landed { .. }) {
            return false;
        }

        let soi_body = &solar_system.bodies[self.soi_body];
        let threshold = soi_body.radius;

        // Use cached orbit if available
        if let Some(ref ship_orbit) = self.cached_orbit {
            let a = ship_orbit.orbit.semi_major_axis;
            let e = ship_orbit.orbit.eccentricity;
            let periapsis = if e < 1.0 {
                a * (1.0 - e)
            } else {
                a.abs() * (e - 1.0)
            };
            return periapsis < threshold;
        }

        // Fallback: compute from state vectors using vis-viva and angular momentum
        let r = (self.rel_position[0].powi(2) + self.rel_position[1].powi(2)).sqrt();
        let v2 = self.rel_velocity[0].powi(2) + self.rel_velocity[1].powi(2);
        let mu = G * soi_body.mass;

        // Semi-major axis from vis-viva: 1/a = 2/r - v²/μ
        let inv_a = 2.0 / r - v2 / mu;
        if inv_a.abs() < 1e-20 {
            return true; // parabolic - will hit surface
        }
        let a = 1.0 / inv_a;

        // Angular momentum magnitude: h = |r × v| (2D cross product)
        let h = (self.rel_position[0] * self.rel_velocity[1] - self.rel_position[1] * self.rel_velocity[0]).abs();

        // Semi-latus rectum: p = h²/μ
        let p = h * h / mu;

        // Eccentricity from p = a(1-e²) for elliptical, p = |a|(e²-1) for hyperbolic
        let e = if a > 0.0 {
            (1.0 - p / a).max(0.0).sqrt()
        } else {
            (1.0 + p / a.abs()).max(0.0).sqrt()
        };

        let periapsis = if a > 0.0 {
            a * (1.0 - e)
        } else {
            a.abs() * (e - 1.0)
        };

        periapsis < threshold
    }

    /// Check if the ship's orbit is suborbital (periapsis below body surface).
    /// Landed state is always considered suborbital.
    pub fn is_suborbital(&self, solar_system: &SolarSystem) -> bool {
        if matches!(self.state, ShipState::Landed { .. }) {
            return true;
        }

        let soi_body = &solar_system.bodies[self.soi_body];

        // Use cached orbit if available
        if let Some(ref ship_orbit) = self.cached_orbit {
            let a = ship_orbit.orbit.semi_major_axis;
            let e = ship_orbit.orbit.eccentricity;
            let periapsis = if e < 1.0 {
                a * (1.0 - e) // elliptical
            } else {
                a.abs() * (e - 1.0) // hyperbolic
            };
            return periapsis < soi_body.radius;
        }

        // Fallback: compute from state vectors using vis-viva and angular momentum
        let r = (self.rel_position[0].powi(2) + self.rel_position[1].powi(2)).sqrt();
        let v2 = self.rel_velocity[0].powi(2) + self.rel_velocity[1].powi(2);
        let mu = G * soi_body.mass;

        // Semi-major axis from vis-viva: 1/a = 2/r - v²/μ
        let inv_a = 2.0 / r - v2 / mu;
        if inv_a.abs() < 1e-20 {
            return true; // parabolic - will hit surface
        }
        let a = 1.0 / inv_a;

        // Angular momentum magnitude: h = |r × v| (2D cross product)
        let h = (self.rel_position[0] * self.rel_velocity[1] - self.rel_position[1] * self.rel_velocity[0]).abs();

        // Semi-latus rectum: p = h²/μ
        let p = h * h / mu;

        // Eccentricity from p = a(1-e²) for elliptical, p = |a|(e²-1) for hyperbolic
        let e = if a > 0.0 {
            (1.0 - p / a).max(0.0).sqrt()
        } else {
            (1.0 + p / a.abs()).max(0.0).sqrt()
        };

        let periapsis = if a > 0.0 {
            a * (1.0 - e)
        } else {
            a.abs() * (e - 1.0)
        };

        periapsis < soi_body.radius
    }

    /// Update ship physics for one frame
    pub fn update(&mut self, dt: f64, time_warp: f64, input: &ShipInput, solar_system: &SolarSystem, vessel: Option<&VesselPhysicsData>, autopilot_active: bool) {
        let in_landing_zone = self.below_landing_altitude(solar_system);
        let actually_thrusting = self.throttle > 0.0
            && vessel.map(|v| v.max_thrust_vac > 0.0).unwrap_or(false);
        let should_be_on_rails = time_warp > RAILS_WARP_THRESHOLD
            && matches!(self.state, ShipState::Flying)
            && !actually_thrusting
            && !in_landing_zone;

        if should_be_on_rails && !self.on_rails {
            self.enter_rails_mode(solar_system);
        } else if !should_be_on_rails && self.on_rails {
            self.exit_rails_mode(solar_system);
        }

        if self.on_rails {
            self.throttle = 0.0;
        } else {
            self.update_throttle(dt, input);
        }

        match &self.state {
            ShipState::Flying => {
                if self.on_rails {
                    self.update_on_rails(dt, solar_system);
                } else {
                    let effective_dt = if self.throttle > 0.0 && time_warp > RAILS_WARP_THRESHOLD {
                        dt / time_warp * RAILS_WARP_THRESHOLD
                    } else {
                        dt
                    };
                    self.update_flying(effective_dt, input, solar_system, vessel, autopilot_active);
                }
            }
            ShipState::Landed { body_index, surface_angle } => {
                let body_index = *body_index;
                let surface_angle = *surface_angle;
                self.update_landed(dt, input, solar_system, body_index, surface_angle, vessel);
            }
        }
    }

    /// Enter on-rails mode - calculate and cache the current orbit
    pub fn enter_rails_mode(&mut self, solar_system: &SolarSystem) {
        if let Some(ship_orbit) = self.calculate_orbit_with_anomaly(solar_system) {
            self.cached_orbit = Some(ship_orbit);
            self.on_rails = true;
        }
    }

    /// Exit on-rails mode - restore position and velocity from orbit
    pub fn exit_rails_mode(&mut self, solar_system: &SolarSystem) {
        if let Some(ref ship_orbit) = self.cached_orbit {
            let parent = &solar_system.bodies[ship_orbit.parent_idx];
            self.rel_position = ship_orbit.orbit.position_from_mean_anomaly(
                ship_orbit.mean_anomaly,
                parent.mass,
            );
            self.rel_velocity = ship_orbit.orbit.velocity_from_mean_anomaly_with_direction(
                ship_orbit.mean_anomaly,
                parent.mass,
                ship_orbit.retrograde,
            );
        }
        self.on_rails = false;
    }

    /// Update throttle based on input
    fn update_throttle(&mut self, dt: f64, input: &ShipInput) {
        if input.throttle_full {
            self.throttle = 1.0;
        } else if input.throttle_zero {
            self.throttle = 0.0;
        } else {
            if input.throttle_up {
                self.throttle += THROTTLE_RATE * dt;
            }
            if input.throttle_down {
                self.throttle -= THROTTLE_RATE * dt;
            }
        }
        self.throttle = self.throttle.clamp(0.0, 1.0);
    }

    /// Update while on rails - follow the orbit exactly
    pub fn update_on_rails(&mut self, dt: f64, solar_system: &SolarSystem) {
        if let Some(ref mut ship_orbit) = self.cached_orbit {
            let parent = &solar_system.bodies[ship_orbit.parent_idx];

            let mean_motion = ship_orbit.orbit.mean_motion(parent.mass);
            let direction = if ship_orbit.retrograde { -1.0 } else { 1.0 };
            ship_orbit.mean_anomaly += direction * mean_motion * dt;

            ship_orbit.mean_anomaly %= std::f64::consts::TAU;
            if ship_orbit.mean_anomaly < 0.0 {
                ship_orbit.mean_anomaly += std::f64::consts::TAU;
            }

            self.rel_position = ship_orbit.orbit.position_from_mean_anomaly(
                ship_orbit.mean_anomaly,
                parent.mass,
            );
            self.rel_velocity = ship_orbit.orbit.velocity_from_mean_anomaly_with_direction(
                ship_orbit.mean_anomaly,
                parent.mass,
                ship_orbit.retrograde,
            );

            self.check_soi_transition_on_rails(solar_system);
        }

        // Radiative cooling while on rails (exponential decay toward ambient)
        if self.temperature > AMBIENT_TEMPERATURE {
            self.temperature += (AMBIENT_TEMPERATURE - self.temperature) * (1.0 - (-0.01 * dt).exp());
            self.temperature = self.temperature.max(AMBIENT_TEMPERATURE);
        }
        self.heat_flux = 0.0;
    }

    /// Update while flying (physics simulation with sub-stepping)
    fn update_flying(&mut self, dt: f64, input: &ShipInput, solar_system: &SolarSystem, vessel: Option<&VesselPhysicsData>, autopilot_active: bool) {
        // Gimbal torque always applies (set by manual input or autopilot)
        let gimbal_accel = vessel
            .map(|v| if v.moment_of_inertia > 0.0 { v.gimbal_torque / v.moment_of_inertia } else { 0.0 })
            .unwrap_or(0.0);
        self.rotational_velocity += gimbal_accel * 0.5 * dt;

        // Manual rotation: reaction wheels + drag (skip when autopilot controls rotation)
        if !autopilot_active {
            let rw_accel = vessel
                .map(|v| if v.moment_of_inertia > 0.0 { v.torque / v.moment_of_inertia } else { ROTATION_ACCEL })
                .unwrap_or(ROTATION_ACCEL);

            if input.rotate_left {
                self.rotational_velocity += rw_accel * dt;
            } else if input.rotate_right {
                self.rotational_velocity -= rw_accel * dt;
            } else {
                // Apply drag when no rotation input
                if self.rotational_velocity.abs() > 0.0 {
                    let drag = ROTATION_DRAG * dt;
                    if self.rotational_velocity > drag {
                        self.rotational_velocity -= drag;
                    } else if self.rotational_velocity < -drag {
                        self.rotational_velocity += drag;
                    } else {
                        self.rotational_velocity = 0.0;
                    }
                }
            }
            self.rotation += self.rotational_velocity * dt;
        }

        // Cap physics substeps to prevent lag at high time warp when not on rails
        const MAX_SUBSTEPS: usize = 1000;
        let num_steps = ((dt / MAX_PHYSICS_DT).ceil() as usize).clamp(1, MAX_SUBSTEPS);
        let sub_dt = dt / num_steps as f64;

        for _ in 0..num_steps {
            self.physics_substep(sub_dt, input, solar_system, vessel);
        }

        // Update temperature from aerodynamic heating (once per frame)
        self.update_temperature(dt, solar_system, vessel);

        self.check_and_handle_collisions(solar_system, vessel);
    }

    /// Update vessel temperature from aerodynamic heating and radiative cooling
    fn update_temperature(&mut self, dt: f64, solar_system: &SolarSystem, vessel: Option<&VesselPhysicsData>) {
        let soi_body = &solar_system.bodies[self.soi_body];
        let atmo = match &soi_body.atmosphere {
            Some(a) => a,
            None => {
                // No atmosphere: cool toward ambient
                self.heat_flux = 0.0;
                if self.temperature > AMBIENT_TEMPERATURE {
                    self.temperature += (AMBIENT_TEMPERATURE - self.temperature) * (1.0 - (-0.01 * dt).exp());
                    self.temperature = self.temperature.max(AMBIENT_TEMPERATURE);
                }
                return;
            }
        };

        let dist = (self.rel_position[0].powi(2) + self.rel_position[1].powi(2)).sqrt();
        let altitude = dist - soi_body.radius;
        if altitude < 0.0 || altitude > atmo.visible_height() {
            self.heat_flux = 0.0;
            if self.temperature > AMBIENT_TEMPERATURE {
                self.temperature += (AMBIENT_TEMPERATURE - self.temperature) * (1.0 - (-0.01 * dt).exp());
                self.temperature = self.temperature.max(AMBIENT_TEMPERATURE);
            }
            return;
        }

        let density = atmo.density_at_altitude(altitude);

        // Surface-relative airspeed
        let surface_vel = soi_body.surface_velocity_at(dist);
        let radial_x = self.rel_position[0] / dist;
        let radial_y = self.rel_position[1] / dist;
        let tangent_x = -radial_y;
        let tangent_y = radial_x;
        let airspeed_x = self.rel_velocity[0] - surface_vel * tangent_x;
        let airspeed_y = self.rel_velocity[1] - surface_vel * tangent_y;
        let airspeed = (airspeed_x.powi(2) + airspeed_y.powi(2)).sqrt();

        let (half_width, half_height) = vessel
            .map(|v| (v.vessel_half_width, v.vessel_height))
            .unwrap_or((SHIP_SIZE / 4.0, SHIP_SIZE / 2.0));

        let velocity_angle = airspeed_y.atan2(airspeed_x);
        let aoa = (self.rotation - velocity_angle).sin().abs();
        let frontal_area = half_width * 2.0 * (1.0 - aoa) + half_height * 2.0 * aoa;

        // Heating: q_in = HEAT_COEFFICIENT * sqrt(density) * airspeed^3 * frontal_area
        let q_in = HEAT_COEFFICIENT * density.sqrt() * airspeed.powi(3) * frontal_area;

        // Radiative cooling: q_out = emissivity * sigma * T^4 * surface_area
        // Surface area approximation: perimeter * 1m depth
        let perimeter = 2.0 * (half_width * 2.0 + half_height * 2.0);
        let q_out = VESSEL_EMISSIVITY * STEFAN_BOLTZMANN * self.temperature.powi(4) * perimeter;

        let total_mass_kg = vessel
            .map(|v| v.total_mass * 1000.0)
            .unwrap_or(1000.0);

        let d_temp = (q_in - q_out) / (total_mass_kg * VESSEL_SPECIFIC_HEAT) * dt;
        self.temperature += d_temp;
        self.temperature = self.temperature.max(AMBIENT_TEMPERATURE);
        self.heat_flux = q_in / frontal_area.max(0.01);
    }

    /// Single physics sub-step using velocity Verlet integration
    fn physics_substep(&mut self, dt: f64, _input: &ShipInput, solar_system: &SolarSystem, vessel: Option<&VesselPhysicsData>) {
        let soi_body = &solar_system.bodies[self.soi_body];

        let calc_gravity_accel = |pos: [f64; 2]| -> [f64; 2] {
            let dist_sq = pos[0] * pos[0] + pos[1] * pos[1];
            let dist = dist_sq.sqrt();
            if dist > soi_body.radius {
                let accel_mag = G * soi_body.mass / dist_sq;
                [-pos[0] / dist * accel_mag, -pos[1] / dist * accel_mag]
            } else {
                [0.0, 0.0]
            }
        };

        // Use vessel-derived thrust acceleration if available, else fallback
        let max_thrust_accel = vessel
            .map(|v| if v.total_mass > 0.0 { v.max_thrust_vac / v.total_mass } else { 0.0 })
            .unwrap_or(MAX_THRUST_ACCELERATION);

        let thrust_accel = if self.throttle > 0.0 {
            let mag = self.throttle * max_thrust_accel;
            [self.rotation.cos() * mag, self.rotation.sin() * mag]
        } else {
            [0.0, 0.0]
        };

        // Compute aerodynamic drag acceleration
        let drag_accel = self.compute_drag_accel(soi_body, vessel);

        let grav_accel_1 = calc_gravity_accel(self.rel_position);
        let accel_1 = [
            grav_accel_1[0] + thrust_accel[0] + drag_accel[0],
            grav_accel_1[1] + thrust_accel[1] + drag_accel[1],
        ];

        let prev_rel_pos = self.rel_position;
        let prev_rel_vel = self.rel_velocity;

        self.rel_position[0] += self.rel_velocity[0] * dt + 0.5 * accel_1[0] * dt * dt;
        self.rel_position[1] += self.rel_velocity[1] * dt + 0.5 * accel_1[1] * dt * dt;

        let grav_accel_2 = calc_gravity_accel(self.rel_position);
        let accel_2 = [
            grav_accel_2[0] + thrust_accel[0] + drag_accel[0],
            grav_accel_2[1] + thrust_accel[1] + drag_accel[1],
        ];

        self.rel_velocity[0] += 0.5 * (accel_1[0] + accel_2[0]) * dt;
        self.rel_velocity[1] += 0.5 * (accel_1[1] + accel_2[1]) * dt;

        self.check_soi_transition_precise(solar_system, prev_rel_pos, prev_rel_vel, dt);
    }

    /// Compute aerodynamic drag acceleration (m/s²) opposing airspeed
    pub fn compute_drag_accel(&self, soi_body: &crate::bodies::CelestialBody, vessel: Option<&VesselPhysicsData>) -> [f64; 2] {
        let atmo = match &soi_body.atmosphere {
            Some(a) => a,
            None => return [0.0, 0.0],
        };

        let dist = (self.rel_position[0].powi(2) + self.rel_position[1].powi(2)).sqrt();
        let altitude = dist - soi_body.radius;
        if altitude < 0.0 || altitude > atmo.visible_height() {
            return [0.0, 0.0];
        }

        let density = atmo.density_at_altitude(altitude);
        if density < 1e-12 {
            return [0.0, 0.0];
        }

        // Surface-relative airspeed: subtract body rotation velocity
        let surface_vel = soi_body.surface_velocity_at(dist);
        // Rotation velocity is perpendicular to radial direction (tangential, CCW)
        let radial_x = self.rel_position[0] / dist;
        let radial_y = self.rel_position[1] / dist;
        let tangent_x = -radial_y;
        let tangent_y = radial_x;

        let airspeed_x = self.rel_velocity[0] - surface_vel * tangent_x;
        let airspeed_y = self.rel_velocity[1] - surface_vel * tangent_y;
        let airspeed = (airspeed_x.powi(2) + airspeed_y.powi(2)).sqrt();
        if airspeed < 0.01 {
            return [0.0, 0.0];
        }

        // Orientation-dependent cross-section
        let (half_width, half_height) = vessel
            .map(|v| (v.vessel_half_width, v.vessel_height))
            .unwrap_or((SHIP_SIZE / 4.0, SHIP_SIZE / 2.0));

        let velocity_angle = airspeed_y.atan2(airspeed_x);
        let aoa = (self.rotation - velocity_angle).sin().abs(); // 0 = nose-on, 1 = broadside
        let cross_section = half_width * 2.0 * (1.0 - aoa) + half_height * 2.0 * aoa;

        // F_drag = 0.5 * rho * v^2 * Cd * A
        let total_mass_kg = vessel
            .map(|v| v.total_mass * 1000.0) // tonnes -> kg
            .unwrap_or(1000.0);
        let drag_force = 0.5 * density * airspeed * airspeed * DRAG_COEFFICIENT * cross_section;
        let drag_accel_mag = drag_force / total_mass_kg;

        // Apply opposite to airspeed direction
        [
            -airspeed_x / airspeed * drag_accel_mag,
            -airspeed_y / airspeed * drag_accel_mag,
        ]
    }

    /// Update while landed on a surface
    fn update_landed(
        &mut self,
        dt: f64,
        _input: &ShipInput,
        solar_system: &SolarSystem,
        body_index: usize,
        surface_angle: f64,
        vessel: Option<&VesselPhysicsData>,
    ) {
        use crate::game::{LAUNCHPAD_BODY_INDEX, LAUNCHPAD_SURFACE_ANGLE, LAUNCHPAD_HEIGHT, LAUNCHPAD_BOTTOM_WIDTH};

        let body = &solar_system.bodies[body_index];
        let body_radius = body.radius;
        let surface_gravity = G * body.mass / (body_radius * body_radius);

        let max_thrust_accel = vessel
            .map(|v| if v.total_mass > 0.0 { v.max_thrust_vac / v.total_mass } else { 0.0 })
            .unwrap_or(MAX_THRUST_ACCELERATION);
        let thrust_accel = self.throttle * max_thrust_accel;

        let bottom = vessel
            .map(|v| v.bottom_extent)
            .unwrap_or(SHIP_SIZE / 2.0);

        // Account for launchpad height
        let launchpad_offset = if body_index == LAUNCHPAD_BODY_INDEX {
            let angle_diff = surface_angle - LAUNCHPAD_SURFACE_ANGLE;
            let angle_diff = angle_diff - (angle_diff / std::f64::consts::TAU).round() * std::f64::consts::TAU;
            let half_angle = (LAUNCHPAD_BOTTOM_WIDTH * 0.5) / body_radius;
            if angle_diff.abs() < half_angle {
                LAUNCHPAD_HEIGHT
            } else {
                0.0
            }
        } else {
            0.0
        };

        self.rotation = surface_angle;
        self.rotational_velocity = 0.0;
        let surface_distance = body_radius + launchpad_offset + bottom;

        if thrust_accel > surface_gravity {
            let net_accel = thrust_accel - surface_gravity;
            let up_dir = [surface_angle.cos(), surface_angle.sin()];

            self.rel_velocity = [
                up_dir[0] * net_accel * dt,
                up_dir[1] * net_accel * dt,
            ];

            // Move position above the surface so the terrain collision check
            // doesn't immediately catch us and zero throttle
            let displacement = 0.5 * net_accel * dt * dt;
            self.rel_position = [
                (surface_distance + displacement) * up_dir[0],
                (surface_distance + displacement) * up_dir[1],
            ];

            self.state = ShipState::Flying;
            self.cached_orbit = None;
        } else {
            self.rel_position = [
                surface_distance * surface_angle.cos(),
                surface_distance * surface_angle.sin(),
            ];
            self.rel_velocity = [0.0, 0.0];
        }
    }

    /// Check for collisions with bodies
    fn check_and_handle_collisions(&mut self, solar_system: &SolarSystem, vessel: Option<&VesselPhysicsData>) {
        use crate::game::{LAUNCHPAD_BODY_INDEX, LAUNCHPAD_SURFACE_ANGLE,
                          LAUNCHPAD_HEIGHT, LAUNCHPAD_BOTTOM_WIDTH};

        let ship_radius = vessel
            .map(|v| v.bottom_extent)
            .unwrap_or(SHIP_SIZE / 2.0);
        let abs_pos = self.absolute_position(solar_system);

        for (i, body) in solar_system.bodies.iter().enumerate() {
            let body_pos = solar_system.body_position(i);
            let dx = abs_pos[0] - body_pos[0];
            let dy = abs_pos[1] - body_pos[1];
            let dist = (dx * dx + dy * dy).sqrt();

            // Check launchpad collision (raised surface)
            if i == LAUNCHPAD_BODY_INDEX {
                let lp_collision_dist = body.radius + LAUNCHPAD_HEIGHT + ship_radius;
                if dist < lp_collision_dist {
                    let surface_angle = dy.atan2(dx);
                    let angle_diff = surface_angle - LAUNCHPAD_SURFACE_ANGLE;
                    let angle_diff = angle_diff - (angle_diff / std::f64::consts::TAU).round() * std::f64::consts::TAU;
                    let half_angle = (LAUNCHPAD_BOTTOM_WIDTH * 0.5) / body.radius;
                    if angle_diff.abs() < half_angle {
                        if i != self.soi_body {
                            self.soi_body = i;
                        }
                        let surface_distance = body.radius + LAUNCHPAD_HEIGHT + ship_radius;
                        self.rel_position = [
                            surface_distance * surface_angle.cos(),
                            surface_distance * surface_angle.sin(),
                        ];
                        self.rel_velocity = [0.0, 0.0];
                        self.throttle = 0.0;
                        self.rotation = surface_angle;
                        self.state = ShipState::Landed {
                            body_index: i,
                            surface_angle,
                        };
                        self.on_rails = false;
                        self.cached_orbit = None;
                        return;
                    }
                }
            }

            let collision_dist = body.radius + ship_radius;

            if dist < collision_dist {
                let surface_angle = dy.atan2(dx);

                if i != self.soi_body {
                    self.soi_body = i;
                }

                let surface_distance = body.radius + ship_radius;
                self.rel_position = [
                    surface_distance * surface_angle.cos(),
                    surface_distance * surface_angle.sin(),
                ];

                self.rel_velocity = [0.0, 0.0];
                self.throttle = 0.0;
                self.rotation = surface_angle;

                self.state = ShipState::Landed {
                    body_index: i,
                    surface_angle,
                };
                self.on_rails = false;
                self.cached_orbit = None;

                return;
            }
        }
    }

    /// Get the current orbit for rendering (uses cached orbit if on rails)
    pub fn get_render_orbit(&self) -> Option<(Orbit, usize)> {
        if matches!(self.state, ShipState::Landed { .. }) {
            return None;
        }
        self.cached_orbit.as_ref().map(|o| (o.orbit, o.parent_idx))
    }

    /// Calculate orbital elements (for when not on rails)
    pub fn calculate_orbit(&mut self, solar_system: &SolarSystem) -> Option<(Orbit, usize)> {
        if self.on_rails {
            return self.get_render_orbit();
        }

        if let Some(ship_orbit) = self.calculate_orbit_with_anomaly(solar_system) {
            let result = (ship_orbit.orbit, ship_orbit.parent_idx);
            self.cached_orbit = Some(ship_orbit);
            return Some(result);
        }

        None
    }

    /// Calculate a predicted trajectory from a given state (for maneuver node predictions)
    /// Returns segments similar to patched conics trajectory
    pub fn calculate_predicted_trajectory(
        &self,
        pos: [f64; 2],
        vel: [f64; 2],
        parent_idx: usize,
        solar_system: &SolarSystem,
    ) -> Option<PatchedTrajectory> {
        let parent = &solar_system.bodies[parent_idx];

        let (orbit, true_anomaly, retrograde) = self.calculate_orbit_from_state(pos, vel, parent.mass)?;

        let mut segments = Vec::new();

        // Handle hyperbolic orbit
        if orbit.eccentricity >= 1.0 {
            let e = orbit.eccentricity;
            let a_abs = orbit.semi_major_axis.abs();
            let p = a_abs * (e * e - 1.0);

            let soi_radius = parent.soi_radius;
            let cos_nu_exit = (p / soi_radius - 1.0) / e;

            let exit_true_anomaly = if cos_nu_exit.abs() <= 1.0 {
                let ta = cos_nu_exit.acos();
                if retrograde { -ta } else { ta }
            } else {
                let ta = (-1.0 / e).acos() - HYPERBOLIC_ANGLE_MARGIN;
                if retrograde { -ta } else { ta }
            };

            segments.push(PatchedConicSegment {
                orbit,
                parent_idx,
                retrograde,
                start_true_anomaly: true_anomaly,
                end_true_anomaly: Some(exit_true_anomaly),
                start_time: 0.0,
                end_time: None,
            });

            // Continue to parent body after SOI exit
            if let Some(grandparent_idx) = parent.parent {
                let exit_mean_anomaly = self.true_to_mean_anomaly(&orbit, exit_true_anomaly);

                let exit_pos = orbit.position_from_mean_anomaly(exit_mean_anomaly, parent.mass);
                let exit_vel = orbit.velocity_from_mean_anomaly_with_direction(
                    exit_mean_anomaly,
                    parent.mass,
                    retrograde,
                );

                let (new_pos, new_vel, _) = self.convert_to_parent_frame(
                    exit_pos, exit_vel,
                    parent_idx,
                    grandparent_idx,
                    solar_system.time,
                    solar_system,
                );

                let grandparent = &solar_system.bodies[grandparent_idx];
                if let Some((new_orbit, new_ta, new_retro)) = self.calculate_orbit_from_state(
                    new_pos, new_vel, grandparent.mass,
                ) {
                    // Check if this orbit also escapes or enters another SOI
                    let new_e = new_orbit.eccentricity;
                    let end_ta = if new_e >= 1.0 {
                        let a_abs = new_orbit.semi_major_axis.abs();
                        let p = a_abs * (new_e * new_e - 1.0);
                        let soi = grandparent.soi_radius;
                        let cos_exit = (p / soi - 1.0) / new_e;
                        if cos_exit.abs() <= 1.0 {
                            let ta = cos_exit.acos();
                            Some(if new_retro { -ta } else { ta })
                        } else {
                            None
                        }
                    } else {
                        // Check for SOI exit for elliptical orbit
                        let apoapsis = new_orbit.semi_major_axis * (1.0 + new_e);
                        if grandparent.parent.is_some() && apoapsis > grandparent.soi_radius {
                            // Will exit SOI - calculate where
                            let p = new_orbit.semi_major_axis * (1.0 - new_e * new_e);
                            let cos_exit = (p / grandparent.soi_radius - 1.0) / new_e;
                            if cos_exit.abs() <= 1.0 {
                                let ta = cos_exit.acos();
                                Some(if new_retro { -ta } else { ta })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    segments.push(PatchedConicSegment {
                        orbit: new_orbit,
                        parent_idx: grandparent_idx,
                        retrograde: new_retro,
                        start_true_anomaly: new_ta,
                        end_true_anomaly: end_ta,
                        start_time: 0.0,
                        end_time: None,
                    });
                }
            }

            return Some(PatchedTrajectory { segments });
        }

        // Elliptical orbit - check for SOI transitions
        let mean_anomaly = self.true_to_mean_anomaly(&orbit, true_anomaly);

        let intersection = self.find_soi_intersection(
            &orbit,
            parent_idx,
            mean_anomaly,
            retrograde,
            solar_system,
            solar_system.time,
        );

        match intersection {
            Some((intersect_true_anomaly, intersect_time, new_parent_idx, entry)) => {
                segments.push(PatchedConicSegment {
                    orbit,
                    parent_idx,
                    retrograde,
                    start_true_anomaly: true_anomaly,
                    end_true_anomaly: Some(intersect_true_anomaly),
                    start_time: 0.0,
                    end_time: Some(intersect_time),
                });

                let intersect_mean_anomaly = self.true_to_mean_anomaly(&orbit, intersect_true_anomaly);
                let int_pos = orbit.position_from_mean_anomaly(intersect_mean_anomaly, parent.mass);
                let int_vel = orbit.velocity_from_mean_anomaly_with_direction(
                    intersect_mean_anomaly,
                    parent.mass,
                    retrograde,
                );

                let absolute_intersect_time = solar_system.time + intersect_time;
                let (new_pos, new_vel, _) = if entry {
                    self.convert_to_child_frame(
                        int_pos, int_vel,
                        parent_idx,
                        new_parent_idx,
                        absolute_intersect_time,
                        solar_system,
                    )
                } else {
                    self.convert_to_parent_frame(
                        int_pos, int_vel,
                        parent_idx,
                        new_parent_idx,
                        absolute_intersect_time,
                        solar_system,
                    )
                };

                let new_parent = &solar_system.bodies[new_parent_idx];
                if let Some((new_orbit, new_ta, new_retro)) = self.calculate_orbit_from_state(
                    new_pos, new_vel, new_parent.mass,
                ) {
                    let end_ta = if new_orbit.eccentricity >= 1.0 {
                        let e = new_orbit.eccentricity;
                        let a_abs = new_orbit.semi_major_axis.abs();
                        let p = a_abs * (e * e - 1.0);
                        let soi = new_parent.soi_radius;
                        let cos_exit = (p / soi - 1.0) / e;
                        if cos_exit.abs() <= 1.0 {
                            Some(if new_retro { -cos_exit.acos() } else { cos_exit.acos() })
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    segments.push(PatchedConicSegment {
                        orbit: new_orbit,
                        parent_idx: new_parent_idx,
                        retrograde: new_retro,
                        start_true_anomaly: new_ta,
                        end_true_anomaly: end_ta,
                        start_time: intersect_time,
                        end_time: None,
                    });
                }
            }
            None => {
                segments.push(PatchedConicSegment {
                    orbit,
                    parent_idx,
                    retrograde,
                    start_true_anomaly: true_anomaly,
                    end_true_anomaly: None,
                    start_time: 0.0,
                    end_time: None,
                });
            }
        }

        if segments.is_empty() {
            None
        } else {
            Some(PatchedTrajectory { segments })
        }
    }

    /// Calculate target angle for autopilot based on current mode
    /// Returns the target rotation angle, or None if no valid target
    pub fn autopilot_target_angle(
        &self,
        target: AutopilotTarget,
        maneuver_node: Option<&ManeuverNode>,
    ) -> Option<f64> {
        match target {
            AutopilotTarget::Off => None,
            AutopilotTarget::Prograde | AutopilotTarget::Retrograde |
            AutopilotTarget::RadialIn | AutopilotTarget::RadialOut => {
                let vel = self.rel_velocity;
                let vel_mag = (vel[0] * vel[0] + vel[1] * vel[1]).sqrt();
                if vel_mag > 0.1 {
                    let prograde_angle = vel[1].atan2(vel[0]);
                    match target {
                        AutopilotTarget::Prograde => Some(prograde_angle),
                        AutopilotTarget::Retrograde => Some(prograde_angle + std::f64::consts::PI),
                        AutopilotTarget::RadialOut => Some(prograde_angle + std::f64::consts::FRAC_PI_2),
                        AutopilotTarget::RadialIn => Some(prograde_angle - std::f64::consts::FRAC_PI_2),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            AutopilotTarget::ManeuverNode => {
                if let Some(node) = maneuver_node {
                    let prograde = node.prograde_unit();
                    let radial = node.radial_unit();
                    let dv_x = prograde[0] * node.delta_v.prograde + radial[0] * node.delta_v.radial_out;
                    let dv_y = prograde[1] * node.delta_v.prograde + radial[1] * node.delta_v.radial_out;
                    let dv_mag = (dv_x * dv_x + dv_y * dv_y).sqrt();
                    if dv_mag > 0.001 {
                        Some(dv_y.atan2(dv_x))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    /// Compute the direction the autopilot wants to rotate: +1 (CCW), -1 (CW), or 0 (hold).
    /// Used to command engine gimbals before physics update.
    pub fn autopilot_desired_direction(&self, target_angle: f64, vessel: Option<&VesselPhysicsData>) -> f64 {
        let mut angle_diff = target_angle - self.rotation;
        while angle_diff > std::f64::consts::PI {
            angle_diff -= std::f64::consts::TAU;
        }
        while angle_diff < -std::f64::consts::PI {
            angle_diff += std::f64::consts::TAU;
        }

        let vel = self.rotational_velocity;
        let accel = vessel
            .map(|v| if v.moment_of_inertia > 0.0 { v.torque / v.moment_of_inertia } else { ROTATION_ACCEL })
            .unwrap_or(ROTATION_ACCEL);
        let threshold = 0.002;

        if angle_diff.abs() < threshold && vel.abs() < 0.01 {
            return 0.0;
        }

        let stopping_dist = vel.powi(2) / (2.0 * accel);
        let going_right_way = (angle_diff > 0.0 && vel >= 0.0) || (angle_diff < 0.0 && vel <= 0.0);
        let should_brake = going_right_way && stopping_dist >= angle_diff.abs() * 0.5;

        if should_brake {
            if vel > 0.0 { -1.0 } else { 1.0 }
        } else {
            if angle_diff > 0.0 { 1.0 } else { -1.0 }
        }
    }

    /// Update ship rotation toward target angle using autopilot
    /// Uses acceleration-based control with stopping distance braking.
    /// Applies only reaction wheel torque here; gimbal torque is applied
    /// separately in update_flying (always active).
    pub fn autopilot_rotate(&mut self, target_angle: f64, dt: f64, vessel: Option<&VesselPhysicsData>) {
        // Normalize angle difference to [-PI, PI]
        let mut angle_diff = target_angle - self.rotation;
        while angle_diff > std::f64::consts::PI {
            angle_diff -= std::f64::consts::TAU;
        }
        while angle_diff < -std::f64::consts::PI {
            angle_diff += std::f64::consts::TAU;
        }

        let vel = self.rotational_velocity;
        // Use reaction wheel torque for direct control
        let rw_accel = vessel
            .map(|v| if v.moment_of_inertia > 0.0 { v.torque / v.moment_of_inertia } else { ROTATION_ACCEL })
            .unwrap_or(ROTATION_ACCEL);
        // Include gimbal torque in total available acceleration for braking calculations
        let gimbal_accel = vessel
            .map(|v| if v.moment_of_inertia > 0.0 { v.gimbal_torque.abs() / v.moment_of_inertia } else { 0.0 })
            .unwrap_or(0.0);
        let total_accel = rw_accel + gimbal_accel;
        let threshold = 0.002; // ~0.1 degrees

        if angle_diff.abs() < threshold && vel.abs() < 0.01 {
            // Close enough and nearly stopped - snap to target
            self.rotational_velocity = 0.0;
            self.rotation = target_angle;
        } else {
            // Calculate stopping distance at current velocity: s = v²/(2a)
            let stopping_dist = vel.powi(2) / (2.0 * total_accel);

            // Are we going the right direction?
            let going_right_way = (angle_diff > 0.0 && vel >= 0.0) || (angle_diff < 0.0 && vel <= 0.0);

            // Should we brake? Start braking when stopping distance reaches 50% of remaining
            let should_brake = going_right_way && stopping_dist >= angle_diff.abs() * 0.5;

            if should_brake {
                // Brake: accelerate opposite to velocity (RW only; gimbal assists via update_flying)
                if vel > 0.0 {
                    self.rotational_velocity -= rw_accel * dt;
                } else {
                    self.rotational_velocity += rw_accel * dt;
                }
            } else {
                // Accelerate toward target (RW only; gimbal assists via update_flying)
                if angle_diff > 0.0 {
                    self.rotational_velocity += rw_accel * dt;
                } else {
                    self.rotational_velocity -= rw_accel * dt;
                }
            }

            // Apply rotational velocity
            self.rotation += self.rotational_velocity * dt;
        }
    }
}
