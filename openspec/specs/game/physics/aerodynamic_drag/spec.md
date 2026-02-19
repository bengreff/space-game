# Aerodynamic Drag

Orientation-dependent atmospheric drag that decelerates the ship relative to the body's rotating atmosphere.

## Drag Force

### Requirement: Drag coefficient

The system SHALL use a constant drag coefficient `DRAG_COEFFICIENT = 0.4` (blunt body approximation).

### Requirement: Surface-relative airspeed

Airspeed SHALL be computed relative to the body's rotating surface:
1. Compute body surface velocity at ship distance: `v_surface = surface_velocity_at(distance)`
2. Surface velocity direction is tangential (perpendicular to radial, CCW): `tangent = [-radial_y, radial_x]`
3. Airspeed vector: `airspeed = rel_velocity - v_surface * tangent`

#### Scenario: Ship in LEO matching Earth rotation
- **WHEN** the ship orbits at ~7.8 km/s and Earth surface velocity is ~465 m/s at the equator
- **THEN** the surface-relative airspeed SHALL be approximately 7.3 km/s

### Requirement: Orientation-dependent cross-section

The drag cross-section SHALL interpolate between nose-on and broadside areas based on angle of attack:
1. `aoa = abs(sin(ship_rotation - velocity_angle))` (0 = nose-on, 1 = broadside)
2. `cross_section = width * 2 * (1 - aoa) + height * 2 * aoa`

Where `width` is `vessel_half_width` and `height` is `vessel_height` (bounding half-height) from `VesselPhysicsData`. Fallback values: width = `SHIP_SIZE/4`, height = `SHIP_SIZE/2`.

#### Scenario: Nose-first entry
- **WHEN** ship rotation matches velocity direction exactly (aoa = 0)
- **THEN** cross-section SHALL be `2 * vessel_half_width` (minimum area)

#### Scenario: Broadside entry
- **WHEN** ship rotation is perpendicular to velocity (aoa = 1)
- **THEN** cross-section SHALL be `2 * vessel_height` (maximum area)

### Requirement: Drag force calculation

Drag force SHALL be computed using the drag equation: `F_drag = 0.5 * density * airspeed^2 * Cd * cross_section`. Drag acceleration is `F_drag / total_mass_kg`, applied opposite to the airspeed direction.

### Requirement: Drag skip conditions

Drag computation SHALL return `[0.0, 0.0]` when:
- The SOI body has no atmosphere
- Altitude is negative or exceeds `visible_height()`
- Density is below `1e-12` kg/m^3
- Airspeed is below `0.01` m/s

### Requirement: Drag in Verlet integration

Drag acceleration SHALL be included in both half-steps of the Velocity Verlet integrator, using the same drag value for both steps (drag changes slowly per substep).

## Vessel Cross-Section

### Requirement: Vessel bounding half-width

`FlightVessel` SHALL provide a `bounding_half_width()` method that returns the maximum X-axis extent from center of mass across all non-decoupled, non-destroyed parts. Minimum return value SHALL be 0.5 meters.

### Requirement: Vessel half-width in physics data

`VesselPhysicsData` SHALL include a `vessel_half_width: f64` field, populated from `FlightVessel::bounding_half_width()`.

## Atmospheric Engine Performance

### Requirement: Engine ISP varies with atmospheric pressure

Engine ISP and thrust SHALL interpolate between vacuum and sea-level values based on local atmospheric pressure: `pressure_fraction = pressure_at_altitude(alt) / 101325.0`, clamped to [0.0, 1.0]. This fraction SHALL be passed to `consume_fuel()` instead of a hardcoded 0.0.
