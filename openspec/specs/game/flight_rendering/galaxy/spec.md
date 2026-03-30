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

## Star Classification

### Requirement: StarType enum

Each ProceduralStar stores a `StarType` enum classifying its evolutionary state:
- `MainSequence(char)` — spectral class O/B/A/F/G/K/M, determined from the spectral roll
- `WhiteDwarf` — from evolution roll < 6.0
- `RedGiant` — from evolution roll 6.0–8.0
- `Supergiant` — from evolution roll 8.0–8.01
- `NeutronStar` — from evolution roll 8.01–8.11

`display_name()` returns human-readable strings like "G-type Main Sequence", "White Dwarf", etc.

### Requirement: Physical radius calculation

Star physical radius is computed from luminosity and temperature via Stefan-Boltzmann law:
`R = R_sun * sqrt(L_solar) / (T / T_sun)^2`

This is used for real circle rendering when zoomed in close enough that the star subtends > 1 pixel.

## Rendering

### Requirement: Star visibility threshold

Stars become visible once the screen diagonal covers at least 0.1 light-years (~100 AU) and disappear at galaxy view scale. The galaxy view threshold scales with distance from the galactic center: 144 ly screen span near Sgr A* (dense region), linearly increasing to 640 ly at the Sun's distance (26,000 ly) and beyond. Stars are culled to rectangular screen bounds (no off-screen stars loaded). Sector lookup uses the screen half-diagonal plus a rotation margin, capped at 1,000 ly.

### Requirement: Sector star count cap

Each sector is capped at 2,000 stars (`MAX_STARS_PER_SECTOR`). This bounds galactic center sectors (which would otherwise have ~29,000 stars) to a manageable count while preserving visual density.

### Requirement: Distance-ordered sector iteration

Sectors are sorted by distance from the camera (closest first) before iterating. All on-screen stars from all searched sectors are collected, then if the total exceeds the cap, the closest stars by actual distance from the camera are kept via partial sort.

### Requirement: Star count cap

Total rendered stars are capped at 50,000 (`MAX_STARS`). When the cap is exceeded, stars are trimmed by distance from the camera center (closest kept), producing a smooth circular boundary rather than cutting off at sector edges.

### Requirement: Adaptive circle rendering

When a star's physical radius is large enough on screen (> 1 pixel), it is rendered as a real filled circle with adaptive segment count (`clamp(circumference/3, 16, 256)`) instead of the standard hexagon dot. This provides smooth visual scaling as the camera zooms in on individual stars.

### Requirement: Screen position storage for hit testing

During star rendering, each on-screen star's screen pixel coordinates are stored in `procedural_star_screen_positions`. These positions account for camera rotation and are used for hover detection and click handling.

## Interaction

### Requirement: Star hover detection

`update_star_hover()` in `interaction.rs` checks stored screen positions to find the closest star within a 20px screen radius. Body hover takes priority — star hover only activates when no body is hovered.

### Requirement: Star hover labels

When a star is hovered, its catalog name is displayed above the star in both flight mode and tracking station. Labels are rendered in a light blue color (200, 200, 255) to distinguish from body labels. Hover is cleared when the cursor moves onto an egui panel or leaves the window.

### Requirement: Star double-click focus

Double-clicking a procedural star in flight or tracking station:
1. Centers the camera on the star's world position (meters * SCALE)
2. Sets `focused_star` index and `focused_star_name` for tracking
3. Clears body/vessel tracking
4. The camera follows the star each frame via `update_tracking()` using `focused_star_world_pos`

The focused star is re-resolved by name each frame when the star list is rebuilt. If the star goes out of the visible set, the index is cleared (preventing stale data) but the world position is preserved for continued camera tracking.

### Requirement: Unified info panel (tracking station)

Both solar system bodies and procedural stars use the same `BodyInfoData` struct and the same right-side info panel in the tracking station. The panel is shown when a body is tracked OR a procedural star is focused (body tracking takes priority).

Panel sections:
- **Name** — light blue (200, 200, 255) for stars, white for non-stars
- **Description** — star type string (italic, light blue-grey) for stars, or body description for non-stars
- **Physical Properties** — Radius (R☉ + metric for stars, metric only for non-stars), surface gravity, mass (M☉ for stars, metric for non-stars)
- **Stellar Properties** (stars only) — luminosity (L☉), temperature (K), SOI
- **Atmosphere** (non-stars only) — pressure + height, or "No atmosphere"
- **Orbit** — labeled "Galactic Orbit" with pc/kpc formatting when `is_galactic_orbit` is true; otherwise standard metric formatting
- **SOI** (non-star bodies only)
- **Colony Prospects** (non-star bodies with resources/habitability)

### Requirement: BodyInfoData extended fields

BodyInfoData includes:
- `luminosity_solar: Option<f64>` — solar luminosities (stars only)
- `star_type: Option<String>` — e.g. "G-type Main Sequence" (stars only)
- `temperature_k: Option<f64>` — surface temperature in Kelvin (stars only)
- `soi_radius_m: Option<f64>` — SOI radius in meters (all bodies with finite SOI)
- `is_galactic_orbit: bool` — true for bodies/stars orbiting Sgr A* (orbit section uses pc/kpc)

For solar system stars: L ∝ M^3.5 luminosity relation, temperature from Stefan-Boltzmann law. Special cases: Sgr A* (supermassive black hole) has no luminosity/temperature; the Sun is hardcoded at 1.0 L☉, 5778 K.

### Requirement: Procedural star SOI

Procedural stars have SOIs computed as `calculate_soi(sma, mass_kg, galactic_enclosed_mass(sma)) / 20.0`, using the same 1/20 scaling factor as the Sun's SOI. The SOI is displayed in the stellar properties section of the info panel.

## Star Naming

### Requirement: Catalog naming scheme

Procedural stars are named `{PREFIX}-{sector_x:04}-{sector_y:04}-{sector_index:04}`, where PREFIX is the star type catalog prefix (e.g. "G" for G-type main sequence, "WD" for white dwarf, "RG" for red giant, "SG" for supergiant, "NS" for neutron star). Names are stable across frames for the same star and are used for cross-frame identity tracking of focused stars.

## Files

- `src/galaxy/mod.rs` — ProceduralStar struct, StarType enum, GalaxyState cache, solve_kepler_nr(), kepler_position()
- `src/galaxy/generation.rs` — Per-sector star generation with elliptical orbital elements and StarType classification
- `src/galaxy/density.rs` — Sector star count from galactic position (exponential disk + Gaussian bulge), capped at MAX_STARS_PER_SECTOR (2000)
- `src/main.rs` — `build_procedural_star_data()`: backward rotation, distance-ordered sectors, Kepler propagation, MAX_STARS cap, star metadata population; `lookup_focused_star()`: resolves focused star by name from galaxy cache when below visibility threshold; star double-click handling in flight and tracking station
- `src/render/scene.rs` — StarRenderData struct, adaptive dot/circle rendering, screen position storage
- `src/render/state.rs` — Star hover/focus state fields on RenderState
- `src/render/interaction.rs` — `update_star_hover()`, `star_at_screen_pos()`, star focus tracking in `update_tracking()`
- `src/render/menus.rs` — Star hover labels and info panel in tracking station
- `src/render/flight.rs` — Star hover labels in flight mode
- `src/render/types.rs` — BodyInfoData with luminosity_solar, star_type, temperature_k, soi_radius_m, is_galactic_orbit fields
- `src/bodies.rs` — `calculate_soi()` (public), `galactic_enclosed_mass()` used for star SOI computation
