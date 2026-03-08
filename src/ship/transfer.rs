//! Transfer planner: Hohmann, Lambert, and interplanetary transfer calculations

use crate::bodies::{CelestialBody, G, Orbit};
use std::f64::consts::{PI, TAU};

/// Result of a Hohmann transfer calculation
pub struct HohmannResult {
    pub departure_delta_v: f64,       // m/s prograde
    pub arrival_delta_v: f64,         // m/s (info only)
    pub transfer_time: f64,           // seconds
    pub required_phase_angle: f64,    // radians
    pub current_phase_angle: f64,     // radians
    pub time_to_window: f64,          // seconds until phase angle matches
    pub departure_position_angle: f64, // inertial angle where burn should happen (radians)
}

/// Result of a Lambert solver
pub struct LambertResult {
    pub v1: [f64; 2], // departure velocity vector
    pub v2: [f64; 2], // arrival velocity vector
}

/// Result of an interplanetary transfer calculation
pub struct InterplanetaryResult {
    pub ejection_delta_v: f64,        // total burn magnitude at parking orbit
    pub ejection_prograde: f64,       // prograde component in ship frame
    pub ejection_radial: f64,         // radial component in ship frame
    pub v_infinity: f64,              // excess velocity at SOI edge
    pub transfer_time: f64,           // seconds
    pub departure_position_angle: f64, // inertial angle where ejection burn should happen (radians)
    pub current_phase_angle: f64,     // radians (between planets)
    pub required_phase_angle: f64,    // radians
    pub time_to_window: f64,          // seconds
    pub arrival_v_infinity: f64,      // m/s at target
}

/// Pre-formatted transfer data for the UI (avoids complex structs in RenderState)
pub struct TransferDisplay {
    pub mode: u8,                      // 0 = Hohmann, 1 = Lambert
    pub target_name: String,
    pub departure_dv: f64,             // m/s
    pub arrival_dv: f64,               // m/s (Hohmann) or arrival v_infinity (Lambert)
    pub transfer_time: f64,            // seconds
    pub current_phase_angle: f64,      // degrees
    pub required_phase_angle: f64,     // degrees
    pub time_to_window: f64,           // seconds
    pub departure_position_angle: f64, // inertial position angle for node placement (radians)
    pub prograde_dv: f64,              // prograde component for the node
    pub radial_dv: f64,                // radial component for the node
    pub valid: bool,
}

/// Get valid Hohmann transfer targets (bodies within the same SOI, i.e. children of ship_soi)
/// Example: ship orbits Earth → targets are Moon (orbits Earth)
pub fn hohmann_targets(ship_soi: usize, bodies: &[CelestialBody]) -> Vec<(usize, String)> {
    bodies.iter().enumerate()
        .filter(|(_, b)| b.parent == Some(ship_soi) && b.orbit.is_some())
        .map(|(i, b)| (i, b.name.clone()))
        .collect()
}

/// Get valid Lambert transfer targets (siblings of ship_soi orbiting the same parent)
/// Example: ship orbits Earth → targets are Mars, Venus, etc. (also orbit Sun)
pub fn lambert_targets(ship_soi: usize, bodies: &[CelestialBody]) -> Vec<(usize, String)> {
    let parent = match bodies[ship_soi].parent {
        Some(p) => p,
        None => return Vec::new(),
    };
    bodies.iter().enumerate()
        .filter(|(i, b)| *i != ship_soi && b.parent == Some(parent) && b.orbit.is_some())
        .map(|(i, b)| (i, b.name.clone()))
        .collect()
}

