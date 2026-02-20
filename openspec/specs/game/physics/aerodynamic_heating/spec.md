# Aerodynamic Heating

Convective aerodynamic heating, radiative cooling, per-part thermal destruction, vessel splitting, heat shields, and heat visualization.

## Per-Part Temperature Model

### Requirement: Per-part thermal fields on PartDefinition

`PartDefinition` SHALL have the following fields with serde defaults:
- `max_heat_tolerance: f64` — default 2000.0 K
- `specific_heat: f64` — default 900.0 J/(kg*K)
- `emissivity: f64` — default 0.8
- `is_heat_shield: bool` — default false (for rendering dispatch)

### Requirement: Per-part thermal fields on FlightPart

`FlightPart` SHALL have:
- `temperature: f64` — initialized to 300.0 K in `from_blueprint()`
- `max_heat_tolerance: f64` — initialized from `PartDefinition.max_heat_tolerance`

### Requirement: Ship temperature field (fallback)

The `Ship` struct SHALL have a `temperature: f64` field (Kelvin) and a `heat_flux: f64` field (W/m^2). Initial/ambient temperature SHALL be `AMBIENT_TEMPERATURE = 300.0` K. This is used when no `FlightVessel` is present.

### Requirement: Temperature constants

The system SHALL define the following constants:
- `STEFAN_BOLTZMANN = 5.670374419e-8` W/(m^2*K^4)
- `HEAT_COEFFICIENT = 1.0e-4` (Sutton-Graves simplified coefficient)
- `VESSEL_DESTRUCTION_TEMP = 2000.0` K (ship-level fallback only)
- `AMBIENT_TEMPERATURE = 300.0` K

## Heat Shields

### Requirement: Heat shield part definitions

Four heat shield parts SHALL exist in `data/parts/aerodynamic.ron` under the `Aerodynamic` category:
- Tiny (HS-1): 1x0.5 grid, 0.02t, hitbox_height 1
- Small (HS-3): 3x0.5 grid, 0.08t, hitbox_height 1
- Medium (HS-5): 5x0.5 grid, 0.15t, hitbox_height 1
- Large (HS-9): 9x0.5 grid, 0.3t, hitbox_height 1

All heat shields SHALL have:
- `max_heat_tolerance: 4000.0` K
- `specific_heat: 1600.0` J/(kg*K)
- `emissivity: 0.95`
- `is_heat_shield: true`

### Requirement: Heat shield rendering

Heat shields SHALL be rendered with `generate_heat_shield_details()`:
- Near-black ablative face (bottom 60%) with colors `[0.05, 0.05, 0.05, 1.0]` — convex dome shape with curved bottom edge bulging downward (8 segments, sag = 30% of visual height)
- Dark backing structure (top 40%) with colors `[0.12, 0.12, 0.12, 1.0]` — flat rectangle
- Drawn on the upper half of the hitbox (shield_top = y + hitbox_half_h)
- Dispatched before decouplers in `generate_part_vertices()`, `generate_single_ghost_vertices()`, and `generate_part_shape_vertices()`

## Per-Part Heating

### Requirement: Aero environment extraction

`Ship::compute_aero_environment()` SHALL return `Option<(density, airspeed, airspeed_dir_world)>` — the atmospheric density, surface-relative airspeed, and airspeed direction vector. Returns `None` if not in atmosphere or density < 1e-15 or airspeed < 1.0.

### Requirement: Ship-level heating conditional on vessel

`Ship::update_temperature()` SHALL skip all processing when a `FlightVessel` exists (per-part system handles it). The ship-level model is kept as a fallback for no-vessel mode.

### Requirement: Per-part exposure calculation (1D interval occlusion)

`FlightVessel::update_part_temperatures()` SHALL:
1. Project each part onto the velocity axis (in vessel-local coordinates)
2. Sort parts by projection (most forward first)
3. For each part, compute the perpendicular cross-section interval [min, max]
4. Track a set of occluded perpendicular intervals (sorted, non-overlapping)
5. Calculate the exposed width as the part's interval minus any overlapping occluded intervals
6. After processing each part, add its interval to the occluded set

