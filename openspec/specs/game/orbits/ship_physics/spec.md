# Ship Physics

Ship state machine, thrust, rotation, Velocity Verlet integration, collision detection, and autopilot.

## State Machine

### Requirement: Ship state machine

The ship SHALL have two states in the `ShipState` enum:
- `Flying`: Active physics simulation
- `Landed { body_index, surface_angle }`: Resting on a body's surface at a specific angle

### Requirement: Ship position and velocity frame

Ship `rel_position` and `rel_velocity` SHALL be stored RELATIVE to the current SOI body (identified by `soi_body` index). All position/velocity values use f64.

### Requirement: Absolute position calculation

Absolute position SHALL be computed as `soi_body_position + rel_position`.

### Requirement: Absolute velocity calculation

Absolute velocity SHALL be computed as `soi_body_velocity + rel_velocity`.

### Requirement: Default spawn in LEO

When spawning on Earth (body index 3), the ship SHALL be placed in Low Earth Orbit at 400 km altitude (ISS orbit). Circular orbital velocity SHALL be `sqrt(G * earth_mass / orbital_radius)`. The spawn angle SHALL be `PI/2` (top of orbit), with velocity perpendicular to position (prograde direction).

## Throttle

### Requirement: Throttle control

The throttle SHALL be a value in the range [0.0, 1.0], clamped after every update. Throttle behavior:
- Throttle up: `throttle += THROTTLE_RATE * dt` where `THROTTLE_RATE = 0.25` per second
- Throttle down: `throttle -= THROTTLE_RATE * dt`
- Full throttle: instantly set to 1.0
- Zero throttle: instantly set to 0.0
- On-rails mode forces throttle to 0.0

## Rotation

### Requirement: Rotation with acceleration model

Ship rotation SHALL use an acceleration-based model:
- Reaction wheel acceleration: `rw_accel = torque / moment_of_inertia` (from vessel data), falling back to `ROTATION_ACCEL = 30 deg/s^2` (0.5236 rad/s^2)
- Gimbal acceleration: `gimbal_accel = gimbal_torque / moment_of_inertia` (always applied when non-zero, signed)
- Rotate left: `rotational_velocity += rw_accel * dt`
- Rotate right: `rotational_velocity -= rw_accel * dt`
- No input: apply rotation drag of `ROTATION_DRAG = 9 deg/s^2` (0.157 rad/s^2), reducing `rotational_velocity` toward zero
- Rotation angle updated: `rotation += rotational_velocity * dt`

#### Scenario: Rotation drag stops rotation
- **WHEN** no rotation input is active and `rotational_velocity` is positive but less than `ROTATION_DRAG * dt`
- **THEN** `rotational_velocity` SHALL be set to 0.0 (not go negative)

## Vessel Physics Data

### Requirement: Vessel physics data bridge

The `VesselPhysicsData` struct SHALL bridge vessel data into physics: `total_mass` (tonnes), `max_thrust_vac` (kN), `max_thrust_asl` (kN), `vessel_height` (meters), `bottom_extent` (meters), `moment_of_inertia`, `torque` (kN*m from reaction wheels), and `gimbal_torque` (kN*m, signed).

## Thrust

### Requirement: Thrust acceleration calculation

Thrust acceleration SHALL be computed as:
- With vessel data: `thrust_accel = throttle * max_thrust_vac / total_mass`
- Without vessel data (fallback): `thrust_accel = throttle * MAX_THRUST_ACCELERATION` where `MAX_THRUST_ACCELERATION = 20.0 m/s^2`
- Thrust direction follows ship rotation: `[cos(rotation) * mag, sin(rotation) * mag]`

## Integration

### Requirement: Velocity Verlet integration

Each physics substep SHALL use velocity Verlet integration:
1. Calculate gravity acceleration at current position: `a_grav = -G*M/r^2 * r_hat` (only if distance > body radius)
2. Total acceleration = gravity + thrust
3. Update position: `pos += vel*dt + 0.5*accel*dt^2`
4. Recalculate gravity at new position
5. Update velocity: `vel += 0.5*(accel_old + accel_new)*dt`

#### Scenario: No gravity inside body
- **WHEN** the ship's distance from the SOI body center is less than the body's radius
- **THEN** gravity acceleration SHALL be `[0.0, 0.0]`

### Requirement: Physics sub-stepping

The flying physics update SHALL divide the timestep into substeps:
- Maximum physics timestep: `MAX_PHYSICS_DT = 0.01` seconds
- Number of substeps: `ceil(dt / MAX_PHYSICS_DT)`, clamped to range [1, 1000]
- Each substep duration: `dt / num_steps`