/// Compute a Hohmann transfer to a sibling body
pub fn compute_hohmann(
    ship_orbit: &Orbit,
    ship_retrograde: bool,
    ship_mean_anomaly: f64,
    target_orbit: &Orbit,
    parent_mass: f64,
    sim_time: f64,
) -> Option<HohmannResult> {
    let mu = G * parent_mass;

    let r1 = ship_orbit.semi_major_axis;
    let r2 = target_orbit.semi_major_axis;

    if r1 <= 0.0 || r2 <= 0.0 || mu <= 0.0 {
        return None;
    }

    // Transfer orbit semi-major axis
    let a_t = (r1 + r2) / 2.0;

    // Departure delta-v (tangential burn)
    let v_circ1 = (mu / r1).sqrt();
    let v_transfer_departure = (mu * (2.0 / r1 - 1.0 / a_t)).sqrt();
    let departure_dv = v_transfer_departure - v_circ1;

    // Arrival delta-v
    let v_circ2 = (mu / r2).sqrt();
    let v_transfer_arrival = (mu * (2.0 / r2 - 1.0 / a_t)).sqrt();
    let arrival_dv = (v_circ2 - v_transfer_arrival).abs();

    // Transfer time (half orbit)
    let transfer_time = PI * (a_t.powi(3) / mu).sqrt();

    // Target's angular velocity
    let omega_target = target_orbit.mean_motion(parent_mass);

    // Required phase angle: where target must be relative to ship at departure
    // so that target arrives at the transfer orbit's destination when ship does
    let required_phase = if r2 > r1 {
        // Transfer to higher orbit
        PI - omega_target * transfer_time
    } else {
        // Transfer to lower orbit
        -(PI - omega_target * transfer_time)
    };

    // Normalize to [-PI, PI]
    let required_phase = normalize_angle(required_phase);

    // Current phase angle: target position - ship position
    let ship_ea = ship_orbit.solve_kepler(ship_mean_anomaly);
    let ship_ta = ship_orbit.true_anomaly(ship_ea);
    let ship_angle = ship_ta + ship_orbit.argument_of_periapsis;

    let target_ma = target_orbit.mean_anomaly_at(sim_time, parent_mass);
    let target_ea = target_orbit.solve_kepler(target_ma);
    let target_ta = target_orbit.true_anomaly(target_ea);
    let target_angle = target_ta + target_orbit.argument_of_periapsis;

    let current_phase = normalize_angle(target_angle - ship_angle);

    // Time to window from synodic period
    // Use signed rate: phase = (target_angle - ship_angle) changes at (omega_target - omega_ship_signed)
    let omega_ship = ship_orbit.mean_motion(parent_mass);
    let omega_ship_signed = if ship_retrograde { -omega_ship } else { omega_ship };
    let synodic_rate_signed = omega_target - omega_ship_signed;
    let time_to_window = if synodic_rate_signed.abs() > 1e-15 {
        let phase_diff = normalize_angle(required_phase - current_phase);
        let t = phase_diff / synodic_rate_signed;
        let synodic_period = TAU / synodic_rate_signed.abs();
        t.rem_euclid(synodic_period)
    } else {
        0.0
    };

    // Where to place the departure node: propagate ship's mean anomaly forward
    // Return the inertial position angle (not true anomaly) so it's independent of arg_peri,
    // which is ill-defined for near-circular parking orbits.
    let departure_ma = ship_mean_anomaly + omega_ship * time_to_window;
    let direction = if ship_retrograde { -1.0 } else { 1.0 };
    let departure_ma = departure_ma * direction;
    let departure_ea = ship_orbit.solve_kepler(departure_ma);
    let departure_ta = ship_orbit.true_anomaly(departure_ea);
    let departure_position = departure_ta + ship_orbit.argument_of_periapsis;

    Some(HohmannResult {
        departure_delta_v: departure_dv,
        arrival_delta_v: arrival_dv,
        transfer_time,
        required_phase_angle: required_phase,
        current_phase_angle: current_phase,
        time_to_window,
        departure_position_angle: departure_position,
    })
}

// --- Stumpff functions for universal variable Lambert solver ---

fn stumpff_c2(psi: f64) -> f64 {
    if psi.abs() < 1e-6 {
        // Taylor series: 1/2 - psi/24 + psi^2/720 - ...
        1.0 / 2.0 - psi / 24.0 + psi * psi / 720.0
    } else if psi > 0.0 {
        (1.0 - psi.sqrt().cos()) / psi
    } else {
        ((-psi).sqrt().cosh() - 1.0) / (-psi)
    }
}