### Requirement: Per-part heat input

Heat input per part SHALL be: `q_in = HEAT_COEFFICIENT * sqrt(density) * airspeed^3 * exposed_area`, where `exposed_area = exposed_width * 0.5` (depth approximation using GRID_SQUARE_SIZE).

### Requirement: Per-part radiative cooling

Heat output per part SHALL be: `q_out = emissivity * STEFAN_BOLTZMANN * T^4 * surface_area`, where `surface_area = 2 * (width + height)` (perimeter approximation).

### Requirement: Per-part temperature update

Temperature update per part: `dT = (q_in - q_out) / (mass_kg * specific_heat) * dt`. Temperature SHALL be clamped to a minimum of 300.0 K.

### Requirement: No-atmosphere cooling

When density or airspeed is insufficient for heating, parts SHALL still undergo radiative cooling toward ambient temperature.

### Requirement: No heat conduction

Parts SHALL heat and cool independently — no heat conduction between parts.

## Thermal Destruction

### Requirement: Per-part destruction

`FlightVessel::destroy_overheated_parts()` SHALL destroy (set `destroyed = true`) any part whose `temperature >= max_heat_tolerance`. Destroyed part indices are returned for staging cleanup.

### Requirement: Staging cleanup on destruction

When parts are destroyed, their indices SHALL be removed from all staging lists.

### Requirement: Vessel splitting on part destruction

After parts are destroyed, `FlightVessel::check_and_split()` SHALL:
1. BFS from `root_part_index` through `find_weld_connections()` to find reachable parts
2. If root is destroyed, pick a new root (prefer pods, then any non-destroyed part)
3. Group unreachable (non-destroyed, non-decoupled) parts into connected components
4. Create debris `FlightVessel` for each component (reusing `extract_decoupled_parts` pattern)
5. Mark source parts as destroyed in the parent vessel
6. Return `Vec<(FlightVessel, [f64; 2])>` — debris vessels with COM offsets

### Requirement: Complete vessel destruction

If no non-destroyed, non-decoupled parts remain, the vessel SHALL be removed (`game.flight.vessel = None`) and ship temperature/heat_flux reset to ambient.

## Heat Visualization

### Requirement: Per-part heat fraction

`ShipPartRenderData` SHALL include `heat_fraction: f32` computed as `((temperature - 300) / (max_tolerance - 300)).clamp(0, 1)`.

### Requirement: Ship-level heat fraction from hottest part

`ShipRenderData.temperature` and `heat_fraction` SHALL reflect the hottest part's temperature when a vessel exists.

### Requirement: Per-part heat tinting on vertices

When rendering parts in flight, per-part `heat_fraction` SHALL be used for tinting (not ship-level). The tinting formula transitions from original color -> orange -> white:
- R: lerp toward 1.0
- G: lerp toward 0.6 (h < 0.5), then toward 1.0 (h >= 0.5)
- B: reduce toward 0 (h < 0.5), then increase toward 1.0 (h >= 0.5)
- Alpha unchanged

### Requirement: Heat tinting on ship triangle icon

The same heat tinting SHALL be applied to the ship's triangle indicator using the ship-level `heat_fraction` (hottest part) via `apply_heat_tint()`.

### Requirement: Heat bar in left HUD panel

When `temperature > 350K`, a vertical heat bar SHALL be shown (uses hottest part temperature):
- Colors: `< 0.33` -> yellow, `< 0.66` -> orange, `>= 0.66` -> red
- Background `rgb(40, 40, 50)`, border gray 1px

### Requirement: Temperature readout in bottom panel

When `heat_fraction > 0.01`, temperature readout ("{temp}K") SHALL appear in the bottom panel after altitude, colored by heat bar thresholds.
