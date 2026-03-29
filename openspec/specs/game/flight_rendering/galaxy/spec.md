# Procedural Star Field

Procedural stars generated per-sector, cached with LRU eviction, and rendered as colored dots in galaxy view.

## Star Generation

### Requirement: Procedural stars generated per-sector

Stars are generated deterministically from a galaxy seed + sector coordinates. Each star has a position (t=0), spectral properties (temperature, luminosity, mass), and pre-computed render color. Stars are stored in a sector cache with LRU eviction (max 200 sectors, 300-frame age).

### Requirement: Evolved stellar types

After the main-sequence spectral type roll, a secondary "evolution" roll overrides ~8% of stars to evolved types:

| Roll Range | Type | Fraction | Mass (M☉) | Temperature (K) | Luminosity (L☉) |
|-----------|------|----------|-----------|-----------------|------------------|
| < 6.0 | White Dwarf | ~6% | 0.5–0.8 | 4,000–40,000 | 0.0001–0.01 |
| < 8.0 | Red Giant | ~2% | 0.8–8.0 | 3,000–5,000 | 50–2,000 |
| < 8.01 | Supergiant | ~0.01% | 10–70 | 3,500–30,000 | 10,000–500,000 |
| < 8.11 | Neutron Star | ~0.1% | 1.4–2.1 | 100,000–1,000,000 | 0.00001–0.0001 |
| ≥ 8.11 | Main Sequence | ~91.89% | (from spectral roll) | (from spectral roll) | M^3.5 |

For evolved types, mass, temperature, and luminosity are independently re-rolled within their ranges (overriding the main-sequence values). The existing `stellar_color()` function clamps temperature to [2000K, 40000K], so neutron stars render as deep blue. The log-scale luminosity renderer produces:
- Red giants: bright visible dots (L=50–2000)
- Supergiants: largest, brightest dots (L=10k–500k)
- White dwarfs: dim blue-white scatter (L=0.0001–0.01)
- Neutron stars: effectively invisible (L<0.0001)

### Requirement: Elliptical galactic orbital elements

Each ProceduralStar stores orbital elements for an elliptical orbit around the galactic center:
- `semi_major_axis` — orbital semi-major axis (meters), derived so radial position at t=0 matches the star's generated distance from galactic center
- `mean_motion` — angular rate n = sqrt(G * M_enclosed(a) / a^3) (rad/s)
- `mean_anomaly_0` — mean anomaly at t=0 (radians), randomly assigned
- `eccentricity` — orbital eccentricity (f32, range 0..0.6), generated from Rayleigh distribution with radius-dependent σ
- `arg_periapsis` — argument of periapsis (f32, radians), derived so angular position at t=0 matches the star's generated galactic angle

### Requirement: Eccentricity generation

Eccentricity is drawn from a Rayleigh distribution with σ varying by galactic radius:
- `σ(r) = 0.05 + 0.20·exp(−r/2kpc) + 0.03·(r/20kpc)` (1 kpc = 3.086e19 m)
- At r=0 (bulge): σ≈0.25, mean e≈0.31 (high radial oscillation)
- At r=8kpc (Sun): σ≈0.07, mean e≈0.09 (realistic for solar neighborhood)
- At r=30kpc (outer disk): σ≈0.10, mean e≈0.12
- `e = σ · sqrt(−2·ln(max(u, 1e-10)))`, capped at 0.95

### Requirement: Consistent element derivation

Orbital elements are derived so that the star's position at t=0 exactly matches its generated (px, py):
1. Random `mean_anomaly_0` ∈ [0, 2π)
2. Solve Kepler's equation for E₀
3. True anomaly: `ν₀ = 2·atan2(sqrt(1+e)·sin(E₀/2), sqrt(1-e)·cos(E₀/2))`
4. `arg_periapsis = theta_0 − ν₀` (preserves angular position)
5. `semi_major_axis = galactic_r / (1 − e·cos(E₀))` (preserves radial distance)
6. `mean_motion = sqrt(G · M_enclosed(a) / a³)`

## Star Propagation

### Requirement: Elliptical Keplerian position at game time

