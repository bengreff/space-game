# Feature: Orbital Mechanics

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Orbital elements struct | ✅ Done | `Orbit` in bodies.rs |
| Kepler equation solver | ✅ Done | Newton-Raphson, 20 iterations |
| Position from elements | ✅ Done | `Orbit::position_at()` |
| SOI calculation | ✅ Done | `calculate_soi()` in bodies.rs |
| Full solar system | ✅ Done | 20 bodies with 1/4 scale physics |
| 1/4 scale physics | ✅ Done | PHYSICS_SCALE = 0.25 for radii, masses (²), distances |
| Velocity from elements | ❌ TODO | Needed for vessel physics |
| State vector conversion | ❌ TODO | Needed for thrust/maneuvers |
| SOI transitions | ❌ TODO | Detection logic exists, transitions not |
| Numerical integration | ❌ TODO | RK4 for active physics |

## Physics Scale

The game uses 1/4 scale physics (similar to KSP's approach):

| Property | Scaling | Result |
|----------|---------|--------|
| Body radii | × 0.25 | 1/4 size |
| Body masses | × 0.0625 | 1/16 mass (maintains surface gravity) |
| Orbital distances | × 0.25 | 1/4 distance |
| Orbital velocity | × 0.5 | Half of real |
| Orbital periods | × 0.5 | Half of real |
| Delta-v to orbit | × 0.5 | ~4.7 km/s for Earth (vs ~9.4 km/s real) |

## Overview

The orbital mechanics system is the core physics simulation of the game. It determines how vessels move through space under gravitational influence.

## Approach: Patched Conics

We use the **patched conics** approximation:
- At any time, a vessel is influenced by exactly ONE celestial body (its current SOI)
- Within that SOI, the vessel follows a perfect Keplerian orbit
- When crossing SOI boundaries, we "patch" to the new body's reference frame

This differs from **N-body** physics where all bodies influence each other simultaneously.

## Key Concepts

### Sphere of Influence (SOI)

Each body has an SOI radius where its gravity dominates. Calculated as:

```
SOI = a × (m_body / m_parent)^(2/5)
```

Where `a` is the body's semi-major axis and `m` is mass.

### Orbital Elements

Six parameters define an orbit:
- **Semi-major axis (a)**: Size of orbit
- **Eccentricity (e)**: Shape (0 = circle, <1 = ellipse, 1 = parabola, >1 = hyperbola)
- **Argument of periapsis (ω)**: Rotation of orbit
- **Mean anomaly at epoch (M₀)**: Position in orbit at reference time
- **Epoch (t₀)**: Reference time

(In 2D, we don't need inclination or longitude of ascending node)

### State Vectors

Alternative orbit representation:
- **Position (r)**: [x, y] relative to body center
- **Velocity (v)**: [vx, vy] relative to body

Can convert between state vectors and orbital elements.

## Implementation

### On-Rails Propagation

For vessels not actively thrusting:

```rust
fn propagate_orbit(orbit: &OrbitalElements, body: &Body, dt: f64) -> DVec2 {
    let mu = G * body.mass;
    let n = (mu / orbit.semi_major_axis.powi(3)).sqrt();  // Mean motion
    let new_mean_anomaly = orbit.mean_anomaly + n * dt;

    let true_anomaly = solve_kepler(new_mean_anomaly, orbit.eccentricity);
    position_from_true_anomaly(orbit, true_anomaly)
}
```

### Active Physics

When vessel is thrusting or in atmosphere:

```rust
fn physics_tick(vessel: &mut Vessel, bodies: &Bodies, dt: f64) {
    let body = bodies.get(vessel.current_body);

    // Gravity
    let r = vessel.position.length();
    let gravity_mag = G * body.mass / (r * r);
    let gravity = -vessel.position.normalize() * gravity_mag;

    // Thrust
    let thrust = vessel.thrust_direction() * vessel.thrust_magnitude();

    // Drag (if in atmosphere)
    let drag = calculate_drag(vessel, body);

    // Integration (RK4)
    let acceleration = gravity + thrust / vessel.mass + drag;
    integrate_rk4(vessel, acceleration, dt);
}
```

### SOI Transition

```rust
fn check_transition(vessel: &Vessel, bodies: &Bodies) -> Option<BodyId> {
    let current = bodies.get(vessel.current_body);

    // Check if exiting current SOI
    if vessel.position.length() > current.soi_radius {
        return current.parent;
    }

    // Check if entering child SOI
    for child_id in current.children {
        let child = bodies.get(child_id);
        let child_pos = child.position_at(vessel.time);
        let distance = (vessel.position - child_pos).length();
        if distance < child.soi_radius {
            return Some(child_id);
        }
    }

    None
}

fn perform_transition(vessel: &mut Vessel, new_body: BodyId, bodies: &Bodies) {
    // Convert position/velocity to new reference frame
    let old_body = bodies.get(vessel.current_body);
    let new_body_ref = bodies.get(new_body);

    // Transform coordinates
    // ... (depends on whether entering child or exiting to parent)

    // Recompute orbital elements in new frame
    vessel.orbit = elements_from_state_vectors(vessel.position, vessel.velocity, new_body_ref);
    vessel.current_body = new_body;
}
```

## Time Warp Considerations

| Warp Level | Physics Mode | Notes |
|------------|--------------|-------|
| 1-4x | Active | Full physics simulation |
| 5-100x | Hybrid | Simplified physics |
| 100x+ | On-rails | Pure Keplerian propagation |

### Warp Constraints
- No warp in atmosphere
- Limited warp near SOI boundaries
- Warp breaks when thrust applied

## Edge Cases

### Hyperbolic Orbits
- Eccentricity > 1
- True anomaly limited to ±acos(-1/e)
- Need different rendering (hyperbola not ellipse)

### Very Eccentric Ellipses
- Kepler solver may need more iterations
- Numerical precision issues near periapsis

### SOI Boundary Precision
- Check transitions at small timesteps near boundaries
- May need to "search" for exact crossing time

## Testing

Key scenarios to test:
1. Circular orbit stability (should not drift over many orbits)
2. Hohmann transfer between circular orbits
3. SOI transition (Moon capture from Earth orbit)
4. Hyperbolic flyby
5. Very eccentric orbit (e = 0.99)
