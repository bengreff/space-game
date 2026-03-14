use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Atmosphere data for a celestial body
#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    pub surface_pressure: f64,    // Pascals (Pa) at sea level
    pub scale_height: f64,        // meters - height at which pressure drops by factor e
    pub color: [f32; 3],          // RGB color at surface level
}

impl Atmosphere {
    /// Atmosphere cutoff height in meters (~100km for Earth's 8.5km scale height)
    pub fn visible_height(&self) -> f64 {
        self.scale_height * (100_000.0 / 8_500.0)
    }


    /// Pressure at a given altitude (meters above surface)
    pub fn pressure_at_altitude(&self, altitude: f64) -> f64 {
        self.surface_pressure * (-altitude / self.scale_height).exp()
    }

    /// Surface density (kg/m³) using ideal gas law with fixed T=250K, R=287 J/(kg·K) for N2/O2
    pub fn surface_density(&self) -> f64 {
        self.surface_pressure / (287.0 * 250.0)
    }

    /// Density at a given altitude (meters above surface)
    pub fn density_at_altitude(&self, altitude: f64) -> f64 {
        self.surface_density() * (-altitude / self.scale_height).exp()
    }
}

/// Accretion disc data for supermassive objects (e.g., Sagittarius A*)
#[derive(Clone, Copy, Debug)]
pub struct AccretionDisc {
    pub inner_radius: f64,     // meters
    pub outer_radius: f64,     // meters
    pub color_inner: [f32; 3], // RGB (hot white/yellow)
    pub color_outer: [f32; 3], // RGB (orange/red)
}

/// One light-year in meters
pub const LIGHT_YEAR: f64 = 9.461e15;

/// Celestial body representation
#[derive(Clone)]
pub struct CelestialBody {
    pub name: String,
    pub description: String,
    pub mass: f64,           // kg
    pub radius: f64,         // meters
    pub color: [f32; 4],     // RGBA
    pub parent: Option<usize>, // Index of parent body (None for root)
    pub orbit: Option<Orbit>,  // Orbital parameters (None for root)
    pub soi_radius: f64,     // Sphere of influence radius
    pub atmosphere: Option<Atmosphere>, // Atmospheric data (None = no atmosphere)
    pub sidereal_period: Option<f64>,  // Rotation period in seconds (None = tidally locked / negligible)
    pub accretion_disc: Option<AccretionDisc>, // Accretion disc (e.g., for black holes)
    pub galactic_mass_profile: bool, // If true, effective_mass_at(r) uses enclosed galactic mass model
}

impl CelestialBody {
    /// Surface gravity in m/s² (g = GM/r²)
    pub fn surface_gravity(&self) -> f64 {
        G * self.mass / (self.radius * self.radius)
    }

    /// Returns the gravitational mass felt at distance `distance` from this body's center.
    /// For bodies with a galactic mass profile (Sgr A*), returns the enclosed galactic mass
    /// from the 4-component Milky Way model. For all other bodies, returns `self.mass`.
    pub fn effective_mass_at(&self, distance: f64) -> f64 {
        if self.galactic_mass_profile {
            galactic_enclosed_mass(distance)
        } else {
            self.mass
        }
    }

    /// Landing altitude: the height below which warp is restricted and on-rails is blocked.
    /// For atmospheric bodies, uses the atmosphere visible height.
    /// For airless bodies, uses 1% of body radius.
    pub fn landing_altitude(&self) -> f64 {
        match &self.atmosphere {
            Some(atmo) => atmo.visible_height(),
            None => self.radius * 0.01,
        }
    }

    /// Whether this body causes gravitational time dilation (black holes, neutron stars).
    /// True when Schwarzschild radius > 1% of body radius.
    pub fn is_compact(&self) -> bool {
        const C_SQ: f64 = 2.998e8 * 2.998e8;
        let r_s = 2.0 * G * self.mass / C_SQ;
        r_s / self.radius > 0.01
    }

    /// Surface rotational velocity at a given distance from body center (m/s)
    pub fn surface_velocity_at(&self, distance: f64) -> f64 {
        match self.sidereal_period {
            Some(period) if period > 0.0 => std::f64::consts::TAU * distance / period,
            _ => 0.0,
        }
    }
}

/// Keplerian orbital elements (simplified for 2D)
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Orbit {
    pub semi_major_axis: f64,     // meters
    pub eccentricity: f64,        // 0 = circle, <1 = ellipse
    pub argument_of_periapsis: f64, // radians
    pub mean_anomaly_at_epoch: f64, // radians at t=0
}

