use crate::bodies::{SolarSystem, G, Orbit};

mod orbit;
mod patched_conics;
mod soi;

/// Ship size in meters (physics space)
pub const SHIP_SIZE: f64 = 10.0;

/// Maximum thrust acceleration in m/s²
pub const MAX_THRUST_ACCELERATION: f64 = 20.0;

/// Rotation speed in radians/second
pub const ROTATION_SPEED: f64 = 2.0;

/// Throttle change rate per second (0-1 scale)
pub const THROTTLE_RATE: f64 = 0.5;

/// Time warp threshold for on-rails mode
pub const RAILS_WARP_THRESHOLD: f64 = 10.0;

/// Maximum physics timestep for accurate integration (seconds)
const MAX_PHYSICS_DT: f64 = 0.01;

/// Maximum number of SOI changes to predict in patched conics
pub const MAX_PATCHED_CONICS: usize = 3;

/// Number of samples for SOI intersection detection
pub(crate) const SOI_INTERSECTION_SAMPLES: usize = 2000;

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
    pub throttle: f64,
    pub state: ShipState,
    pub color: [f32; 4],
    pub soi_body: usize,
    pub on_rails: bool,
    pub(crate) cached_orbit: Option<ShipOrbit>,
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
            throttle: 0.0,
            state: ShipState::Flying,
            color: [1.0, 0.2, 0.2, 1.0],
            soi_body: earth_index,
            on_rails: false,
            cached_orbit: None,
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

    /// Update ship physics for one frame
    pub fn update(&mut self, dt: f64, time_warp: f64, input: &ShipInput, solar_system: &SolarSystem) {
        let should_be_on_rails = time_warp > RAILS_WARP_THRESHOLD
            && matches!(self.state, ShipState::Flying)
            && self.throttle == 0.0;

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
                    self.update_flying(effective_dt, input, solar_system);
                }
            }
            ShipState::Landed { body_index, surface_angle } => {
                let body_index = *body_index;
                let surface_angle = *surface_angle;
                self.update_landed(dt, input, solar_system, body_index, surface_angle);
            }
        }
    }

    /// Enter on-rails mode - calculate and cache the current orbit
    fn enter_rails_mode(&mut self, solar_system: &SolarSystem) {
        if let Some(ship_orbit) = self.calculate_orbit_with_anomaly(solar_system) {
            self.cached_orbit = Some(ship_orbit);
            self.on_rails = true;
        }
    }

    /// Exit on-rails mode - restore position and velocity from orbit
    fn exit_rails_mode(&mut self, solar_system: &SolarSystem) {
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
    fn update_on_rails(&mut self, dt: f64, solar_system: &SolarSystem) {
        if let Some(ref mut ship_orbit) = self.cached_orbit {
            let parent = &solar_system.bodies[ship_orbit.parent_idx];

            let mean_motion = ship_orbit.orbit.mean_motion(parent.mass);
            let direction = if ship_orbit.retrograde { -1.0 } else { 1.0 };
            ship_orbit.mean_anomaly += direction * mean_motion * dt;

            ship_orbit.mean_anomaly = ship_orbit.mean_anomaly % std::f64::consts::TAU;
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
    }

    /// Update while flying (physics simulation with sub-stepping)
    fn update_flying(&mut self, dt: f64, input: &ShipInput, solar_system: &SolarSystem) {
        if input.rotate_left {
            self.rotation += ROTATION_SPEED * dt;
        }
        if input.rotate_right {
            self.rotation -= ROTATION_SPEED * dt;
        }

        let num_steps = ((dt / MAX_PHYSICS_DT).ceil() as usize).max(1);
        let sub_dt = dt / num_steps as f64;

        for _ in 0..num_steps {
            self.physics_substep(sub_dt, input, solar_system);
        }

        self.check_and_handle_collisions(solar_system);
    }

    /// Single physics sub-step using velocity Verlet integration
    fn physics_substep(&mut self, dt: f64, _input: &ShipInput, solar_system: &SolarSystem) {
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

        let thrust_accel = if self.throttle > 0.0 {
            let mag = self.throttle * MAX_THRUST_ACCELERATION;
            [self.rotation.cos() * mag, self.rotation.sin() * mag]
        } else {
            [0.0, 0.0]
        };

        let grav_accel_1 = calc_gravity_accel(self.rel_position);
        let accel_1 = [
            grav_accel_1[0] + thrust_accel[0],
            grav_accel_1[1] + thrust_accel[1],
        ];

        let prev_rel_pos = self.rel_position;
        let prev_rel_vel = self.rel_velocity;

        self.rel_position[0] += self.rel_velocity[0] * dt + 0.5 * accel_1[0] * dt * dt;
        self.rel_position[1] += self.rel_velocity[1] * dt + 0.5 * accel_1[1] * dt * dt;

        let grav_accel_2 = calc_gravity_accel(self.rel_position);
        let accel_2 = [
            grav_accel_2[0] + thrust_accel[0],
            grav_accel_2[1] + thrust_accel[1],
        ];

        self.rel_velocity[0] += 0.5 * (accel_1[0] + accel_2[0]) * dt;
        self.rel_velocity[1] += 0.5 * (accel_1[1] + accel_2[1]) * dt;

        self.check_soi_transition_precise(solar_system, prev_rel_pos, prev_rel_vel, dt);
    }

    /// Update while landed on a surface
    fn update_landed(
        &mut self,
        dt: f64,
        input: &ShipInput,
        solar_system: &SolarSystem,
        body_index: usize,
        surface_angle: f64,
    ) {
        let body = &solar_system.bodies[body_index];
        let body_radius = body.radius;
        let surface_gravity = G * body.mass / (body_radius * body_radius);
        let thrust_accel = self.throttle * MAX_THRUST_ACCELERATION;

        self.rotation = surface_angle;
        let surface_distance = body_radius + SHIP_SIZE / 2.0;

        if thrust_accel > surface_gravity {
            let net_accel = thrust_accel - surface_gravity;
            let up_dir = [surface_angle.cos(), surface_angle.sin()];

            self.rel_velocity = [
                up_dir[0] * net_accel * dt,
                up_dir[1] * net_accel * dt,
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

        if input.rotate_left {
            self.rotation += ROTATION_SPEED * dt;
        }
        if input.rotate_right {
            self.rotation -= ROTATION_SPEED * dt;
        }
    }

    /// Check for collisions with bodies
    fn check_and_handle_collisions(&mut self, solar_system: &SolarSystem) {
        let ship_radius = SHIP_SIZE / 2.0;
        let abs_pos = self.absolute_position(solar_system);

        for (i, body) in solar_system.bodies.iter().enumerate() {
            let body_pos = solar_system.body_position(i);
            let dx = abs_pos[0] - body_pos[0];
            let dy = abs_pos[1] - body_pos[1];
            let dist = (dx * dx + dy * dy).sqrt();

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
}
