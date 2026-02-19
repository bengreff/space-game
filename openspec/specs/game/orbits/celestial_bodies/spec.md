# Celestial Bodies

Solar system body definitions, gravitational constants, Keplerian orbital elements, and Kepler equation solvers.

## Constants

### Requirement: Gravitational constant

The system SHALL define the gravitational constant as `G = 6.67430e-11` m^3/(kg*s^2).

### Requirement: Physics scale factor

The system SHALL define `PHYSICS_SCALE = 1.0`, meaning all values match real life exactly. Body masses are scaled by `PHYSICS_SCALE^2`, radii and distances by `PHYSICS_SCALE`.

## Body Properties

### Requirement: Celestial body properties

Each `CelestialBody` SHALL have the following properties: `name` (String), `mass` (kg, f64), `radius` (meters, f64), `color` (RGBA [f32; 4]), `parent` (Optional index of parent body, None for root), `orbit` (Optional Keplerian orbital elements, None for root), `soi_radius` (meters, f64), and `atmosphere` (Optional atmospheric data).

### Requirement: Surface gravity calculation

Surface gravity SHALL be calculated as `g = G * mass / radius^2`.

### Requirement: Sphere of influence calculation

SOI radius SHALL be calculated using the formula `soi = semi_major_axis * (mass / parent_mass)^0.4`. The Sun's SOI radius SHALL be `f64::INFINITY`.

### Requirement: Atmosphere data

Bodies with atmospheres SHALL define `Atmosphere` with: `surface_pressure` (Pascals), `scale_height` (meters), and `color` (RGB [f32; 3]). The visible atmosphere height SHALL be `scale_height * 5.0` (~0.7% pressure level).

## Solar System

### Requirement: Solar system body hierarchy

The solar system SHALL contain the following bodies as a hierarchy indexed from 0:

| Index | Body      | Parent       | Mass (kg)    | Radius (m) | SMA (m)     | Eccentricity |
|-------|-----------|-------------|-------------|------------|-------------|-------------|
| 0     | Sun       | None (root) | 1.989e30    | 6.96e8     | N/A         | N/A         |
| 1     | Mercury   | Sun (0)     | 3.301e23    | 2.44e6     | 5.79e10     | 0.2056      |
| 2     | Venus     | Sun (0)     | 4.867e24    | 6.052e6    | 1.082e11    | 0.0068      |
| 3     | Earth     | Sun (0)     | 5.972e24    | 6.371e6    | 1.496e11    | 0.0167      |
| 4     | Moon      | Earth (3)   | 7.342e22    | 1.737e6    | 3.844e8     | 0.0549      |
| 5     | Mars      | Sun (0)     | 6.417e23    | 3.39e6     | 2.279e11    | 0.0934      |
| 6     | Phobos    | Mars (5)    | 1.066e16    | 1.127e4    | 9.376e6     | 0.0151      |
| 7     | Deimos    | Mars (5)    | 1.476e15    | 6.2e3      | 2.346e7     | 0.0002      |
| 8     | Jupiter   | Sun (0)     | 1.898e27    | 6.991e7    | 7.785e11    | 0.0489      |
| 9     | Io        | Jupiter (8) | 8.932e22    | 1.822e6    | 4.218e8     | 0.0041      |
| 10    | Europa    | Jupiter (8) | 4.800e22    | 1.561e6    | 6.711e8     | 0.0094      |
| 11    | Ganymede  | Jupiter (8) | 1.482e23    | 2.634e6    | 1.070e9     | 0.0013      |
| 12    | Callisto  | Jupiter (8) | 1.076e23    | 2.410e6    | 1.883e9     | 0.0074      |
| 13    | Saturn    | Sun (0)     | 5.683e26    | 5.823e7    | 1.432e12    | 0.0565      |
| 14    | Titan     | Saturn (13) | 1.345e23    | 2.575e6    | 1.222e9     | 0.0288      |
| 15    | Rhea      | Saturn (13) | 2.307e21    | 7.64e5     | 5.27e8      | 0.001       |
| 16    | Iapetus   | Saturn (13) | 1.806e21    | 7.36e5     | 3.56e9      | 0.0283      |
| 17    | Dione     | Saturn (13) | 1.095e21    | 5.62e5     | 3.774e8     | 0.0022      |
| 18    | Uranus    | Sun (0)     | 8.681e25    | 2.536e7    | 2.867e12    | 0.0457      |
| 19    | Neptune   | Sun (0)     | 1.024e26    | 2.462e7    | 4.515e12    | 0.0113      |

## Orbital Elements

### Requirement: Keplerian orbital elements (2D)