fn stumpff_c3(psi: f64) -> f64 {
    if psi.abs() < 1e-6 {
        // Taylor series: 1/6 - psi/120 + psi^2/5040 - ...
        1.0 / 6.0 - psi / 120.0 + psi * psi / 5040.0
    } else if psi > 0.0 {
        let sqrt_psi = psi.sqrt();
        (sqrt_psi - sqrt_psi.sin()) / (psi * sqrt_psi)
    } else {
        let sqrt_neg = (-psi).sqrt();
        (sqrt_neg.sinh() - sqrt_neg) / ((-psi) * sqrt_neg)
    }
}

/// Solve the Lambert problem in 2D using universal variable formulation
pub fn solve_lambert_2d(
    r1_vec: [f64; 2],
    r2_vec: [f64; 2],
    tof: f64,
    mu: f64,
    prograde: bool,
) -> Option<LambertResult> {
    let r1_mag = (r1_vec[0].powi(2) + r1_vec[1].powi(2)).sqrt();
    let r2_mag = (r2_vec[0].powi(2) + r2_vec[1].powi(2)).sqrt();

    if r1_mag < 1e-6 || r2_mag < 1e-6 || tof <= 0.0 {
        return None;
    }

    // Cross product (z-component in 2D) to determine transfer angle
    let cross = r1_vec[0] * r2_vec[1] - r1_vec[1] * r2_vec[0];
    let cos_dnu = (r1_vec[0] * r2_vec[0] + r1_vec[1] * r2_vec[1]) / (r1_mag * r2_mag);
    let cos_dnu = cos_dnu.clamp(-1.0, 1.0);

    let sin_dnu = if prograde {
        if cross >= 0.0 { (1.0 - cos_dnu * cos_dnu).sqrt() } else { -(1.0 - cos_dnu * cos_dnu).sqrt() }
    } else if cross >= 0.0 {
        -(1.0 - cos_dnu * cos_dnu).sqrt()
    } else {
        (1.0 - cos_dnu * cos_dnu).sqrt()
    };

    // Check for degenerate 180-degree transfer
    if (1.0 - cos_dnu).abs() < 1e-12 {
        return None;
    }
    let big_a = sin_dnu * (r1_mag * r2_mag / (1.0 - cos_dnu)).sqrt();
    if big_a.abs() < 1e-12 {
        return None;
    }

    // Newton-Raphson with bisection fallback on universal variable psi
    let mut psi_low = -4.0 * PI * PI;
    let mut psi_high = 4.0 * PI * PI;
    let mut psi = 0.0;

    for _ in 0..60 {
        let c2 = stumpff_c2(psi);
        let c3 = stumpff_c3(psi);

        // y = r1 + r2 + A * (psi * c3 - 1) / sqrt(c2)
        if c2.abs() < 1e-20 {
            psi = (psi_low + psi_high) / 2.0;
            continue;
        }
        let y = r1_mag + r2_mag + big_a * (psi * c3 - 1.0) / c2.sqrt();

        if y < 0.0 {
            // Adjust bounds
            psi_low = psi;
            psi = (psi_low + psi_high) / 2.0;
            continue;
        }

        let chi = (y / c2).sqrt();
        let f_psi = chi.powi(3) * c3 + big_a * y.sqrt() - tof * mu.sqrt();

        // Derivative
        let d_psi = if psi.abs() > 1e-6 {
            let term1 = chi.powi(3) * (stumpff_c3(psi) - 3.0 * c3 * stumpff_c2(psi) / (2.0 * c2)) / (2.0 * psi);
            let term2 = 3.0 * c3 * chi / (2.0 * c2);
            term1 + term2 + big_a / (2.0 * y.sqrt()) * (c2.sqrt() - psi * stumpff_c3(psi) / c2.sqrt()) * chi
        } else {
            // Near zero: simplified
            let term = (2.0 / 5.0_f64.sqrt()) * y.powf(1.5) / 3.0 + big_a * y.sqrt() / 2.0;
            if term.abs() < 1e-20 { 1.0 } else { term }
        };

        // Newton step with bounds checking
        if d_psi.abs() < 1e-20 {
            psi = (psi_low + psi_high) / 2.0;
            continue;
        }

        let psi_new = psi - f_psi / d_psi;
        if f_psi < 0.0 {
            psi_low = psi;
        } else {
            psi_high = psi;
        }

        // Use Newton step if in bounds, bisection otherwise
        psi = if psi_new > psi_low && psi_new < psi_high {
            psi_new
        } else {
            (psi_low + psi_high) / 2.0
        };

        if (f_psi / mu.sqrt()).abs() < 1e-10 {
            break;
        }
    }

    let c2 = stumpff_c2(psi);
    let c3 = stumpff_c3(psi);
    let y = r1_mag + r2_mag + big_a * (psi * c3 - 1.0) / c2.sqrt();

    if y < 0.0 {
        return None;
    }

    // Lagrange coefficients
    let f = 1.0 - y / r1_mag;
    let g_dot = 1.0 - y / r2_mag;
    let g = big_a * (y / mu).sqrt();

    if g.abs() < 1e-20 {
        return None;
    }

    let v1 = [
        (r2_vec[0] - f * r1_vec[0]) / g,
        (r2_vec[1] - f * r1_vec[1]) / g,
    ];
    let v2 = [
        (g_dot * r2_vec[0] - r1_vec[0]) / g,
        (g_dot * r2_vec[1] - r1_vec[1]) / g,
    ];

    // Validate result
    let v1_mag = (v1[0].powi(2) + v1[1].powi(2)).sqrt();
    let v2_mag = (v2[0].powi(2) + v2[1].powi(2)).sqrt();
    if !v1_mag.is_finite() || !v2_mag.is_finite() || v1_mag > 1e8 || v2_mag > 1e8 {
        return None;
    }

    Some(LambertResult { v1, v2 })
}