impl Orbit {
    /// Calculate mean anomaly at a given time
    pub fn mean_anomaly_at(&self, time: f64, parent_mass: f64) -> f64 {
        let mu = G * parent_mass;
        let n = (mu / self.semi_major_axis.powi(3)).sqrt(); // mean motion
        self.mean_anomaly_at_epoch + n * time
    }

    /// Solve Kepler's equation for eccentric/hyperbolic anomaly
    /// For elliptical orbits: M = E - e*sin(E)
    /// For hyperbolic orbits: M = e*sinh(H) - H
    pub fn solve_kepler(&self, mean_anomaly: f64) -> f64 {
        let e = self.eccentricity;

        if e < 1.0 {
            // Elliptical orbit
            let m = mean_anomaly.rem_euclid(std::f64::consts::TAU);
            let mut e_anomaly = m; // Initial guess

            // Newton-Raphson iteration
            for _ in 0..20 {
                let delta = (e_anomaly - e * e_anomaly.sin() - m)
                    / (1.0 - e * e_anomaly.cos());
                e_anomaly -= delta;
                if delta.abs() < 1e-12 {
                    break;
                }
            }
            e_anomaly
        } else {
            // Hyperbolic orbit
            let m = mean_anomaly;
            let mut h_anomaly = if m.abs() < 1.0 {
                m
            } else {
                m.signum() * (m.abs() / e).asinh()
            };

            // Newton-Raphson iteration
            for _ in 0..30 {
                let f = e * h_anomaly.sinh() - h_anomaly - m;
                let f_prime = e * h_anomaly.cosh() - 1.0;
                if f_prime.abs() < 1e-12 {
                    break;
                }
                let delta = f / f_prime;
                h_anomaly -= delta;
                if delta.abs() < 1e-12 {
                    break;
                }
            }
            h_anomaly
        }
    }

    /// Calculate true anomaly from eccentric/hyperbolic anomaly
    pub fn true_anomaly(&self, anomaly: f64) -> f64 {
        let e = self.eccentricity;

        if e < 1.0 {
            // Elliptical: tan(ν/2) = sqrt((1+e)/(1-e)) * tan(E/2)
            2.0 * ((1.0 + e).sqrt() * (anomaly / 2.0).tan())
                .atan2((1.0 - e).sqrt())
        } else {
            // Hyperbolic: tan(ν/2) = sqrt((e+1)/(e-1)) * tanh(H/2)
            let tanh_half = (anomaly / 2.0).tanh();
            let tan_half_nu = ((e + 1.0) / (e - 1.0)).sqrt() * tanh_half;
            2.0 * tan_half_nu.atan()
        }
    }

    /// Get position relative to parent at given time
    pub fn position_at(&self, time: f64, parent_mass: f64) -> [f64; 2] {
        let mean_anomaly = self.mean_anomaly_at(time, parent_mass);
        let anomaly = self.solve_kepler(mean_anomaly);
        let true_anomaly = self.true_anomaly(anomaly);

        // Distance from focus (different formula for ellipse vs hyperbola)
        let r = if self.eccentricity < 1.0 {
            self.semi_major_axis * (1.0 - self.eccentricity * anomaly.cos())
        } else {
            // For hyperbola: r = |a| * (e*cosh(H) - 1), and a is negative
            self.semi_major_axis.abs() * (self.eccentricity * anomaly.cosh() - 1.0)
        };

        // Position in orbital plane (rotated by argument of periapsis)
        let angle = true_anomaly + self.argument_of_periapsis;
        let x = r * angle.cos();
        let y = r * angle.sin();

        [x, y]
    }

    /// Get position relative to parent from a given mean anomaly
    pub fn position_from_mean_anomaly(&self, mean_anomaly: f64, parent_mass: f64) -> [f64; 2] {
        let _ = parent_mass; // Not needed for position calculation from mean anomaly
        let anomaly = self.solve_kepler(mean_anomaly);
        let true_anomaly = self.true_anomaly(anomaly);

        // Distance from focus (different formula for ellipse vs hyperbola)
        let r = if self.eccentricity < 1.0 {
            self.semi_major_axis * (1.0 - self.eccentricity * anomaly.cos())
        } else {
            // For hyperbola: r = |a| * (e*cosh(H) - 1)
            self.semi_major_axis.abs() * (self.eccentricity * anomaly.cosh() - 1.0)
        };

        // Position in orbital plane (rotated by argument of periapsis)
        let angle = true_anomaly + self.argument_of_periapsis;
        let x = r * angle.cos();
        let y = r * angle.sin();

        [x, y]
    }

