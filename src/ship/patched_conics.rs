use crate::bodies::{SolarSystem, G, Orbit};
use super::{
    Ship, ShipState, PatchedConicSegment, PatchedTrajectory, MAX_PATCHED_CONICS,
    SOI_INTERSECTION_SAMPLES, SOI_EXIT_THRESHOLD, SOI_ENTRY_THRESHOLD,
    MIN_INTERSECTION_TIME, SOI_REFINE_ITERATIONS, HYPERBOLIC_ANGLE_MARGIN,
};

impl Ship {
    /// Calculate patched conics trajectory predicting future SOI changes
    pub fn calculate_patched_trajectory(&self, solar_system: &SolarSystem) -> Option<PatchedTrajectory> {
        if matches!(self.state, ShipState::Landed { .. }) {
            return None;
        }

        let mut segments = Vec::new();

        let parent_idx = self.soi_body;
        let parent = &solar_system.bodies[parent_idx];

        if let Some((orbit, true_anomaly, retrograde)) = self.calculate_orbit_from_state(
            self.rel_position,
            self.rel_velocity,
            parent.mass,
        ) {
            // If current orbit is hyperbolic, calculate where it exits the SOI
            if orbit.eccentricity >= 1.0 {
                let e = orbit.eccentricity;
                let a_abs = orbit.semi_major_axis.abs();
                let p = a_abs * (e * e - 1.0);

                let soi_radius = parent.soi_radius;
                let cos_nu_exit = (p / soi_radius - 1.0) / e;

                // For retrograde orbits, exit is at negative true anomaly
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
                    let current_mean_anomaly = self.true_to_mean_anomaly(&orbit, true_anomaly);

                    let mu = G * parent.mass;
                    let mean_motion = (mu / orbit.semi_major_axis.abs().powi(3)).sqrt();
                    // For retrograde, mean anomaly decreases, so reverse the difference
                    let time_to_exit = if retrograde {
                        (current_mean_anomaly - exit_mean_anomaly) / mean_motion
                    } else {
                        (exit_mean_anomaly - current_mean_anomaly) / mean_motion
                    };
                    let exit_time = solar_system.time + time_to_exit.abs().max(0.0);

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
                        exit_time,
                        solar_system,
                    );

                    let grandparent = &solar_system.bodies[grandparent_idx];
                    if let Some((new_orbit, new_ta, new_retro)) = self.calculate_orbit_from_state(
                        new_pos, new_vel, grandparent.mass,
                    ) {
                        segments.push(PatchedConicSegment {
                            orbit: new_orbit,
                            parent_idx: grandparent_idx,
                            retrograde: new_retro,
                            start_true_anomaly: new_ta,
                            end_true_anomaly: None,
                            start_time: 0.0,
                            end_time: None,
                        });
                    }
                }

                return Some(PatchedTrajectory { segments });
            }

            // Elliptical orbit - calculate trajectory with SOI transitions
            let mean_anomaly = self.true_to_mean_anomaly(&orbit, true_anomaly);
            let (current_orbit, current_mean_anomaly, current_retrograde) = (orbit, mean_anomaly, retrograde);

            let mut current_orbit = current_orbit;
            let mut current_parent_idx = parent_idx;
            let mut current_mean_anomaly = current_mean_anomaly;
            let mut current_retrograde = current_retrograde;
            let mut cumulative_time = 0.0;