/// Compute time to traverse a hyperbolic escape trajectory from periapsis to SOI boundary.
/// Used to adjust the Lambert departure time so the v_inf direction is correct at SOI exit
/// rather than at the LEO burn point.
fn compute_hyperbolic_escape_time(
    v_inf_mag: f64,
    r_periapsis: f64,
    soi_radius: f64,
    mu: f64,
) -> f64 {
    if v_inf_mag <= 0.0 || r_periapsis <= 0.0 || soi_radius <= r_periapsis {
        return 0.0;
    }

    // Hyperbolic orbit parameters
    let a_hyp = mu / v_inf_mag.powi(2); // |a| (positive value; real a is negative for hyperbola)
    let e_hyp = 1.0 + r_periapsis / a_hyp; // = 1 + r_p * v_inf^2 / mu

    // Semi-latus rectum: p = |a| * (e^2 - 1)
    let p = a_hyp * (e_hyp.powi(2) - 1.0);

    // True anomaly at SOI radius: r = p / (1 + e*cos(ta))
    let cos_ta = ((p / soi_radius) - 1.0) / e_hyp;
    if cos_ta.abs() > 1.0 {
        return 0.0; // SOI beyond hyperbola's reach
    }
    let ta_exit = cos_ta.acos();

    // Convert true anomaly to hyperbolic anomaly H
    // tanh(H/2) = sqrt((e-1)/(e+1)) * tan(ta/2)
    let tan_half_ta = (ta_exit / 2.0).tan();
    let factor = ((e_hyp - 1.0) / (e_hyp + 1.0)).sqrt();
    let tanh_half_h = factor * tan_half_ta;
    if tanh_half_h.abs() >= 1.0 {
        return 0.0;
    }
    let h_exit = 2.0 * tanh_half_h.atanh();

    // Hyperbolic mean anomaly: M = e * sinh(H) - H
    let m_hyp = e_hyp * h_exit.sinh() - h_exit;

    // Mean motion: n = sqrt(mu / |a|^3)
    let n = (mu / a_hyp.powi(3)).sqrt();

    if n > 0.0 { m_hyp / n } else { 0.0 }
}

