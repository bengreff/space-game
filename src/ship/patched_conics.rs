use crate::bodies::{SolarSystem, G, Orbit};
use super::{
    Ship, ShipState, PatchedConicSegment, PatchedTrajectory, MAX_PATCHED_CONICS,
    SOI_INTERSECTION_SAMPLES, SOI_EXIT_THRESHOLD, SOI_ENTRY_THRESHOLD,
    MIN_INTERSECTION_TIME, SOI_REFINE_ITERATIONS, HYPERBOLIC_ANGLE_MARGIN,
};

/// How often to recalculate trajectory (in frames, ~0.5 seconds at 60fps)
const TRAJECTORY_CACHE_FRAMES: u64 = 30;

impl Ship {
    /// Get patched conics trajectory, using cache when possible
    pub fn get_patched_trajectory(&mut self, solar_system: &SolarSystem) -> Option<PatchedTrajectory> {
        // Increment frame counter
        self.frame_counter += 1;

        // Invalidate cache if:
        // 1. Ship is thrusting (orbit changing)
        // 2. SOI body changed
        // 3. Enough frames have passed
        let frames_since_calc = self.frame_counter.saturating_sub(self.trajectory_calc_frame);
        let cache_valid = self.throttle == 0.0
            && self.soi_body == self.trajectory_soi_body
            && frames_since_calc < TRAJECTORY_CACHE_FRAMES
            && self.cached_trajectory.is_some();

        if cache_valid {
            return self.cached_trajectory.clone();
        }

        // Recalculate and cache
        let trajectory = self.calculate_patched_trajectory_internal(solar_system);
        self.cached_trajectory = trajectory.clone();
        self.trajectory_calc_frame = self.frame_counter;
        self.trajectory_soi_body = self.soi_body;
        trajectory
    }

    /// Calculate patched conics trajectory predicting future SOI changes (internal, uncached)
    fn calculate_patched_trajectory_internal(&self, solar_system: &SolarSystem) -> Option<PatchedTrajectory> {
        if matches!(self.state, ShipState::Landed { .. }) {
            return None;
        }

        let parent_idx = self.soi_body;
        let parent = &solar_system.bodies[parent_idx];

        let (orbit, true_anomaly, retrograde) = self.calculate_orbit_from_state(
            self.rel_position,
            self.rel_velocity,
            parent.mass,
        )?;

        let segments = self.compute_patched_segments(
            orbit, true_anomaly, retrograde, parent_idx,
            solar_system.time, solar_system,
        );

        if segments.is_empty() { None } else { Some(PatchedTrajectory { segments }) }
    }