            for _ in 0..MAX_PATCHED_CONICS {
                let parent = &solar_system.bodies[current_parent_idx];

                let intersection = self.find_soi_intersection(
                    &current_orbit,
                    current_parent_idx,
                    current_mean_anomaly,
                    current_retrograde,
                    solar_system,
                    solar_system.time + cumulative_time,
                );

                match intersection {
                    Some((intersect_true_anomaly, intersect_time, new_parent_idx, entry)) => {
                        segments.push(PatchedConicSegment {
                            orbit: current_orbit,
                            parent_idx: current_parent_idx,
                            retrograde: current_retrograde,
                            start_true_anomaly: self.mean_to_true_anomaly(&current_orbit, current_mean_anomaly),
                            end_true_anomaly: Some(intersect_true_anomaly),
                            start_time: cumulative_time,
                            end_time: Some(cumulative_time + intersect_time),
                        });

                        let intersect_mean_anomaly = self.true_to_mean_anomaly(&current_orbit, intersect_true_anomaly);
                        let pos = current_orbit.position_from_mean_anomaly(intersect_mean_anomaly, parent.mass);
                        let vel = current_orbit.velocity_from_mean_anomaly_with_direction(
                            intersect_mean_anomaly,
                            parent.mass,
                            current_retrograde,
                        );

                        let absolute_intersect_time = solar_system.time + cumulative_time + intersect_time;
                        let (new_pos, new_vel, _new_retrograde) = if entry {
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
                        if let Some((new_orbit, new_ta, retrograde)) = self.calculate_orbit_from_state(
                            new_pos, new_vel, new_parent.mass,
                        ) {
                            cumulative_time += intersect_time;

                            // Check if new orbit is hyperbolic
                            if new_orbit.eccentricity >= 1.0 {
                                let start_ta = new_ta;

                                let e = new_orbit.eccentricity;
                                let a_abs = new_orbit.semi_major_axis.abs();
                                let p = a_abs * (e * e - 1.0);
                                let soi_radius = new_parent.soi_radius;
                                let cos_nu_exit = (p / soi_radius - 1.0) / e;

                                let end_true_anomaly = if cos_nu_exit.abs() <= 1.0 {
                                    let nu_exit = cos_nu_exit.acos();
                                    if retrograde {
                                        Some(-nu_exit)
                                    } else {
                                        Some(nu_exit)
                                    }
                                } else {
                                    None
                                };

                                segments.push(PatchedConicSegment {
                                    orbit: new_orbit,
                                    parent_idx: new_parent_idx,
                                    retrograde,
                                    start_true_anomaly: start_ta,
                                    end_true_anomaly,
                                    start_time: cumulative_time,
                                    end_time: None,
                                });
                                break;
                            }

                            current_orbit = new_orbit;
                            current_parent_idx = new_parent_idx;
                            current_mean_anomaly = self.true_to_mean_anomaly(&new_orbit, new_ta);
                            current_retrograde = retrograde;
                        } else {
                            break;
                        }
                    }
                    None => {
                        segments.push(PatchedConicSegment {
                            orbit: current_orbit,
                            parent_idx: current_parent_idx,
                            retrograde: current_retrograde,
                            start_true_anomaly: self.mean_to_true_anomaly(&current_orbit, current_mean_anomaly),
                            end_true_anomaly: None,
                            start_time: cumulative_time,
                            end_time: None,
                        });
                        break;
                    }
                }
            }

            if segments.is_empty() {
                None
            } else {
                Some(PatchedTrajectory { segments })
            }
        } else {
            None
        }
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

        // Count child bodies that could be entered
        let child_bodies: Vec<usize> = solar_system.bodies.iter()
            .enumerate()
            .filter(|(_, b)| b.parent == Some(parent_idx))
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
            if child.orbit.is_some() {
                let search_window = std::f64::consts::TAU / num_samples as f64 * 2.0;
                let approx_delta_m = approx_time * mean_motion;

                let mut low_delta_m = (approx_delta_m - search_window).max(0.0);
                let mut high_delta_m = approx_delta_m + search_window;

                for _ in 0..SOI_REFINE_ITERATIONS {
                    let mid_delta_m = (low_delta_m + high_delta_m) / 2.0;
                    let mid_sample_m = get_sample_m(mid_delta_m);
                    let mid_time = mid_delta_m / mean_motion;

                    let ship_pos = orbit.position_from_mean_anomaly(mid_sample_m, parent.mass);
                    let child_pos = child.orbit.as_ref().unwrap()
                        .position_at(base_time + mid_time, parent.mass);

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