/// Compute an interplanetary transfer using Lambert solver
pub fn compute_interplanetary(
    ship_orbit: &Orbit,
    ship_retrograde: bool,
    _ship_mean_anomaly: f64,
    ship_soi_body: usize,        // e.g. Earth (index 4)
    target_body: usize,          // e.g. Mars (index 6)
    departure_time: f64,         // absolute sim time of departure
    arrival_time: f64,           // absolute sim time of arrival
    sim_time: f64,               // current simulation time
    bodies: &[CelestialBody],
) -> Option<InterplanetaryResult> {
    let parent = bodies[ship_soi_body].parent?; // e.g. Sun
    let grandparent_mass = bodies[parent].mass;
    let mu_sun = G * grandparent_mass;

    let departure_orbit = bodies[ship_soi_body].orbit.as_ref()?;
    let target_orbit = bodies[target_body].orbit.as_ref()?;

    let mu_planet = G * bodies[ship_soi_body].mass;
    let r_parking = ship_orbit.semi_major_axis;
    if r_parking <= 0.0 {
        return None;
    }

    // --- Step 1: Initial Lambert solve to get approximate v_inf ---
    let r1_vec = departure_orbit.position_at(departure_time, grandparent_mass);
    let r2_vec = target_orbit.position_at(arrival_time, grandparent_mass);
    let tof = arrival_time - departure_time;
    if tof <= 0.0 {
        return None;
    }

    let lambert_initial = solve_lambert_2d(r1_vec, r2_vec, tof, mu_sun, true)?;
    let dep_ma_initial = departure_orbit.mean_anomaly_at(departure_time, grandparent_mass);
    let planet_vel_initial = departure_orbit.velocity_from_mean_anomaly(dep_ma_initial, grandparent_mass);
    let v_inf_initial_mag = {
        let vx = lambert_initial.v1[0] - planet_vel_initial[0];
        let vy = lambert_initial.v1[1] - planet_vel_initial[1];
        (vx.powi(2) + vy.powi(2)).sqrt()
    };

    // --- Step 2: Compute escape time (LEO burn to SOI exit on hyperbolic trajectory) ---
    // The ship takes hours/days to escape from parking orbit to the SOI boundary.
    // During this time the departure planet moves in its orbit, changing the required
    // v_inf direction. We correct for this by solving Lambert for the SOI exit time.
    let soi_radius = bodies[ship_soi_body].soi_radius;
    let escape_time = compute_hyperbolic_escape_time(
        v_inf_initial_mag, r_parking, soi_radius, mu_planet,
    );

    // --- Step 3: Re-solve Lambert for effective departure (SOI exit time) ---
    let effective_departure = departure_time + escape_time;
    let r1_corrected = departure_orbit.position_at(effective_departure, grandparent_mass);
    let tof_corrected = arrival_time - effective_departure;
    if tof_corrected <= 0.0 {
        return None;
    }

    let lambert = solve_lambert_2d(r1_corrected, r2_vec, tof_corrected, mu_sun, true)?;

    // Corrected v_inf: what the ship needs at SOI exit
    let dep_ma = departure_orbit.mean_anomaly_at(effective_departure, grandparent_mass);
    let planet_vel = departure_orbit.velocity_from_mean_anomaly(dep_ma, grandparent_mass);
    let v_inf = [
        lambert.v1[0] - planet_vel[0],
        lambert.v1[1] - planet_vel[1],
    ];
    let v_inf_mag = (v_inf[0].powi(2) + v_inf[1].powi(2)).sqrt();

    // Arrival v_inf (arrival time unchanged, but Lambert solution differs slightly)
    let arr_ma = target_orbit.mean_anomaly_at(arrival_time, grandparent_mass);
    let target_vel = target_orbit.velocity_from_mean_anomaly(arr_ma, grandparent_mass);
    let arr_v_inf = [
        lambert.v2[0] - target_vel[0],
        lambert.v2[1] - target_vel[1],
    ];
    let arrival_v_infinity = (arr_v_inf[0].powi(2) + arr_v_inf[1].powi(2)).sqrt();

    // Ejection burn from parking orbit
    let v_parking = (mu_planet / r_parking).sqrt();
    let v_ejection = (v_inf_mag.powi(2) + 2.0 * mu_planet / r_parking).sqrt();
    let ejection_dv = (v_ejection - v_parking).abs();

    // Ejection angle calculation: find the position on the parking orbit where a
    // purely prograde burn produces an escape trajectory with the desired v_infinity direction.
    //
    // A prograde burn on a circular orbit keeps the burn point as periapsis of the
    // resulting hyperbola. We need to find the burn position (arg of periapsis of the
    // hyperbola) such that at the SOI exit, the velocity direction matches what Lambert needs.
    //
    // The velocity direction at true anomaly ta on a prograde hyperbola with arg_peri ω is:
    //   vel_angle = ω + atan2(e + cos(ta), -sin(ta))
    // Setting vel_angle = v_inf_angle and solving for ω gives the burn position.
    //
    // Using the actual SOI exit true anomaly (finite distance) instead of the asymptotic
    // turn angle acos(-1/e) accounts for Earth's gravity still curving the trajectory
    // at the SOI boundary.

    // Hyperbolic eccentricity of the ejection orbit
    let e_hyp = 1.0 + r_parking * v_inf_mag.powi(2) / mu_planet;

    // True anomaly at SOI exit on the escape hyperbola
    let a_hyp = mu_planet / v_inf_mag.powi(2);
    let p_hyp = a_hyp * (e_hyp.powi(2) - 1.0);
    let cos_ta_exit = ((p_hyp / soi_radius) - 1.0) / e_hyp;
    let ta_exit = cos_ta_exit.clamp(-1.0, 1.0).acos();

    // Turn angle from periapsis to the velocity direction at SOI exit
    // (replaces the asymptotic theta_inf = acos(-1/e) with the finite-distance version)
    let turn_angle = (e_hyp + ta_exit.cos()).atan2(-ta_exit.sin());

    // V-infinity direction in planet-centered frame (velocity at SOI exit from Lambert)
    let v_inf_angle = v_inf[1].atan2(v_inf[0]);

    // Ejection position angle (where on the orbit to burn)
    // burn_position = v_inf_angle - turn_angle (prograde)
    // burn_position = v_inf_angle + turn_angle (retrograde)
    let ejection_position_angle = if ship_retrograde {
        v_inf_angle + turn_angle
    } else {
        v_inf_angle - turn_angle
    };

    // Return the inertial position angle directly (not true anomaly).
    // Converting to true anomaly here would use ship_orbit.argument_of_periapsis, which is
    // ill-defined for near-circular parking orbits. The caller converts to true anomaly
    // using the trajectory segment's arg_peri at node creation time.

    // Prograde-only: full ejection dv is prograde
    let ejection_prograde = ejection_dv;
    let ejection_radial = 0.0;

    // Phase angles between planets at current time (not departure time)
    let dep_ma_now = departure_orbit.mean_anomaly_at(sim_time, grandparent_mass);
    let dep_ea_now = departure_orbit.solve_kepler(dep_ma_now);
    let dep_ta_now = departure_orbit.true_anomaly(dep_ea_now);
    let dep_angle_now = dep_ta_now + departure_orbit.argument_of_periapsis;

    let target_ma_now = target_orbit.mean_anomaly_at(sim_time, grandparent_mass);
    let target_ea_now = target_orbit.solve_kepler(target_ma_now);
    let target_ta_now = target_orbit.true_anomaly(target_ea_now);
    let target_angle_now = target_ta_now + target_orbit.argument_of_periapsis;

    let current_phase = normalize_angle(target_angle_now - dep_angle_now);

    // Required phase angle (from Hohmann approximation for reference)
    let a_t_ip = (departure_orbit.semi_major_axis + target_orbit.semi_major_axis) / 2.0;
    let hohmann_tof = PI * (a_t_ip.powi(3) / mu_sun).sqrt();
    let omega_target = target_orbit.mean_motion(grandparent_mass);
    let required_phase = normalize_angle(PI - omega_target * hohmann_tof);

    // Time to departure — if departure_time is in the past, advance by
    // synodic period(s) to the next window with the same geometry.
    let raw_dt = departure_time - sim_time;
    let time_to_window = if raw_dt >= 0.0 {
        raw_dt
    } else {
        let omega_dep = departure_orbit.mean_motion(grandparent_mass);
        let omega_tgt = target_orbit.mean_motion(grandparent_mass);
        let synodic_rate = (omega_dep - omega_tgt).abs();
        if synodic_rate > 1e-15 {
            let synodic_period = TAU / synodic_rate;
            let periods_behind = (-raw_dt / synodic_period).ceil();
            raw_dt + periods_behind * synodic_period
        } else {
            0.0
        }
    };

    Some(InterplanetaryResult {
        ejection_delta_v: ejection_dv,
        ejection_prograde,
        ejection_radial,
        v_infinity: v_inf_mag,
        transfer_time: tof,
        departure_position_angle: ejection_position_angle,
        current_phase_angle: current_phase,
        required_phase_angle: required_phase,
        time_to_window,
        arrival_v_infinity,
    })
}