    /// Compute patched conic segments from given orbital state.
    /// Handles both elliptical and hyperbolic orbits, continuing across SOI boundaries
    /// up to MAX_PATCHED_CONICS transitions.
    pub(crate) fn compute_patched_segments(
        &self,
        initial_orbit: Orbit,
        initial_ta: f64,
        initial_retrograde: bool,
        initial_parent_idx: usize,
        base_time: f64,
        solar_system: &SolarSystem,
    ) -> Vec<PatchedConicSegment> {
        let mut segments = Vec::new();
        let mut current_orbit = initial_orbit;
        let mut current_parent_idx = initial_parent_idx;
        let mut current_ta = initial_ta;
        let mut current_retrograde = initial_retrograde;
        let mut cumulative_time = 0.0;
        let mut soi_crossings = 0;

        loop {
            let parent = &solar_system.bodies[current_parent_idx];

            if current_orbit.eccentricity >= 1.0 {
                // Hyperbolic orbit - calculate SOI exit analytically
                let e = current_orbit.eccentricity;
                let a_abs = current_orbit.semi_major_axis.abs();
                let p = a_abs * (e * e - 1.0);
                let soi_radius = parent.soi_radius;
                let cos_nu_exit = (p / soi_radius - 1.0) / e;

                let exit_true_anomaly = if cos_nu_exit.abs() <= 1.0 {
                    let ta = cos_nu_exit.acos();
                    if current_retrograde { -ta } else { ta }
                } else {
                    let ta = (-1.0 / e).acos() - HYPERBOLIC_ANGLE_MARGIN;
                    if current_retrograde { -ta } else { ta }
                };

                // Calculate time to exit for body position computation
                let exit_ma = self.true_to_mean_anomaly(&current_orbit, exit_true_anomaly);
                let start_ma = self.true_to_mean_anomaly(&current_orbit, current_ta);
                let mu = G * parent.mass;
                let n = (mu / a_abs.powi(3)).sqrt();
                let time_to_exit = if n > 0.0 {
                    (exit_ma - start_ma).abs() / n
                } else {
                    0.0
                };

                segments.push(PatchedConicSegment {
                    orbit: current_orbit,
                    parent_idx: current_parent_idx,
                    retrograde: current_retrograde,
                    start_true_anomaly: current_ta,
                    end_true_anomaly: Some(exit_true_anomaly),
                    start_time: cumulative_time,
                    end_time: Some(cumulative_time + time_to_exit),
                });

                // Try to continue to parent body after SOI exit
                if soi_crossings >= MAX_PATCHED_CONICS {
                    break;
                }

                if let Some(grandparent_idx) = parent.parent {
                    let exit_pos = current_orbit.position_from_mean_anomaly(exit_ma, parent.mass);
                    let exit_vel = current_orbit.velocity_from_mean_anomaly_with_direction(
                        exit_ma, parent.mass, current_retrograde,
                    );
                    let exit_absolute_time = base_time + cumulative_time + time_to_exit;

                    let (new_pos, new_vel, _) = self.convert_to_parent_frame(
                        exit_pos, exit_vel,
                        current_parent_idx,
                        grandparent_idx,
                        exit_absolute_time,
                        solar_system,
                    );

                    let grandparent = &solar_system.bodies[grandparent_idx];
                    if let Some((new_orbit, new_ta, new_retro)) = self.calculate_orbit_from_state(
                        new_pos, new_vel, grandparent.mass,
                    ) {
                        cumulative_time += time_to_exit;
                        current_orbit = new_orbit;
                        current_parent_idx = grandparent_idx;
                        current_ta = new_ta;
                        current_retrograde = new_retro;
                        soi_crossings += 1;
                        continue;
                    }
                }
                break;
            }

            // Elliptical orbit - if max crossings reached, push full orbit as final segment
            if soi_crossings >= MAX_PATCHED_CONICS {
                segments.push(PatchedConicSegment {
                    orbit: current_orbit,
                    parent_idx: current_parent_idx,
                    retrograde: current_retrograde,
                    start_true_anomaly: current_ta,
                    end_true_anomaly: None,
                    start_time: cumulative_time,
                    end_time: None,
                });
                break;
            }

            let current_ma = self.true_to_mean_anomaly(&current_orbit, current_ta);

            let intersection = self.find_soi_intersection(
                &current_orbit,
                current_parent_idx,
                current_ma,
                current_retrograde,
                solar_system,
                base_time + cumulative_time,
            );

            match intersection {
                Some((intersect_ta, intersect_time, new_parent_idx, entry)) => {
                    segments.push(PatchedConicSegment {
                        orbit: current_orbit,
                        parent_idx: current_parent_idx,
                        retrograde: current_retrograde,
                        start_true_anomaly: current_ta,
                        end_true_anomaly: Some(intersect_ta),
                        start_time: cumulative_time,
                        end_time: Some(cumulative_time + intersect_time),
                    });

                    let intersect_mean_anomaly = self.true_to_mean_anomaly(&current_orbit, intersect_ta);
                    let pos = current_orbit.position_from_mean_anomaly(intersect_mean_anomaly, parent.mass);
                    let vel = current_orbit.velocity_from_mean_anomaly_with_direction(
                        intersect_mean_anomaly,
                        parent.mass,
                        current_retrograde,
                    );

                    let absolute_intersect_time = base_time + cumulative_time + intersect_time;
                    let (new_pos, new_vel, _) = if entry {
                        self.convert_to_child_frame(
                            pos, vel,
                            current_parent_idx,
                            new_parent_idx,
                            absolute_intersect_time,
                            solar_system,
                        )
                    } else {
                        self.convert_to_parent_frame(
                            pos, vel,
                            current_parent_idx,
                            new_parent_idx,
                            absolute_intersect_time,
                            solar_system,
                        )
                    };

                    let new_parent = &solar_system.bodies[new_parent_idx];
                    if let Some((new_orbit, new_ta, new_retro)) = self.calculate_orbit_from_state(
                        new_pos, new_vel, new_parent.mass,
                    ) {
                        cumulative_time += intersect_time;
                        current_orbit = new_orbit;
                        current_parent_idx = new_parent_idx;
                        current_ta = new_ta;
                        current_retrograde = new_retro;
                        soi_crossings += 1;
                        continue;
                    }
                    break;
                }
                None => {
                    segments.push(PatchedConicSegment {
                        orbit: current_orbit,
                        parent_idx: current_parent_idx,
                        retrograde: current_retrograde,
                        start_true_anomaly: current_ta,
                        end_true_anomaly: None,
                        start_time: cumulative_time,
                        end_time: None,
                    });
                    break;
                }
            }
        }

        segments
    }