Each body's `Orbit` SHALL define: `semi_major_axis` (meters), `eccentricity` (0 = circle, <1 = ellipse, >=1 = hyperbola), `argument_of_periapsis` (radians), and `mean_anomaly_at_epoch` (radians at t=0).

### Requirement: Mean anomaly propagation

Mean anomaly at time `t` SHALL be calculated as `M = M0 + n * t`, where `n = sqrt(mu / a^3)` is the mean motion and `mu = G * parent_mass`.

## Kepler Equation Solvers

### Requirement: Kepler equation solver for elliptical orbits

For elliptical orbits (e < 1.0), the Kepler equation `M = E - e*sin(E)` SHALL be solved using Newton-Raphson iteration with:
- Mean anomaly normalized to [0, 2pi) via `rem_euclid(TAU)`
- Initial guess `E = M`
- Update: `E -= (E - e*sin(E) - M) / (1 - e*cos(E))`
- Maximum 20 iterations
- Convergence threshold `|delta| < 1e-12`

#### Scenario: Circular orbit Kepler solution
- **WHEN** eccentricity is 0.0 and mean anomaly is 1.5
- **THEN** the eccentric anomaly SHALL equal the mean anomaly (1.5)

#### Scenario: Eccentric orbit Kepler convergence
- **WHEN** eccentricity is 0.5 and mean anomaly is pi
- **THEN** the solver SHALL converge within 20 iterations to precision < 1e-12

### Requirement: Kepler equation solver for hyperbolic orbits

For hyperbolic orbits (e >= 1.0), the equation `M = e*sinh(H) - H` SHALL be solved using Newton-Raphson iteration with:
- Initial guess: `H = M` if `|M| < 1.0`, else `H = sign(M) * asinh(|M| / e)`
- Update: `H -= (e*sinh(H) - H - M) / (e*cosh(H) - 1)`
- Maximum 30 iterations
- Convergence threshold `|delta| < 1e-12`
- Early exit if `|f_prime| < 1e-12`

## Anomaly Conversions

### Requirement: True anomaly from eccentric anomaly (elliptical)

For elliptical orbits, true anomaly SHALL be computed as `nu = 2 * atan2(sqrt(1+e) * tan(E/2), sqrt(1-e))`.

### Requirement: True anomaly from hyperbolic anomaly

For hyperbolic orbits, true anomaly SHALL be computed as `nu = 2 * atan(sqrt((e+1)/(e-1)) * tanh(H/2))`.

## Position and Velocity

### Requirement: Position from orbital elements

Position relative to parent SHALL be calculated as:
1. Compute distance `r`:
   - Elliptical: `r = a * (1 - e*cos(E))`
   - Hyperbolic: `r = |a| * (e*cosh(H) - 1)`
2. Compute angle: `angle = true_anomaly + argument_of_periapsis`
3. Position: `[r*cos(angle), r*sin(angle)]`

### Requirement: Velocity from orbital elements using vis-viva

Velocity magnitude SHALL be calculated from the vis-viva equation: `v = sqrt(mu * (2/r - 1/a))`. This works for both elliptical (a > 0) and hyperbolic (a < 0) orbits. If `v^2 <= 0`, velocity SHALL be `[0, 0]`.

### Requirement: Velocity direction from flight path angle

Velocity direction SHALL be computed using the flight path angle:
1. `gamma = atan2(e*sin(nu), 1 + e*cos(nu))`
2. `radius_angle = true_anomaly + argument_of_periapsis`
3. For prograde: `velocity_angle = radius_angle + PI/2 - gamma`
4. For retrograde: `velocity_angle = radius_angle - PI/2 - gamma`
5. Velocity: `[v*cos(velocity_angle), v*sin(velocity_angle)]`

## Body Position Calculation

### Requirement: Body position calculation (recursive)

A body's world position SHALL be computed recursively: for a body with a parent, add the body's relative orbital position to its parent's world position. The root body (Sun) SHALL be at `[0.0, 0.0]`.

### Requirement: Body velocity calculation (recursive)

Body velocity in absolute coordinates SHALL be computed recursively:
1. For a body with a parent: compute velocity magnitude from vis-viva equation, direction perpendicular to radius vector
2. Add parent's absolute velocity (recursive call)
3. Root body velocity is `[0.0, 0.0]`

### Requirement: Body position/velocity at future time

The system SHALL support computing body position and velocity at any time `t` relative to the body's parent, using the orbital elements' `position_at(t)` and `velocity_from_mean_anomaly(M_at_t)` methods.

### Requirement: Solar system time advancement

The solar system SHALL maintain a `time` field (seconds, f64) that advances by `dt` on each `update()` call.