/// Calculate Hohmann-optimal departure and arrival times for Lambert mode defaults
pub fn hohmann_optimal_times(
    ship_soi_body: usize,
    target_body: usize,
    sim_time: f64,
    bodies: &[CelestialBody],
) -> Option<(f64, f64)> {
    let parent = bodies[ship_soi_body].parent?;
    let grandparent_mass = bodies[parent].mass;
    let mu_sun = G * grandparent_mass;

    let dep_orbit = bodies[ship_soi_body].orbit.as_ref()?;
    let target_orbit = bodies[target_body].orbit.as_ref()?;

    // Hohmann transfer time
    let a_t = (dep_orbit.semi_major_axis + target_orbit.semi_major_axis) / 2.0;
    let transfer_time = PI * (a_t.powi(3) / mu_sun).sqrt();

    // Find when phase angle matches
    let omega_dep = dep_orbit.mean_motion(grandparent_mass);
    let omega_target = target_orbit.mean_motion(grandparent_mass);
    let required_phase = normalize_angle(PI - omega_target * transfer_time);

    let dep_ma = dep_orbit.mean_anomaly_at(sim_time, grandparent_mass);
    let dep_ea = dep_orbit.solve_kepler(dep_ma);
    let dep_ta = dep_orbit.true_anomaly(dep_ea);
    let dep_angle = dep_ta + dep_orbit.argument_of_periapsis;

    let target_ma = target_orbit.mean_anomaly_at(sim_time, grandparent_mass);
    let target_ea = target_orbit.solve_kepler(target_ma);
    let target_ta = target_orbit.true_anomaly(target_ea);
    let target_angle = target_ta + target_orbit.argument_of_periapsis;

    let current_phase = normalize_angle(target_angle - dep_angle);

    // Signed rate: phase = (target_angle - dep_angle) changes at (omega_target - omega_dep)
    let synodic_rate_signed = omega_target - omega_dep;
    let time_to_window = if synodic_rate_signed.abs() > 1e-15 {
        let phase_diff = normalize_angle(required_phase - current_phase);
        let t = phase_diff / synodic_rate_signed;
        let synodic_period = TAU / synodic_rate_signed.abs();
        t.rem_euclid(synodic_period)
    } else {
        0.0
    };

    let departure_time = sim_time + time_to_window;
    let arrival_time = departure_time + transfer_time;

    Some((departure_time, arrival_time))
}