### Requirement: Effective time for thrusting at high warp

When the ship is thrusting and time warp exceeds `RAILS_WARP_THRESHOLD`, the effective dt SHALL be reduced: `effective_dt = dt / time_warp * RAILS_WARP_THRESHOLD`. This caps physics simulation speed while allowing time warp UI to show faster rates.

## On-Rails Mode

### Requirement: On-rails mode entry criteria

The ship SHALL enter on-rails mode when ALL of the following are true:
- Time warp > `RAILS_WARP_THRESHOLD` (100.0)
- Ship state is `Flying`
- Throttle is exactly 0.0

#### Scenario: On-rails with throttle
- **WHEN** time warp is 1000x but throttle is 0.5
- **THEN** the ship SHALL NOT enter on-rails mode

### Requirement: On-rails mode propagation

While on-rails, the ship SHALL follow its cached Keplerian orbit exactly:
1. Advance mean anomaly: `M += direction * mean_motion * dt` (direction is -1 for retrograde, +1 for prograde)
2. Normalize mean anomaly to [0, 2pi)
3. Recompute `rel_position` and `rel_velocity` from the updated mean anomaly
4. Check for SOI transitions

### Requirement: On-rails entry and exit

When entering on-rails mode, the ship SHALL calculate and cache its current orbital elements. Only elliptical orbits (e < 1.0) can go on-rails; if orbit calculation fails, on-rails mode SHALL NOT be entered. When exiting on-rails mode, position and velocity SHALL be restored from the cached orbit at the current mean anomaly.

## Landing

### Requirement: Landed state behavior

While landed, the ship SHALL:
- Set rotation to the surface angle and rotational velocity to 0
- Position on the surface at `(body_radius + launchpad_offset + bottom_extent) * [cos(angle), sin(angle)]`
- If thrust acceleration exceeds surface gravity: transition to `Flying` with initial velocity `up_dir * net_accel * dt`
- If thrust is insufficient: remain landed with zero velocity

#### Scenario: Liftoff from surface
- **WHEN** the ship is landed and thrust acceleration > surface gravity
- **THEN** the ship state SHALL change to `Flying` and the ship SHALL have an initial upward velocity

#### Scenario: Insufficient thrust on surface
- **WHEN** the ship is landed and thrust acceleration <= surface gravity
- **THEN** the ship SHALL remain `Landed` with zero velocity

## Collision Detection

### Requirement: Collision detection with bodies

After each flying physics update, the ship SHALL check for collisions with ALL solar system bodies:
1. Compute distance from ship absolute position to body world position
2. If distance < body_radius + ship_radius (half vessel height or `SHIP_SIZE/2 = 5.0m` fallback):
   - Switch SOI body if needed
   - Place ship on surface at collision angle
   - Zero velocity and throttle
   - Set state to `Landed` with the collision surface angle
   - Disable on-rails mode and clear cached orbit
3. Launchpad collision takes priority (checked first for the launchpad body)

## Autopilot

### Requirement: Autopilot target modes

The autopilot SHALL support the following `AutopilotTarget` modes:
- `Off`: No autopilot (default)
- `Prograde`: Point in velocity direction
- `Retrograde`: Point opposite to velocity (prograde + PI)
- `RadialOut`: Point perpendicular to velocity (prograde + PI/2)
- `RadialIn`: Point perpendicular to velocity (prograde - PI/2)
- `ManeuverNode`: Point in maneuver node delta-v direction

#### Scenario: Prograde target angle
- **WHEN** autopilot is set to Prograde and velocity is `[100.0, 0.0]`
- **THEN** the target angle SHALL be `atan2(0.0, 100.0) = 0.0`

#### Scenario: Low velocity ignores autopilot
- **WHEN** autopilot is set to Prograde and velocity magnitude < 0.1 m/s
- **THEN** the target angle SHALL be `None`

### Requirement: Autopilot rotation controller

The autopilot rotation controller SHALL use acceleration-based control with stopping distance braking:
1. Normalize angle difference to [-PI, PI]
2. If `|angle_diff| < 0.002 rad` (~0.1 degrees) and `|rotational_velocity| < 0.01 rad/s`: snap to target, zero rotational velocity
3. Calculate stopping distance: `s = v^2 / (2 * accel)`
4. Brake when going the right direction AND stopping distance >= 50% of remaining angle
5. Otherwise accelerate toward target