    /// Find the next SOI intersection along the orbit
    /// Returns: (true_anomaly, time_to_reach, new_parent_idx, is_entry)
    pub(crate) fn find_soi_intersection(
        &self,
        orbit: &Orbit,
        parent_idx: usize,
        start_mean_anomaly: f64,
        retrograde: bool,
        solar_system: &SolarSystem,
        base_time: f64,
    ) -> Option<(f64, f64, usize, bool)> {
        let parent = &solar_system.bodies[parent_idx];
        let mu = G * parent.mass;
        let period = std::f64::consts::TAU * (orbit.semi_major_axis.powi(3) / mu).sqrt();
        let mean_motion = std::f64::consts::TAU / period;

        let apoapsis = orbit.semi_major_axis * (1.0 + orbit.eccentricity);
        let orbit_escapes = parent.parent.is_some() && apoapsis > parent.soi_radius;

        // Ship's orbital range (periapsis to apoapsis)
        let ship_periapsis = orbit.semi_major_axis * (1.0 - orbit.eccentricity);
        let ship_apoapsis = orbit.semi_major_axis * (1.0 + orbit.eccentricity);

        // Only check child bodies whose orbits could potentially intersect with ship's orbit
        let child_bodies: Vec<usize> = solar_system.bodies.iter()
            .enumerate()
            .filter(|(_, b)| b.parent == Some(parent_idx))
            .filter(|(_, b)| {
                // Check if child's orbital range overlaps with ship's orbital range
                if let Some(ref child_orbit) = b.orbit {
                    let child_periapsis = child_orbit.semi_major_axis * (1.0 - child_orbit.eccentricity);
                    let child_apoapsis = child_orbit.semi_major_axis * (1.0 + child_orbit.eccentricity);
                    // Add SOI radius as margin for intersection check
                    let child_inner = child_periapsis - b.soi_radius;
                    let child_outer = child_apoapsis + b.soi_radius;
                    // Orbits can intersect if ranges overlap
                    ship_apoapsis >= child_inner && ship_periapsis <= child_outer
                } else {
                    true // No orbit data, include to be safe
                }
            })
            .map(|(i, _)| i)
            .collect();
        let has_children = !child_bodies.is_empty();

        // Early exit: if orbit doesn't escape and there are no children, no SOI transitions possible
        if !orbit_escapes && !has_children {
            return None;
        }

        // Fast path: if orbit escapes but there are no children, use analytical SOI exit calculation
        if orbit_escapes && !has_children {
            if let Some(grandparent_idx) = parent.parent {
                let e = orbit.eccentricity;
                let a = orbit.semi_major_axis;
                let p = a * (1.0 - e * e); // semi-latus rectum
                let cos_nu = (p / parent.soi_radius - 1.0) / e;

                if cos_nu.abs() <= 1.0 {
                    let nu_exit = cos_nu.acos();
                    let exit_candidates = [nu_exit, std::f64::consts::TAU - nu_exit];

                    for exit_nu in exit_candidates {
                        let exit_ma = self.true_to_mean_anomaly(orbit, exit_nu);

                        let delta_ma = if retrograde {
                            let mut d = start_mean_anomaly - exit_ma;
                            if d < 0.0 { d += std::f64::consts::TAU; }
                            d
                        } else {
                            let mut d = exit_ma - start_mean_anomaly;
                            if d < 0.0 { d += std::f64::consts::TAU; }
                            d
                        };

                        let time_to_exit = delta_ma / mean_motion;

                        if time_to_exit > MIN_INTERSECTION_TIME {
                            return Some((exit_nu, time_to_exit, grandparent_idx, false));
                        }
                    }
                }
            }
            return None;
        }

        let num_samples = SOI_INTERSECTION_SAMPLES;
        let mut best_intersection: Option<(f64, f64, usize, bool)> = None;
        let mut min_time = f64::MAX;

        let get_sample_m = |delta_m: f64| -> f64 {
            if retrograde {
                (start_mean_anomaly - delta_m).rem_euclid(std::f64::consts::TAU)
            } else {
                (start_mean_anomaly + delta_m).rem_euclid(std::f64::consts::TAU)
            }
        };

        // Coarse sampling to find approximate intersections (only when there are children to check)
        for i in 0..num_samples {
            let fraction = i as f64 / num_samples as f64;
            let delta_m = fraction * std::f64::consts::TAU;
            let sample_m = get_sample_m(delta_m);
            let time_to_sample = delta_m / mean_motion;

            if time_to_sample >= min_time {
                continue;
            }

            let pos = orbit.position_from_mean_anomaly(sample_m, parent.mass);
            let dist_from_parent = (pos[0].powi(2) + pos[1].powi(2)).sqrt();

            // Check if exiting current SOI
            if dist_from_parent > parent.soi_radius * SOI_EXIT_THRESHOLD {
                if let Some(grandparent_idx) = parent.parent {
                    let true_anomaly = self.mean_to_true_anomaly(orbit, sample_m);
                    if time_to_sample < min_time && time_to_sample > MIN_INTERSECTION_TIME {
                        min_time = time_to_sample;
                        best_intersection = Some((true_anomaly, time_to_sample, grandparent_idx, false));
                    }
                }
            }

            // Check if entering a child body's SOI (only if there are children)
            if has_children {
                for &child_idx in &child_bodies {
                    let child = &solar_system.bodies[child_idx];

                    let child_pos = if let Some(ref child_orbit) = child.orbit {
                        child_orbit.position_at(base_time + time_to_sample, parent.mass)
                    } else {
                        self.get_body_position_relative(child_idx, parent_idx, solar_system)
                    };

                    let dx = pos[0] - child_pos[0];
                    let dy = pos[1] - child_pos[1];
                    let dist_from_child = (dx.powi(2) + dy.powi(2)).sqrt();

                    if dist_from_child < child.soi_radius * SOI_ENTRY_THRESHOLD {
                        let true_anomaly = self.mean_to_true_anomaly(orbit, sample_m);
                        if time_to_sample < min_time && time_to_sample > MIN_INTERSECTION_TIME {
                            min_time = time_to_sample;
                            best_intersection = Some((true_anomaly, time_to_sample, child_idx, true));
                        }
                    }
                }
            }
        }

        // Refine child body intersections with binary search
        if let Some((_, approx_time, child_idx, true)) = best_intersection {
            let child = &solar_system.bodies[child_idx];
            if let Some(ref child_orbit) = child.orbit {
                let search_window = std::f64::consts::TAU / num_samples as f64 * 2.0;
                let approx_delta_m = approx_time * mean_motion;

                let mut low_delta_m = (approx_delta_m - search_window).max(0.0);
                let mut high_delta_m = approx_delta_m + search_window;

                for _ in 0..SOI_REFINE_ITERATIONS {
                    let mid_delta_m = (low_delta_m + high_delta_m) / 2.0;
                    let mid_sample_m = get_sample_m(mid_delta_m);
                    let mid_time = mid_delta_m / mean_motion;

                    let ship_pos = orbit.position_from_mean_anomaly(mid_sample_m, parent.mass);
                    let child_pos = child_orbit.position_at(base_time + mid_time, parent.mass);

                    let dx = ship_pos[0] - child_pos[0];
                    let dy = ship_pos[1] - child_pos[1];
                    let dist = (dx.powi(2) + dy.powi(2)).sqrt();

                    if dist < child.soi_radius {
                        high_delta_m = mid_delta_m;
                    } else {
                        low_delta_m = mid_delta_m;
                    }
                }

                let refined_delta_m = (low_delta_m + high_delta_m) / 2.0;
                let refined_sample_m = get_sample_m(refined_delta_m);
                let refined_time = refined_delta_m / mean_motion;
                let refined_ta = self.mean_to_true_anomaly(orbit, refined_sample_m);

                best_intersection = Some((refined_ta, refined_time, child_idx, true));
            }
        }

        // Calculate analytically if orbit escapes but sampling didn't find intersection
        if best_intersection.is_none() && orbit_escapes {
            if let Some(grandparent_idx) = parent.parent {
                let e = orbit.eccentricity;
                let a = orbit.semi_major_axis;
                let p = a * (1.0 - e * e);
                let cos_nu = (p / parent.soi_radius - 1.0) / e;

                if cos_nu.abs() <= 1.0 {
                    let nu_exit = cos_nu.acos();
                    let exit_candidates = [nu_exit, std::f64::consts::TAU - nu_exit];

                    for exit_nu in exit_candidates {
                        let exit_ma = self.true_to_mean_anomaly(orbit, exit_nu);

                        let delta_ma = if retrograde {
                            let mut d = start_mean_anomaly - exit_ma;
                            if d < 0.0 { d += std::f64::consts::TAU; }
                            d
                        } else {
                            let mut d = exit_ma - start_mean_anomaly;
                            if d < 0.0 { d += std::f64::consts::TAU; }
                            d
                        };

                        let time_to_exit = delta_ma / mean_motion;

                        if time_to_exit > MIN_INTERSECTION_TIME && time_to_exit < min_time {
                            min_time = time_to_exit;
                            best_intersection = Some((exit_nu, time_to_exit, grandparent_idx, false));
                        }
                    }
                }
            }
        }

        best_intersection
    }
}
