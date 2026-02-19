# Patched Conics

Trajectory prediction across SOI boundaries, SOI transition detection, and frame conversion.

## Configuration

### Requirement: Maximum patched conic segments

The system SHALL predict a maximum of `MAX_PATCHED_CONICS = 1` SOI change beyond the current orbit.

### Requirement: Trajectory caching

Patched conic trajectories SHALL be cached and reused for `TRAJECTORY_CACHE_FRAMES = 30` frames (~0.5 seconds at 60fps). Cache SHALL be invalidated when:
- Ship is thrusting (throttle > 0)
- SOI body has changed since last calculation
- Frame limit exceeded

### Requirement: Patched conic segment structure

Each `PatchedConicSegment` SHALL define: `orbit`, `parent_idx`, `retrograde`, `start_true_anomaly`, `end_true_anomaly` (None if orbit completes full loop), `start_time`, and `end_time` (None if no SOI exit).

## Hyperbolic SOI Exit

### Requirement: Hyperbolic orbit SOI exit prediction

For hyperbolic orbits, the exit true anomaly SHALL be calculated from the orbit equation:
1. Semi-latus rectum: `p = |a| * (e^2 - 1)`
2. `cos(nu_exit) = (p / soi_radius - 1) / e`
3. If `|cos(nu_exit)| <= 1`: `exit_ta = acos(cos(nu_exit))`; negate for retrograde
4. If `|cos(nu_exit)| > 1` (no intersection): use `exit_ta = acos(-1/e) - HYPERBOLIC_ANGLE_MARGIN`

### Requirement: Hyperbolic SOI exit frame conversion

After predicting a hyperbolic SOI exit, the system SHALL:
1. Calculate exit position and velocity from the orbit at the exit mean anomaly
2. Convert to parent frame via `convert_to_parent_frame()`
3. Calculate a new orbit in the parent body's frame
4. Add the new orbit as a second segment

## Elliptical SOI Intersection

### Requirement: Elliptical orbit SOI intersection detection

For elliptical orbits, SOI intersections SHALL be found by:
1. Early exit: if orbit does not escape SOI and no child bodies have overlapping orbital ranges, return `None`
2. Child body filtering: only check children whose orbital range (periapsis - soi_radius to apoapsis + soi_radius) overlaps the ship's orbital range (periapsis to apoapsis)
3. Analytical fast path for SOI exit (no children): compute exit true anomaly from `cos(nu) = (p/soi_radius - 1) / e` where `p = a*(1 - e^2)`

### Requirement: SOI intersection sampling

When child bodies exist, the system SHALL use coarse sampling with `SOI_INTERSECTION_SAMPLES = 200` samples over one full orbit period:
1. For each sample, compute ship position from mean anomaly
2. Check if ship distance from parent exceeds `soi_radius * SOI_EXIT_THRESHOLD` (0.99) for SOI exit
3. Check if ship distance from any child body is within `child_soi_radius * SOI_ENTRY_THRESHOLD` (1.01) for SOI entry
4. Skip samples beyond the current best intersection time
5. Minimum time threshold: `MIN_INTERSECTION_TIME = 0.01` seconds

### Requirement: SOI intersection binary search refinement

Child body SOI entry intersections SHALL be refined with binary search:
- `SOI_REFINE_ITERATIONS = 10` iterations
- Search window: 2x the sample spacing around the approximate intersection
- Both ship and child body positions are evaluated at the midpoint time
- Converges on the boundary crossing point

## SOI Transitions During Physics

### Requirement: SOI transition during physics simulation (precise)

During physics substeps, SOI transitions SHALL be detected by comparing distances before and after the substep:
- SOI exit: `curr_dist > soi_radius AND prev_dist <= soi_radius`
- SOI entry (child): `curr_dist_to_child < child_soi_radius AND prev_dist_to_child >= child_soi_radius`

### Requirement: Precise SOI crossing interpolation

When a SOI transition is detected during physics, the exact crossing fraction SHALL be found via binary search with `BINARY_SEARCH_ITERATIONS = 50` iterations:
1. For SOI exit: interpolate between previous and current relative positions, find where distance equals SOI radius
2. For SOI entry: interpolate both ship position and child body position (both moving), find where relative distance equals child SOI radius
3. At the crossing point: convert position/velocity to new reference frame
4. Propagate remaining timestep `(1 - fraction) * dt` in the new frame

## Frame Conversion

### Requirement: SOI exit frame conversion

When exiting to a parent SOI, the conversion SHALL:
1. Compute current SOI body position and velocity at crossing time
2. Add SOI body position/velocity to ship relative position/velocity: `new_pos = rel_pos + soi_body_pos`, `new_vel = rel_vel + soi_body_vel`
3. Set `soi_body` to the parent index

### Requirement: SOI entry frame conversion

When entering a child SOI, the conversion SHALL:
1. Compute child body position and velocity at crossing time
2. Subtract child body position/velocity from ship state: `new_pos = pos - child_pos`, `new_vel = vel - child_vel`
3. Set `soi_body` to the child index

### Requirement: SOI transition on-rails handling

When on-rails and a SOI transition is detected:
- SOI exit: add current body position/velocity to ship state, switch to parent, recalculate orbit; fall off rails if orbit calculation fails
- SOI entry: subtract child body position/velocity from ship state, switch to child, recalculate orbit; fall off rails if orbit calculation fails

## Maneuver Node Prediction

### Requirement: Predicted trajectory for maneuver nodes

The system SHALL support calculating a predicted trajectory from arbitrary state vectors (`pos`, `vel`, `parent_idx`) for maneuver node predictions. This uses the same patched conics logic: orbit calculation, hyperbolic exit detection, SOI intersection finding, and frame conversion, returning a `PatchedTrajectory` with up to two segments.

#### Scenario: Maneuver node predicts SOI escape
- **WHEN** a maneuver node creates a hyperbolic orbit relative to the current body
- **THEN** the predicted trajectory SHALL show the escape segment ending at the SOI boundary and a second segment in the parent body's frame

#### Scenario: Maneuver node predicts Moon encounter
- **WHEN** a maneuver node creates an orbit that intersects the Moon's SOI
- **THEN** the predicted trajectory SHALL show the current orbit ending at the Moon's SOI boundary and a second segment in the Moon's frame