    /// Get velocity relative to parent from a given mean anomaly
    pub fn velocity_from_mean_anomaly(&self, mean_anomaly: f64, parent_mass: f64) -> [f64; 2] {
        self.velocity_from_mean_anomaly_with_direction(mean_anomaly, parent_mass, false)
    }

    /// Get velocity relative to parent from a given mean anomaly, with direction flag
    /// retrograde = true means clockwise orbit (negative angular momentum)
    pub fn velocity_from_mean_anomaly_with_direction(&self, mean_anomaly: f64, parent_mass: f64, retrograde: bool) -> [f64; 2] {
        let mu = G * parent_mass;
        let anomaly = self.solve_kepler(mean_anomaly);
        let true_anomaly = self.true_anomaly(anomaly);
        let e = self.eccentricity;

        // Distance from focus (different formula for ellipse vs hyperbola)
        let r = if e < 1.0 {
            self.semi_major_axis * (1.0 - e * anomaly.cos())
        } else {
            // For hyperbola: r = |a| * (e*cosh(H) - 1)
            self.semi_major_axis.abs() * (e * anomaly.cosh() - 1.0)
        };

        // Velocity magnitude from vis-viva equation: v² = μ(2/r - 1/a)
        // This works for both elliptical (a > 0) and hyperbolic (a < 0) orbits
        let v_squared = mu * (2.0 / r - 1.0 / self.semi_major_axis);
        if v_squared <= 0.0 {
            return [0.0, 0.0]; // Invalid state
        }
        let v = v_squared.sqrt();

        // Flight path angle (angle between velocity and local horizontal)
        // tan(γ) = e*sin(ν) / (1 + e*cos(ν)) - works for both ellipse and hyperbola
        let nu = true_anomaly;
        let flight_path_angle = (e * nu.sin()).atan2(1.0 + e * nu.cos());

        // Velocity direction is perpendicular to radius + flight path angle
        // Radius angle in global frame
        let radius_angle = true_anomaly + self.argument_of_periapsis;
        // For prograde: velocity is 90 degrees ahead of radius, minus flight path angle
        // For retrograde: velocity is 90 degrees behind radius, minus flight path angle
        // The flight path angle is always subtracted because it represents radial velocity component
        let direction_sign = if retrograde { -1.0 } else { 1.0 };
        let velocity_angle = radius_angle + direction_sign * std::f64::consts::FRAC_PI_2 - flight_path_angle;

        [v * velocity_angle.cos(), v * velocity_angle.sin()]
    }

    /// Calculate mean motion (radians per second)
    pub fn mean_motion(&self, parent_mass: f64) -> f64 {
        let mu = G * parent_mass;
        (mu / self.semi_major_axis.powi(3)).sqrt()
    }
}

/// Gravitational constant
pub const G: f64 = 6.67430e-11; // m³/(kg·s²)

/// Physics scale factor for 1:1 real-scale solar system simulation
/// At PHYSICS_SCALE = 1.0, all values match real life exactly:
/// - Earth radius: 6,371 km
/// - Earth LEO velocity: ~7.8 km/s
/// - Moon distance: ~384,400 km
/// - Earth-Sun distance: ~150 million km (1 AU)
/// - All masses, radii, and orbital parameters are real values
pub const PHYSICS_SCALE: f64 = 1.0;

/// Compute enclosed galactic mass at distance r from the galactic center.
/// Uses a 4-component Milky Way mass model: SMBH + Hernquist bulge + exponential disk + NFW dark matter halo.
/// M(2.46e20 m) ≈ 1.784e41 kg, matching the Sun's galactic orbital dynamics.
pub fn galactic_enclosed_mass(r: f64) -> f64 {
    // Supermassive black hole (point mass)
    const M_BH: f64 = 8.26e36; // kg (~4.15 million solar masses)

    // Hernquist bulge: M_bulge(r) = M_b * r^2 / (r + a)^2
    const M_B: f64 = 2.0e40; // kg (total bulge mass)
    const A_BULGE: f64 = 7.0e18; // m (~740 ly, scale radius)
    let m_bulge = M_B * r * r / ((r + A_BULGE) * (r + A_BULGE));

    // Exponential disk: M_disk(r) = M_d * [1 - (1 + r/R_d) * exp(-r/R_d)]
    const M_D: f64 = 9.0e40; // kg (total disk mass)
    const R_D: f64 = 8.5e19; // m (~9000 ly, scale length)
    let x_d = r / R_D;
    let m_disk = M_D * (1.0 - (1.0 + x_d) * (-x_d).exp());

    // NFW dark matter halo: M_halo(r) = M_0 * [ln(1 + x) - x/(1 + x)], x = r/r_s
    const M_0: f64 = 1.555e42; // kg (halo normalization mass)
    const R_S: f64 = 5.7e20; // m (~60,000 ly, scale radius)
    let x = r / R_S;
    let m_halo = M_0 * ((1.0 + x).ln() - x / (1.0 + x));

    M_BH + m_bulge + m_disk + m_halo
}

