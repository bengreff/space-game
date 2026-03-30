use crate::bodies::{G, SECTOR_SIDE_LY, LIGHT_YEAR, galactic_enclosed_mass};
use super::prng::Rng64;
use super::density;
use super::star_color;
use super::{ProceduralStar, StarType, solve_kepler_nr};

/// Generate all stars for a sector, deterministically from the galaxy seed.
pub fn generate_sector(
    sector_x: u16,
    sector_y: u16,
    galaxy_seed: u64,
) -> Vec<ProceduralStar> {
    let count = density::sector_star_count(sector_x, sector_y, galaxy_seed);
    let origin = density::sector_origin_meters(sector_x, sector_y);
    let sector_meters = SECTOR_SIDE_LY * LIGHT_YEAR;

    let mut stars = Vec::with_capacity(count as usize);

    for i in 0..count {
        // Deterministic seed per star
        let star_seed = Rng64::mix(
            Rng64::mix(galaxy_seed, sector_x as u64 | ((sector_y as u64) << 16)),
            i as u64,
        );
        let mut rng = Rng64::new(star_seed);

        // Position: uniform within sector
        let px = origin[0] + rng.next_f64() * sector_meters;
        let py = origin[1] + rng.next_f64() * sector_meters;

        // Spectral type by initial mass function (IMF)
        // Weights: M=76%, K=12%, G=7%, F=3%, A=1.5%, B=0.4%, O=0.1%
        let spectral_roll = rng.next_f64() * 100.0;
        let (temp_lo, temp_hi, mass_lo, mass_hi) = if spectral_roll < 76.0 {
            // M: 2400-3700K, 0.08-0.45 solar masses
            (2400.0, 3700.0, 0.08, 0.45)
        } else if spectral_roll < 88.0 {
            // K: 3700-5200K, 0.45-0.8
            (3700.0, 5200.0, 0.45, 0.8)
        } else if spectral_roll < 95.0 {
            // G: 5200-6000K, 0.8-1.04
            (5200.0, 6000.0, 0.8, 1.04)
        } else if spectral_roll < 98.0 {
            // F: 6000-7500K, 1.04-1.4
            (6000.0, 7500.0, 1.04, 1.4)
        } else if spectral_roll < 99.5 {
            // A: 7500-10000K, 1.4-2.1
            (7500.0, 10000.0, 1.4, 2.1)
        } else if spectral_roll < 99.9 {
            // B: 10000-30000K, 2.1-16.0
            (10000.0, 30000.0, 2.1, 16.0)
        } else {
            // O: 30000-50000K, 16.0-90.0
            (30000.0, 50000.0, 16.0, 90.0)
        };

        let mut temperature = rng.range_f64(temp_lo, temp_hi) as f32;
        let mut mass_solar = rng.range_f64(mass_lo, mass_hi);

        // Determine spectral class from initial temperature range
        let spectral_class = if spectral_roll < 76.0 { 'M' }
            else if spectral_roll < 88.0 { 'K' }
            else if spectral_roll < 95.0 { 'G' }
            else if spectral_roll < 98.0 { 'F' }
            else if spectral_roll < 99.5 { 'A' }
            else if spectral_roll < 99.9 { 'B' }
            else { 'O' };

        // Secondary evolution roll: override to evolved stellar type
        // ~6% white dwarfs, ~2% red giants, ~0.01% supergiants, ~0.1% neutron stars
        let evolution_roll = rng.next_f64() * 100.0;
        let luminosity;
        let star_type;
        if evolution_roll < 6.0 {
            // White Dwarf: remnant core, hot but tiny surface area → very dim
            mass_solar = rng.range_f64(0.5, 0.8);
            temperature = rng.range_f64(4_000.0, 40_000.0) as f32;
            luminosity = rng.range_f64(0.0001, 0.01) as f32;
            star_type = StarType::WhiteDwarf;
        } else if evolution_roll < 8.0 {
            // Red Giant: evolved off main sequence, cool but enormous → very bright
            mass_solar = rng.range_f64(0.8, 8.0);
            temperature = rng.range_f64(3_000.0, 5_000.0) as f32;
            luminosity = rng.range_f64(50.0, 2_000.0) as f32;
            star_type = StarType::RedGiant;
        } else if evolution_roll < 8.01 {
            // Supergiant: massive evolved star, enormous luminosity
            mass_solar = rng.range_f64(10.0, 70.0);
            temperature = rng.range_f64(3_500.0, 30_000.0) as f32;
            luminosity = rng.range_f64(10_000.0, 500_000.0) as f32;
            star_type = StarType::Supergiant;
        } else if evolution_roll < 8.11 {
            // Neutron Star: ultra-dense remnant, extremely hot but tiny → nearly invisible
            mass_solar = rng.range_f64(1.4, 2.1);
            temperature = rng.range_f64(100_000.0, 1_000_000.0) as f32;
            luminosity = rng.range_f64(0.00001, 0.0001) as f32;
            star_type = StarType::NeutronStar;
        } else {
            // Main Sequence: L ∝ M^3.5
            luminosity = (mass_solar.powf(3.5)) as f32;
            star_type = StarType::MainSequence(spectral_class);
        }

        let mass_kg = mass_solar * 1.989e30; // solar mass in kg

        // Elliptical galactic orbital elements
        let galactic_r = (px * px + py * py).sqrt();
        let theta_0 = py.atan2(px);

        // Generate eccentricity from Rayleigh distribution with radius-dependent σ
        // σ(r) = 0.05 + 0.20·exp(−r/2kpc) + 0.03·(r/20kpc)
        const KPC: f64 = 3.086e19; // 1 kiloparsec in meters
        let sigma = 0.05 + 0.20 * (-galactic_r / (2.0 * KPC)).exp()
            + 0.03 * (galactic_r / (20.0 * KPC));
        let u = rng.next_f64().max(1e-10);
        let ecc = (sigma * (-2.0 * u.ln()).sqrt()).min(0.95) as f32;
        let e = ecc as f64;

        // Random mean anomaly at t=0
        let mean_anomaly_0 = rng.next_f64() * std::f64::consts::TAU;

        // Derive consistent orbital elements so position at t=0 = (px, py)
        // Solve Kepler for E₀ from M₀
        let big_e0 = solve_kepler_nr(mean_anomaly_0, e);
        // True anomaly at t=0
        let nu_0 = 2.0 * ((1.0 + e).sqrt() * (big_e0 / 2.0).sin())
            .atan2((1.0 - e).sqrt() * (big_e0 / 2.0).cos());
        // Argument of periapsis: angular position at t=0 = theta_0 = nu_0 + arg_peri
        let arg_periapsis = theta_0 - nu_0;
        // Semi-major axis: radial position at t=0 = galactic_r = a(1 − e·cos(E₀))
        let semi_major_axis = if e > 0.0 {
            galactic_r / (1.0 - e * big_e0.cos())
        } else {
            galactic_r
        };
        // Mean motion from enclosed mass at semi-major axis
        let mean_motion = if semi_major_axis > 0.0 {
            (G * galactic_enclosed_mass(semi_major_axis) / (semi_major_axis * semi_major_axis * semi_major_axis)).sqrt()
        } else {
            0.0
        };

        stars.push(ProceduralStar {
            pos: [px, py],
            semi_major_axis,
            mean_motion,
            mean_anomaly_0,
            eccentricity: ecc,
            arg_periapsis: arg_periapsis as f32,
            mass: mass_kg,
            temperature,
            luminosity,
            color: star_color::stellar_color(temperature),
            radius_m: StarType::radius_meters(luminosity, temperature),
            sector_index: i,
            flags: 0,
            star_type,
        });
    }

    stars
}
