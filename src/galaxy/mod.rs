pub mod prng;
pub mod density;
pub mod generation;
pub mod star_color;

use std::collections::HashMap;
use crate::bodies::SectorCoord;

/// A procedurally generated star. Lives only in the sector cache.
#[derive(Clone, Debug)]
pub struct ProceduralStar {
    pub pos: [f64; 2],          // world position at t=0 (meters, Sgr A* origin)
    pub semi_major_axis: f64,   // orbital semi-major axis around galactic center (meters)
    pub mean_motion: f64,       // mean motion n = sqrt(G * M_enclosed(a) / a^3) (rad/s)
    pub mean_anomaly_0: f64,    // mean anomaly at t=0 (radians)
    pub eccentricity: f32,      // orbital eccentricity (0..0.6)
    pub arg_periapsis: f32,     // argument of periapsis (radians)
    pub mass: f64,              // kg
    pub temperature: f32,       // Kelvin (determines render color)
    pub luminosity: f32,        // solar luminosities (determines dot size)
    pub color: [f32; 3],        // pre-computed RGB from temperature (cached)
    pub sector_index: u32,      // unique ID within sector
    pub flags: u32,             // bit 0: is_predefined, bit 1: has_system
}

/// Solve Kepler's equation M = E - e·sin(E) via Newton-Raphson.
/// Returns eccentric anomaly E in radians.
pub fn solve_kepler_nr(m: f64, e: f64) -> f64 {
    let mut big_e = m; // initial guess
    for _ in 0..10 {
        let sin_e = big_e.sin();
        let cos_e = big_e.cos();
        let f = big_e - e * sin_e - m;
        let f_prime = 1.0 - e * cos_e;
        big_e -= f / f_prime;
        if f.abs() < 1e-10 {
            break;
        }
    }
    big_e
}

/// Compute (x, y) position on an elliptical orbit given orbital elements.
/// - `a`: semi-major axis (meters)
/// - `e`: eccentricity
/// - `arg_peri`: argument of periapsis (radians)
/// - `mean_anomaly`: mean anomaly M at current time (radians)
///
/// Uses a first-order expansion for e < 0.1 (covers ~80%+ of stars),
/// avoiding Newton-Raphson entirely. Full NR for higher eccentricities.
pub fn kepler_position(a: f64, e: f64, arg_peri: f64, mean_anomaly: f64) -> [f64; 2] {
    if e < 0.1 {
        // First-order expansion: ν ≈ M + 2e·sin(M), r ≈ a·(1 − e·cos(M))
        // Error O(e²) ≈ 0.01 rad at e=0.1, negligible for rendering
        let sin_m = mean_anomaly.sin();
        let cos_m = mean_anomaly.cos();
        let nu = mean_anomaly + 2.0 * e * sin_m;
        let r = a * (1.0 - e * cos_m);
        let angle = nu + arg_peri;
        [r * angle.cos(), r * angle.sin()]
    } else {
        let big_e = solve_kepler_nr(mean_anomaly, e);
        let nu = 2.0 * ((1.0 + e).sqrt() * (big_e / 2.0).sin())
            .atan2((1.0 - e).sqrt() * (big_e / 2.0).cos());
        let r = a * (1.0 - e * big_e.cos());
        let angle = nu + arg_peri;
        [r * angle.cos(), r * angle.sin()]
    }
}

struct CachedSector {
    stars: Vec<ProceduralStar>,
    last_used_frame: u64,
}

/// Galaxy-wide procedural star system. Owned by Game.
/// Stars are generated per-sector from deterministic seeds and cached with LRU eviction.
pub struct GalaxyState {
    pub galaxy_seed: u64,
    cache: HashMap<SectorCoord, CachedSector>,
    frame_counter: u64,
}

/// Maximum number of sectors to keep cached before eviction.
const MAX_CACHED_SECTORS: usize = 200;
/// Sectors unused for this many frames get evicted.
const EVICTION_AGE_FRAMES: u64 = 300; // ~5 seconds at 60fps

impl GalaxyState {
    pub fn new() -> Self {
        Self {
            galaxy_seed: 0xDEAD_BEEF_CAFE_1234,
            cache: HashMap::new(),
            frame_counter: 0,
        }
    }

    /// Get stars for a sector, generating and caching if needed.
    /// Returns a slice of ProceduralStars.
    pub fn get_sector(&mut self, coord: SectorCoord) -> &[ProceduralStar] {
        let frame = self.frame_counter;
        let seed = self.galaxy_seed;

        self.cache
            .entry(coord)
            .and_modify(|cs| cs.last_used_frame = frame)
            .or_insert_with(|| CachedSector {
                stars: generation::generate_sector(coord.x, coord.y, seed),
                last_used_frame: frame,
            })
            .stars
            .as_slice()
    }

    /// Advance frame counter and evict stale sectors.
    pub fn tick(&mut self) {
        self.frame_counter += 1;

        // Only evict when over capacity
        if self.cache.len() > MAX_CACHED_SECTORS {
            let cutoff = self.frame_counter.saturating_sub(EVICTION_AGE_FRAMES);
            self.cache.retain(|_, cs| cs.last_used_frame >= cutoff);
        }
    }
}