/// Compute a porkchop plot grid of Lambert transfer delta-v values.
/// Horizontal axis: departure time over one synodic period (full transfer window cycle).
/// Vertical axis: transfer time from tof_min to tof_max (log scale).
/// Takes Copy params so it can be called from a background thread.
pub fn compute_porkchop_grid(
    departure_orbit: Orbit,
    target_orbit: Orbit,
    grandparent_mass: f64,
    planet_mass: f64,
    sim_time: f64,
    parking_radius: f64,
    target_idx: usize,
) -> Option<crate::render::PorkchopGrid> {
    use crate::render::{PorkchopGrid, PorkchopPoint};

    let mu_sun = G * grandparent_mass;
    let mu_planet = G * planet_mass;

    // Departure axis: one full synodic period (cycle of transfer windows).
    // The synodic period is the time between successive identical phase angles,
    // which is longer than either orbital period when the orbits are close.
    let omega_dep = departure_orbit.mean_motion(grandparent_mass);
    let omega_tgt = target_orbit.mean_motion(grandparent_mass);
    let synodic_rate = (omega_dep - omega_tgt).abs();
    let synodic_period = if synodic_rate > 1e-15 {
        TAU / synodic_rate
    } else {
        // Degenerate case: same orbital period, fall back to departure period
        TAU / omega_dep
    };
    let dep_start = sim_time;
    let dep_end = sim_time + synodic_period;

    // Hohmann TOF for transfer between the two orbits
    let a_t = (departure_orbit.semi_major_axis + target_orbit.semi_major_axis) / 2.0;
    let hohmann_tof = PI * (a_t.powi(3) / mu_sun).sqrt();

    // Transfer time axis (log scale): hohmann_tof/100 to hohmann_tof*2
    let tof_min = hohmann_tof / 100.0;
    let tof_max = hohmann_tof * 2.0;

    let cols: usize = 60;
    let rows: usize = 50;

    // Precompute per-column departure positions and velocities
    // (same departure time for every row, avoids redundant Kepler solves)
    let mut dep_positions: Vec<[f64; 2]> = Vec::with_capacity(cols);
    let mut dep_velocities: Vec<[f64; 2]> = Vec::with_capacity(cols);
    let mut dep_times: Vec<f64> = Vec::with_capacity(cols);
    for col in 0..cols {
        let dep_time = dep_start + (col as f64 / cols as f64) * (dep_end - dep_start);
        dep_times.push(dep_time);
        dep_positions.push(departure_orbit.position_at(dep_time, grandparent_mass));
        let dep_ma = departure_orbit.mean_anomaly_at(dep_time, grandparent_mass);
        dep_velocities.push(departure_orbit.velocity_from_mean_anomaly(dep_ma, grandparent_mass));
    }

    let v_parking = (mu_planet / parking_radius).sqrt();

    let mut points: Vec<Option<PorkchopPoint>> = Vec::with_capacity(cols * rows);
    let mut min_dv = f64::MAX;
    let mut max_dv = 0.0_f64;
    let mut best_idx: Option<usize> = None;

    let log_ratio = (tof_max / tof_min).ln();

    for row in 0..rows {
        // Log-spaced TOF: tof = tof_min * exp(row/rows * ln(tof_max/tof_min))
        let t = row as f64 / rows as f64;
        let tof = tof_min * (t * log_ratio).exp();

        for col in 0..cols {
            let idx = row * cols + col;

            // Use precomputed departure position; only target position needs Kepler solve
            let r1 = dep_positions[col];
            let r2 = target_orbit.position_at(dep_times[col] + tof, grandparent_mass);

            // Solve Lambert
            let point = solve_lambert_2d(r1, r2, tof, mu_sun, true).and_then(|lambert| {
                let planet_vel = dep_velocities[col];

                // V-infinity = Lambert departure velocity - planet velocity
                let v_inf_x = lambert.v1[0] - planet_vel[0];
                let v_inf_y = lambert.v1[1] - planet_vel[1];
                let v_inf_sq = v_inf_x * v_inf_x + v_inf_y * v_inf_y;

                // Ejection delta-v from parking orbit using vis-viva
                let v_ejection = (v_inf_sq + 2.0 * mu_planet / parking_radius).sqrt();
                let ejection_dv = v_ejection - v_parking;

                if !ejection_dv.is_finite() || ejection_dv < 0.0 {
                    return None;
                }

                Some(PorkchopPoint {
                    ejection_dv,
                    dep_time: dep_times[col],
                    tof,
                })
            });

            if let Some(ref p) = point {
                if p.ejection_dv < min_dv {
                    min_dv = p.ejection_dv;
                    best_idx = Some(idx);
                }
                if p.ejection_dv > max_dv {
                    max_dv = p.ejection_dv;
                }
            }

            points.push(point);
        }
    }

    // No clamping — log-scale coloring handles the full range

    Some(PorkchopGrid {
        points,
        cols,
        rows,
        dep_start,
        dep_end,
        tof_min,
        tof_max,
        min_dv,
        max_dv,
        best_idx,
        target_idx,
    })
}

/// Normalize angle to [-PI, PI]
pub fn normalize_angle(mut angle: f64) -> f64 {
    while angle > PI {
        angle -= TAU;
    }
    while angle < -PI {
        angle += TAU;
    }
    angle
}
