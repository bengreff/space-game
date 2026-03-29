use crate::bodies::{SECTOR_GRID_SIZE, SECTOR_SIDE_LY, LIGHT_YEAR};

/// Compute how many stars a sector should contain based on its galactic position.
///
/// Calibrated against observed Milky Way stellar densities:
/// - Solar neighborhood (R≈26,000 ly): ~4.3 ly average inter-star distance
///   (0.14 stars/pc³ midplane, projected into 2D → ~540 stars per sector)
/// - Galactic center: ~0.6 ly average distance (~29,000 stars/sector)
/// - Outskirts (R≈50,000 ly): ~18 ly average distance (~30 stars/sector)
///
/// Models:
/// - Exponential disk (scale length 8,500 ly, matching thin disk observations)
/// - Gaussian bulge (σ = 2,000 ly, half-light radius ~3,300 ly)
/// - Two logarithmic spiral arms (pitch angle 12.6°, ~1.67:1 contrast)
pub fn sector_star_count(sector_x: u16, sector_y: u16, galaxy_seed: u64) -> u32 {
    // Sector center in light-years relative to galactic center (Sgr A*)
    let cx_ly = (sector_x as f64 - SECTOR_GRID_SIZE as f64 / 2.0 + 0.5) * SECTOR_SIDE_LY;
    let cy_ly = (sector_y as f64 - SECTOR_GRID_SIZE as f64 / 2.0 + 0.5) * SECTOR_SIDE_LY;
    let r_ly = (cx_ly * cx_ly + cy_ly * cy_ly).sqrt();
    let theta = cy_ly.atan2(cx_ly);

    // --- Exponential disk (thin disk scale length: 2.6 kpc ≈ 8,500 ly) ---
    let disk_scale = 8_500.0;
    let disk = (-r_ly / disk_scale).exp();

    // --- Galactic bulge (Gaussian, σ = 2,000 ly) ---
    let bulge_sigma = 2_000.0;
    let bulge = 1.5 * (-r_ly * r_ly / (2.0 * bulge_sigma * bulge_sigma)).exp();

    // --- Spiral arms (2-arm logarithmic, pitch ≈ 12.6°) ---
    let arm_pitch: f64 = 0.22; // radians
    let arm_count = 2.0;
    let log_r = if r_ly > 100.0 { (r_ly / 3000.0).ln() } else { -3.0 };
    let spiral_phase = arm_count * (theta - log_r / arm_pitch.tan());
    let arm_strength = 0.5 * (1.0 + spiral_phase.cos());
    let arm_factor = 0.6 + 0.4 * arm_strength; // 0.6 inter-arm, 1.0 on-arm

    let density = (disk + bulge) * arm_factor;

    // Multiplier calibrated so R=26,000 ly gives ~540 stars/sector (avg 4.3 ly spacing).
    // At that radius: disk≈0.047, bulge≈0, arm_factor≈0.8 avg → density≈0.037.
    // 0.037 * 14,400 ≈ 540.
    let base_count = density * 14_400.0;

    // Deterministic jitter ±15%
    let hash = super::prng::Rng64::mix(galaxy_seed, sector_x as u64 | ((sector_y as u64) << 16));
    let variation = (hash & 0xFFFF) as f64 / 65536.0;
    let jitter = 0.85 + 0.30 * variation;

    const MAX_STARS_PER_SECTOR: u32 = 2000;
    let count = (base_count * jitter).round() as u32;
    count.max(1).min(MAX_STARS_PER_SECTOR)
}

/// Compute the world-space origin of a sector (bottom-left corner) in meters.
pub fn sector_origin_meters(sector_x: u16, sector_y: u16) -> [f64; 2] {
    let half = (SECTOR_GRID_SIZE as f64 / 2.0) * SECTOR_SIDE_LY * LIGHT_YEAR;
    [
        sector_x as f64 * SECTOR_SIDE_LY * LIGHT_YEAR - half,
        sector_y as f64 * SECTOR_SIDE_LY * LIGHT_YEAR - half,
    ]
}