/// Calculate sphere of influence radius
fn calculate_soi(semi_major_axis: f64, mass: f64, parent_mass: f64) -> f64 {
    semi_major_axis * (mass / parent_mass).powf(0.4)
}

// === RON loading types ===

#[derive(Deserialize)]
struct BodyDefinitionFile {
    bodies: Vec<BodyDefinition>,
}

#[derive(Deserialize)]
struct BodyDefinition {
    name: String,
    description: String,
    mass: f64,
    radius: f64,
    color: [f32; 4],
    parent: Option<String>,
    orbit: Option<OrbitDefinition>,
    atmosphere: Option<AtmosphereDefinition>,
    sidereal_period: Option<f64>,
    #[serde(default)]
    accretion_disc: Option<AccretionDiscDefinition>,
    #[serde(default)]
    galactic_mass_profile: bool,
}

#[derive(Deserialize)]
struct OrbitDefinition {
    semi_major_axis: f64,
    eccentricity: f64,
    argument_of_periapsis: f64,
    mean_anomaly_at_epoch: f64,
}

#[derive(Deserialize)]
struct AtmosphereDefinition {
    surface_pressure: f64,
    scale_height: f64,
    color: [f32; 3],
}

#[derive(Deserialize)]
struct AccretionDiscDefinition {
    inner_radius: f64,
    outer_radius: f64,
    color_inner: [f32; 3],
    color_outer: [f32; 3],
}

/// Solar system with all planets and major moons
pub struct SolarSystem {
    pub bodies: Vec<CelestialBody>,
    pub time: f64, // seconds
}

impl SolarSystem {
    pub fn new() -> Self {
        let system = match Self::load_from_file("data/bodies/solar_system.ron") {
            Ok(system) => {
                log::info!("Loaded {} bodies from solar_system.ron", system.bodies.len());
                system
            }
            Err(e) => {
                log::warn!("Failed to load solar_system.ron ({}), using hardcoded fallback", e);
                Self::new_hardcoded()
            }
        };
        system
    }

    fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)?;
        let file: BodyDefinitionFile = ron::from_str(&data)?;

        // First pass: build name → index map and collect scaled masses
        let mut name_to_index: HashMap<String, usize> = HashMap::new();
        for (i, def) in file.bodies.iter().enumerate() {
            name_to_index.insert(def.name.clone(), i);
        }

        // Second pass: build CelestialBody vec
        let mut bodies = Vec::with_capacity(file.bodies.len());
        for def in &file.bodies {
            let mass = def.mass * PHYSICS_SCALE * PHYSICS_SCALE;
            let radius = def.radius * PHYSICS_SCALE;

            let parent_idx = def.parent.as_ref().map(|name| {
                *name_to_index.get(name).unwrap_or_else(|| panic!("Unknown parent body: {}", name))
            });

            let orbit = def.orbit.as_ref().map(|o| Orbit {
                semi_major_axis: o.semi_major_axis * PHYSICS_SCALE,
                eccentricity: o.eccentricity,
                argument_of_periapsis: o.argument_of_periapsis,
                mean_anomaly_at_epoch: o.mean_anomaly_at_epoch,
            });

            let soi_radius = match (parent_idx, &orbit) {
                (Some(pi), Some(o)) => {
                    let parent_mass = if file.bodies[pi].galactic_mass_profile {
                        galactic_enclosed_mass(o.semi_major_axis)
                    } else {
                        file.bodies[pi].mass * PHYSICS_SCALE * PHYSICS_SCALE
                    };
                    let soi = calculate_soi(o.semi_major_axis, mass, parent_mass);
                    // Stars/black holes orbiting the galactic center have vastly oversized
                    // SOIs (~1 ly) that overlap neighbors; shrink to ~0.05 ly for gameplay.
                    if file.bodies[pi].galactic_mass_profile { soi / 20.0 } else { soi }
                }
                _ => f64::INFINITY,
            };

            let atmosphere = def.atmosphere.as_ref().map(|a| Atmosphere {
                surface_pressure: a.surface_pressure,
                scale_height: a.scale_height,
                color: a.color,
            });

            let accretion_disc = def.accretion_disc.as_ref().map(|d| AccretionDisc {
                inner_radius: d.inner_radius * PHYSICS_SCALE,
                outer_radius: d.outer_radius * PHYSICS_SCALE,
                color_inner: d.color_inner,
                color_outer: d.color_outer,
            });

            bodies.push(CelestialBody {
                name: def.name.clone(),
                description: def.description.clone(),
                mass,
                radius,
                color: def.color,
                parent: parent_idx,
                orbit,
                soi_radius,
                atmosphere,
                sidereal_period: def.sidereal_period,
                accretion_disc,
                galactic_mass_profile: def.galactic_mass_profile,
            });
        }

        Ok(Self { bodies, time: 0.0 })
    }

    fn new_hardcoded() -> Self {
        let mut bodies = Vec::new();

        // === SAGITTARIUS A* (index 0) ===
        // mass = actual SMBH mass; effective_mass_at(r) returns enclosed galactic mass
        let sgr_a_mass = 8.26e36 * PHYSICS_SCALE * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Sagittarius A*".to_string(),
            description: String::new(),
            mass: sgr_a_mass,
            radius: 1.2e10 * PHYSICS_SCALE,
            color: [0.0, 0.0, 0.0, 1.0],
            parent: None,
            orbit: None,
            soi_radius: f64::INFINITY,
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: Some(AccretionDisc {
                inner_radius: 1.2e10 * PHYSICS_SCALE,
                outer_radius: 3.6e11 * PHYSICS_SCALE,
                color_inner: [1.0, 0.6, 0.25],
                color_outer: [0.8, 0.08, 0.02],
            }),
            galactic_mass_profile: true,
        });

        // === SUN (index 1) ===
        let sun_mass = 1.989e30 * PHYSICS_SCALE * PHYSICS_SCALE;
        let sun_radius = 6.96e8 * PHYSICS_SCALE;
        // ~21,100 ly from galactic center (target position: -2000 ly x, -21000 ly y)
        let sun_sma = 1.996e20 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Sun".to_string(),
            description: String::new(),
            mass: sun_mass,
            radius: sun_radius,
            color: [1.0, 0.95, 0.3, 1.0],
            parent: Some(0),
            orbit: Some(Orbit {
                semi_major_axis: sun_sma,
                eccentricity: 0.07,
                argument_of_periapsis: 0.0,
                // ν=3π/2 → x=0, y≈-21,000 ly (Sun directly below Sgr A*)
                mean_anomaly_at_epoch: 4.8534,
            }),
            soi_radius: calculate_soi(sun_sma, sun_mass, galactic_enclosed_mass(sun_sma)) / 20.0,
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === MERCURY (index 2) ===
        let mercury_mass = 3.301e23 * PHYSICS_SCALE * PHYSICS_SCALE;
        let mercury_sma = 5.79e10 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Mercury".to_string(),
            description: String::new(),
            mass: mercury_mass,
            radius: 2.44e6 * PHYSICS_SCALE,
            color: [0.7, 0.7, 0.7, 1.0],
            parent: Some(1),
            orbit: Some(Orbit {
                semi_major_axis: mercury_sma,
                eccentricity: 0.2056,
                argument_of_periapsis: 0.508,
                mean_anomaly_at_epoch: 0.0,
            }),
            soi_radius: calculate_soi(mercury_sma, mercury_mass, sun_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === VENUS (index 3) ===
        let venus_mass = 4.867e24 * PHYSICS_SCALE * PHYSICS_SCALE;
        let venus_sma = 1.082e11 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Venus".to_string(),
            description: String::new(),
            mass: venus_mass,
            radius: 6.052e6 * PHYSICS_SCALE,
            color: [0.9, 0.85, 0.7, 1.0],
            parent: Some(1),
            orbit: Some(Orbit {
                semi_major_axis: venus_sma,
                eccentricity: 0.0068,
                argument_of_periapsis: 0.958,
                mean_anomaly_at_epoch: 0.9,
            }),
            soi_radius: calculate_soi(venus_sma, venus_mass, sun_mass),
            atmosphere: Some(Atmosphere {
                surface_pressure: 9.2e6,
                scale_height: 15_900.0,
                color: [0.90, 0.70, 0.30],
            }),
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === EARTH (index 4) ===
        let earth_mass = 5.972e24 * PHYSICS_SCALE * PHYSICS_SCALE;
        let earth_sma = 1.496e11 * PHYSICS_SCALE;
        let earth_idx = bodies.len();
        bodies.push(CelestialBody {
            name: "Earth".to_string(),
            description: String::new(),
            mass: earth_mass,
            radius: 6.371e6 * PHYSICS_SCALE,
            color: [0.15, 0.5, 0.15, 1.0],
            parent: Some(1),
            orbit: Some(Orbit {
                semi_major_axis: earth_sma,
                eccentricity: 0.0167,
                argument_of_periapsis: 1.796,
                mean_anomaly_at_epoch: 1.8,
            }),
            soi_radius: calculate_soi(earth_sma, earth_mass, sun_mass),
            atmosphere: Some(Atmosphere {
                surface_pressure: 101_325.0,
                scale_height: 8_500.0,
                color: [0.25, 0.55, 1.00],
            }),
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === MOON (index 5) ===
        let moon_mass = 7.342e22 * PHYSICS_SCALE * PHYSICS_SCALE;
        let moon_sma = 3.844e8 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Moon".to_string(),
            description: String::new(),
            mass: moon_mass,
            radius: 1.737e6 * PHYSICS_SCALE,
            color: [0.75, 0.75, 0.75, 1.0],
            parent: Some(earth_idx),
            orbit: Some(Orbit {
                semi_major_axis: moon_sma,
                eccentricity: 0.0549,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 0.0,
            }),
            soi_radius: calculate_soi(moon_sma, moon_mass, earth_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === MARS (index 6) ===
        let mars_mass = 6.417e23 * PHYSICS_SCALE * PHYSICS_SCALE;
        let mars_sma = 2.279e11 * PHYSICS_SCALE;
        let mars_idx = bodies.len();
        bodies.push(CelestialBody {
            name: "Mars".to_string(),
            description: String::new(),
            mass: mars_mass,
            radius: 3.39e6 * PHYSICS_SCALE,
            color: [0.8, 0.3, 0.2, 1.0],
            parent: Some(1),
            orbit: Some(Orbit {
                semi_major_axis: mars_sma,
                eccentricity: 0.0934,
                argument_of_periapsis: 5.0,
                mean_anomaly_at_epoch: 2.5,
            }),
            soi_radius: calculate_soi(mars_sma, mars_mass, sun_mass),
            atmosphere: Some(Atmosphere {
                surface_pressure: 636.0,
                scale_height: 11_100.0,
                color: [0.80, 0.55, 0.35],
            }),
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === PHOBOS (index 7) ===
        let phobos_mass = 1.066e16 * PHYSICS_SCALE * PHYSICS_SCALE;
        let phobos_sma = 9.376e6 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Phobos".to_string(),
            description: String::new(),
            mass: phobos_mass,
            radius: 1.127e4 * PHYSICS_SCALE,
            color: [0.5, 0.45, 0.4, 1.0],
            parent: Some(mars_idx),
            orbit: Some(Orbit {
                semi_major_axis: phobos_sma,
                eccentricity: 0.0151,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 0.0,
            }),
            soi_radius: calculate_soi(phobos_sma, phobos_mass, mars_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === DEIMOS (index 8) ===
        let deimos_mass = 1.476e15 * PHYSICS_SCALE * PHYSICS_SCALE;
        let deimos_sma = 2.346e7 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Deimos".to_string(),
            description: String::new(),
            mass: deimos_mass,
            radius: 6.2e3 * PHYSICS_SCALE,
            color: [0.55, 0.5, 0.45, 1.0],
            parent: Some(mars_idx),
            orbit: Some(Orbit {
                semi_major_axis: deimos_sma,
                eccentricity: 0.0002,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 1.0,
            }),
            soi_radius: calculate_soi(deimos_sma, deimos_mass, mars_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === JUPITER (index 9) ===
        let jupiter_mass = 1.898e27 * PHYSICS_SCALE * PHYSICS_SCALE;
        let jupiter_sma = 7.785e11 * PHYSICS_SCALE;
        let jupiter_idx = bodies.len();
        bodies.push(CelestialBody {
            name: "Jupiter".to_string(),
            description: String::new(),
            mass: jupiter_mass,
            radius: 6.991e7 * PHYSICS_SCALE,
            color: [0.8, 0.7, 0.5, 1.0],
            parent: Some(1),
            orbit: Some(Orbit {
                semi_major_axis: jupiter_sma,
                eccentricity: 0.0489,
                argument_of_periapsis: 4.78,
                mean_anomaly_at_epoch: 3.5,
            }),
            soi_radius: calculate_soi(jupiter_sma, jupiter_mass, sun_mass),
            atmosphere: Some(Atmosphere {
                surface_pressure: 200_000.0,
                scale_height: 27_000.0,
                color: [0.75, 0.60, 0.40],
            }),
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === IO (index 10) ===
        let io_mass = 8.932e22 * PHYSICS_SCALE * PHYSICS_SCALE;
        let io_sma = 4.218e8 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Io".to_string(),
            description: String::new(),
            mass: io_mass,
            radius: 1.822e6 * PHYSICS_SCALE,
            color: [0.9, 0.85, 0.3, 1.0],
            parent: Some(jupiter_idx),
            orbit: Some(Orbit {
                semi_major_axis: io_sma,
                eccentricity: 0.0041,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 0.0,
            }),
            soi_radius: calculate_soi(io_sma, io_mass, jupiter_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === EUROPA (index 11) ===
        let europa_mass = 4.800e22 * PHYSICS_SCALE * PHYSICS_SCALE;
        let europa_sma = 6.711e8 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Europa".to_string(),
            description: String::new(),
            mass: europa_mass,
            radius: 1.561e6 * PHYSICS_SCALE,
            color: [0.85, 0.8, 0.75, 1.0],
            parent: Some(jupiter_idx),
            orbit: Some(Orbit {
                semi_major_axis: europa_sma,
                eccentricity: 0.0094,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 1.5,
            }),
            soi_radius: calculate_soi(europa_sma, europa_mass, jupiter_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === GANYMEDE (index 12) ===
        let ganymede_mass = 1.482e23 * PHYSICS_SCALE * PHYSICS_SCALE;
        let ganymede_sma = 1.070e9 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Ganymede".to_string(),
            description: String::new(),
            mass: ganymede_mass,
            radius: 2.634e6 * PHYSICS_SCALE,
            color: [0.65, 0.6, 0.55, 1.0],
            parent: Some(jupiter_idx),
            orbit: Some(Orbit {
                semi_major_axis: ganymede_sma,
                eccentricity: 0.0013,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 3.0,
            }),
            soi_radius: calculate_soi(ganymede_sma, ganymede_mass, jupiter_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === CALLISTO (index 13) ===
        let callisto_mass = 1.076e23 * PHYSICS_SCALE * PHYSICS_SCALE;
        let callisto_sma = 1.883e9 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Callisto".to_string(),
            description: String::new(),
            mass: callisto_mass,
            radius: 2.410e6 * PHYSICS_SCALE,
            color: [0.45, 0.42, 0.4, 1.0],
            parent: Some(jupiter_idx),
            orbit: Some(Orbit {
                semi_major_axis: callisto_sma,
                eccentricity: 0.0074,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 4.5,
            }),
            soi_radius: calculate_soi(callisto_sma, callisto_mass, jupiter_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === SATURN (index 14) ===
        let saturn_mass = 5.683e26 * PHYSICS_SCALE * PHYSICS_SCALE;
        let saturn_sma = 1.432e12 * PHYSICS_SCALE;
        let saturn_idx = bodies.len();
        bodies.push(CelestialBody {
            name: "Saturn".to_string(),
            description: String::new(),
            mass: saturn_mass,
            radius: 5.823e7 * PHYSICS_SCALE,
            color: [0.9, 0.85, 0.6, 1.0],
            parent: Some(1),
            orbit: Some(Orbit {
                semi_major_axis: saturn_sma,
                eccentricity: 0.0565,
                argument_of_periapsis: 5.92,
                mean_anomaly_at_epoch: 5.0,
            }),
            soi_radius: calculate_soi(saturn_sma, saturn_mass, sun_mass),
            atmosphere: Some(Atmosphere {
                surface_pressure: 140_000.0,
                scale_height: 59_500.0,
                color: [0.85, 0.80, 0.50],
            }),
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === TITAN (index 15) ===
        let titan_mass = 1.345e23 * PHYSICS_SCALE * PHYSICS_SCALE;
        let titan_sma = 1.222e9 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Titan".to_string(),
            description: String::new(),
            mass: titan_mass,
            radius: 2.575e6 * PHYSICS_SCALE,
            color: [0.85, 0.7, 0.4, 1.0],
            parent: Some(saturn_idx),
            orbit: Some(Orbit {
                semi_major_axis: titan_sma,
                eccentricity: 0.0288,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 0.0,
            }),
            soi_radius: calculate_soi(titan_sma, titan_mass, saturn_mass),
            atmosphere: Some(Atmosphere {
                surface_pressure: 146_700.0,
                scale_height: 21_000.0,
                color: [0.85, 0.55, 0.20],
            }),
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === RHEA (index 16) ===
        let rhea_mass = 2.307e21 * PHYSICS_SCALE * PHYSICS_SCALE;
        let rhea_sma = 5.27e8 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Rhea".to_string(),
            description: String::new(),
            mass: rhea_mass,
            radius: 7.64e5 * PHYSICS_SCALE,
            color: [0.8, 0.8, 0.8, 1.0],
            parent: Some(saturn_idx),
            orbit: Some(Orbit {
                semi_major_axis: rhea_sma,
                eccentricity: 0.001,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 1.5,
            }),
            soi_radius: calculate_soi(rhea_sma, rhea_mass, saturn_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === IAPETUS (index 17) ===
        let iapetus_mass = 1.806e21 * PHYSICS_SCALE * PHYSICS_SCALE;
        let iapetus_sma = 3.56e9 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Iapetus".to_string(),
            description: String::new(),
            mass: iapetus_mass,
            radius: 7.36e5 * PHYSICS_SCALE,
            color: [0.6, 0.55, 0.5, 1.0],
            parent: Some(saturn_idx),
            orbit: Some(Orbit {
                semi_major_axis: iapetus_sma,
                eccentricity: 0.0283,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 3.0,
            }),
            soi_radius: calculate_soi(iapetus_sma, iapetus_mass, saturn_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === DIONE (index 18) ===
        let dione_mass = 1.095e21 * PHYSICS_SCALE * PHYSICS_SCALE;
        let dione_sma = 3.774e8 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Dione".to_string(),
            description: String::new(),
            mass: dione_mass,
            radius: 5.62e5 * PHYSICS_SCALE,
            color: [0.85, 0.85, 0.85, 1.0],
            parent: Some(saturn_idx),
            orbit: Some(Orbit {
                semi_major_axis: dione_sma,
                eccentricity: 0.0022,
                argument_of_periapsis: 0.0,
                mean_anomaly_at_epoch: 4.5,
            }),
            soi_radius: calculate_soi(dione_sma, dione_mass, saturn_mass),
            atmosphere: None,
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === URANUS (index 19) ===
        let uranus_mass = 8.681e25 * PHYSICS_SCALE * PHYSICS_SCALE;
        let uranus_sma = 2.867e12 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Uranus".to_string(),
            description: String::new(),
            mass: uranus_mass,
            radius: 2.536e7 * PHYSICS_SCALE,
            color: [0.6, 0.85, 0.9, 1.0],
            parent: Some(1),
            orbit: Some(Orbit {
                semi_major_axis: uranus_sma,
                eccentricity: 0.0457,
                argument_of_periapsis: 1.69,
                mean_anomaly_at_epoch: 0.5,
            }),
            soi_radius: calculate_soi(uranus_sma, uranus_mass, sun_mass),
            atmosphere: Some(Atmosphere {
                surface_pressure: 100_000.0,
                scale_height: 27_700.0,
                color: [0.50, 0.80, 0.90],
            }),
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        // === NEPTUNE (index 20) ===
        let neptune_mass = 1.024e26 * PHYSICS_SCALE * PHYSICS_SCALE;
        let neptune_sma = 4.515e12 * PHYSICS_SCALE;
        bodies.push(CelestialBody {
            name: "Neptune".to_string(),
            description: String::new(),
            mass: neptune_mass,
            radius: 2.462e7 * PHYSICS_SCALE,
            color: [0.3, 0.5, 0.9, 1.0],
            parent: Some(1),
            orbit: Some(Orbit {
                semi_major_axis: neptune_sma,
                eccentricity: 0.0113,
                argument_of_periapsis: 4.63,
                mean_anomaly_at_epoch: 4.0,
            }),
            soi_radius: calculate_soi(neptune_sma, neptune_mass, sun_mass),
            atmosphere: Some(Atmosphere {
                surface_pressure: 100_000.0,
                scale_height: 19_700.0,
                color: [0.20, 0.40, 0.90],
            }),
            sidereal_period: None,
            accretion_disc: None,
            galactic_mass_profile: false,
        });

        Self { bodies, time: 0.0 }
    }

    /// Get world position of a body at current time
    pub fn body_position(&self, index: usize) -> [f64; 2] {
        let body = &self.bodies[index];

        match (body.parent, &body.orbit) {
            (Some(parent_idx), Some(orbit)) => {
                // Get parent position
                let parent_pos = self.body_position(parent_idx);
                let parent_mass = self.bodies[parent_idx].effective_mass_at(orbit.semi_major_axis);

                // Get position relative to parent
                let rel_pos = orbit.position_at(self.time, parent_mass);

                [parent_pos[0] + rel_pos[0], parent_pos[1] + rel_pos[1]]
            }
            _ => [0.0, 0.0], // Root body at origin
        }
    }

    /// Advance time
    pub fn update(&mut self, dt: f64) {
        self.time += dt;
    }
}

impl Default for SolarSystem {
    fn default() -> Self {
        Self::new()
    }
}