Each star's current position is computed by solving Kepler's equation:
1. `M(t) = mean_anomaly_0 + mean_motion * game_time`
2. Solve M = E − e·sin(E) via Newton-Raphson (~10 iterations, tolerance 1e-10)
3. True anomaly: `ν = 2·atan2(sqrt(1+e)·sin(E/2), sqrt(1-e)·cos(E/2))`
4. Radial distance: `r = a·(1 − e·cos(E))`
5. Position: `(r·cos(ν + ω_peri), r·sin(ν + ω_peri))`

Stars co-rotate with the Sun at similar galactic radii (differential rotation at different radii). Eccentricity adds realistic radial oscillation — stars drift inward and outward over their orbital period.

### Requirement: Backward rotation for sector lookup

Stars are cached at t=0 positions in sectors. To find stars currently near the camera:
1. Compute camera's galactic radius and angular velocity: `omega_cam = sqrt(G * M_enclosed(r_cam) / r_cam) / r_cam`
2. Rotate camera backward: `theta_0_cam = theta_cam - omega_cam * game_time`
3. Query sectors around the backward-rotated position `(r_cam * cos(theta_0_cam), r_cam * sin(theta_0_cam))`
4. Sector margin = `max(1 sector, 20% of render_radius)` to handle differential rotation and radial drift from eccentricity

#### Scenario: Zero game time
- **WHEN** `game_time == 0`
- **THEN** no backward rotation is applied; lookup center equals camera center; star positions match their generated pos exactly

#### Scenario: Co-rotating stars
- **WHEN** stars are at similar galactic radii to the camera
- **THEN** they have similar mean motion values and appear nearly stationary relative to the camera

#### Scenario: Differential rotation
- **WHEN** stars are at different galactic radii
- **THEN** they rotate at different angular velocities, producing galactic shear

#### Scenario: Radial oscillation
- **WHEN** time is advanced significantly (high warp)
- **THEN** stars with nonzero eccentricity show slight radial drift from their initial circular-like positions

## Kepler Helpers

### Requirement: solve_kepler_nr(M, e) -> E

Newton-Raphson solver for Kepler's equation M = E − e·sin(E). Initial guess E = M, up to 10 iterations, tolerance 1e-10. All procedural stars have e < 0.6, so convergence is fast.

### Requirement: kepler_position(a, e, arg_peri, mean_anomaly) -> [x, y]

Full pipeline: solve Kepler → true anomaly → r = a(1 − e·cos(E)) → angle = ν + arg_peri → (x, y). Used by both generation (element derivation) and propagation.

## Rendering

### Requirement: Star visibility threshold

Stars become visible once the screen diagonal covers at least 0.1 light-years (~100 AU) and disappear at galaxy view scale (screen spans 500+ light-years). A fixed render radius of 500 light-years is used regardless of zoom level — this prevents stars from gradually appearing as the camera zooms out and ensures the star set is constant across zoom levels.

### Requirement: Sector star count cap

Each sector is capped at 2,000 stars (`MAX_STARS_PER_SECTOR`). This bounds galactic center sectors (which would otherwise have ~29,000 stars) to a manageable count while preserving visual density.

### Requirement: Distance-ordered sector iteration

Sectors are sorted by distance from the camera (closest first) before iterating. This ensures nearby stars are always collected before distant ones. Collection stops at 50,000 stars (`MAX_STARS`), so in dense regions the closest stars are preferentially kept rather than bailing to empty.

### Requirement: Star count cap

Total rendered stars are capped at 50,000 (`MAX_STARS`). Unlike the previous bail-to-empty approach, reaching the cap simply stops collecting further stars — the closest stars (from distance-ordered sectors) are always rendered.

## Files

- `src/galaxy/mod.rs` — ProceduralStar struct, GalaxyState cache, solve_kepler_nr(), kepler_position()
- `src/galaxy/generation.rs` — Per-sector star generation with elliptical orbital elements
- `src/galaxy/density.rs` — Sector star count from galactic position, capped at MAX_STARS_PER_SECTOR (2000)
- `src/main.rs` — `build_procedural_star_data()`: backward rotation, distance-ordered sectors, Kepler propagation, MAX_STARS cap
- `src/render/scene.rs` — StarRenderData struct, dot rendering
