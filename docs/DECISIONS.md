# Decision Log

Record significant technical and design decisions with rationale. This helps maintain context over long development periods and explains "why" to future-you and AI assistants.

---

## DEC-001: 2D Instead of 3D

**Date**: 2025-02-15
**Status**: Accepted

**Context**: Debated whether to build in 2D or 3D.

**Decision**: 2D only.

**Rationale**:
- 3D adds ~60% complexity (camera, attitude control, 3D terrain, quaternions)
- 2D proves all core mechanics work first
- Can always add 3D later as separate project
- Spaceflight Simulator proves 2D space games are commercially viable

**Trade-offs**:
- (-) Less immersive than 3D
- (-) Some mechanics simplified (no inclination changes)
- (+) Much faster development
- (+) Simpler to debug and visualize

---

## DEC-002: Rust Without Game Engine

**Date**: 2025-02-15
**Status**: Accepted

**Context**: Considered Bevy, Godot-Rust, Macroquad, or no engine.

**Decision**: No engine. Use wgpu + winit + egui directly.

**Rationale**:
- Full control over physics loop (critical for orbital mechanics)
- Bevy's ECS would fight custom orbital state management
- 2D rendering is simple enough without engine overhead
- Avoid engine update churn during development
- More learning, but applicable knowledge

**Alternatives Considered**:
- **Bevy**: Powerful but opinionated ECS conflicts with custom physics
- **Macroquad**: Simpler but less control, weaker ecosystem
- **Godot-Rust**: Extra complexity of two languages/systems

---

## DEC-003: Patched Conics Over N-Body

**Date**: 2025-02-15
**Status**: Accepted

**Context**: N-body physics is more realistic but significantly more complex.

**Decision**: Patched conics only.

**Rationale**:
- Predictable trajectories (essential for player planning)
- Much simpler to implement correctly
- KSP uses this approach successfully
- N-body causes chaotic orbits that frustrate players
- Lagrange points are interesting but niche

**Trade-offs**:
- (-) No Lagrange points
- (-) Unrealistic around binary systems
- (+) Trajectories are deterministic
- (+) Map view can show exact future path

---

## DEC-004: Crew as Integer Count

**Date**: 2025-02-15
**Status**: Accepted

**Context**: KSP has individual named Kerbals with skills and experience.

**Decision**: Track only crew count as integer, not individual crew members.

**Rationale**:
- Named crew is scope explosion (portraits, skills, experience, rescue missions)
- Integer count achieves core gameplay goals (crew capacity, life support mechanics)
- Can upgrade to named crew later if desired
- Life support math works the same either way

---

## DEC-005: Single Resource Type for Propellant

**Date**: 2025-02-15
**Status**: Proposed

**Context**: KSP has separate Fuel and Oxidizer resources that must be balanced.

**Decision**: Use single "Propellant" resource instead of Fuel + Oxidizer.

**Rationale**:
- Simpler for players to understand
- Removes frustrating "wrong ratio" scenarios
- Real spacecraft often use monopropellant or pre-mixed
- Can always split later if needed

**Trade-offs**:
- (-) Slightly less realistic
- (-) Removes some engineering challenge
- (+) Better UX
- (+) Simpler resource flow code

---

## DEC-006: Data-Driven Configuration

**Date**: 2025-02-15
**Status**: Accepted

**Context**: How should parts, bodies, and game data be defined?

**Decision**: All game data in RON files, loaded at startup. Code contains no part stats.

**Rationale**:
- Easy to tweak balance without recompiling
- Enables modding (eventually)
- Clear separation of data and logic
- RON is human-readable and Rust-native

**Implementation**:
- `data/parts/*.ron` - part definitions
- `data/bodies/*.ron` - celestial body definitions
- `data/resources.ron` - resource types
- `data/tech_tree.ron` - tech tree structure

---

## DEC-007: f64 for All Positions

**Date**: 2025-02-15
**Status**: Accepted

**Context**: What precision to use for positions?

**Decision**: All world positions use `f64` (double precision).

**Rationale**:
- Prevents floating-point precision loss at large distances
- Interstellar distances can be trillions of meters
- f32 loses precision past ~16km from origin
- f64 is accurate to millimeters at solar system scale

**Note**: Rendering positions can be converted to f32 relative to camera.

---

## DEC-008: Real-Scale Physics

**Date**: 2026-02-16
**Status**: Accepted (supersedes previous 1/4 scale decision)

**Context**: Initially considered 1/4 scale physics to reduce delta-v requirements, similar to KSP's approach.

**Decision**: Use real-scale physics with actual values for all celestial bodies.

**Rationale**:
- More educational and accurate simulation
- Simplified solar system (Sun, Earth, Moon only) reduces complexity
- Players can appreciate real orbital mechanics
- LEO velocity of ~7.8 km/s matches real-life data

**Implementation**:
- All body radii, masses, and orbital distances are real values
- Gravitational constant G = 6.674e-11 m³/(kg·s²)
- Simplified to Sun, Earth, Moon for Phase 1

---

## DEC-009: Camera-Relative Coordinate Rendering

**Date**: 2026-02-15
**Status**: Accepted

**Context**: Bodies far from origin (e.g., Mars at 228 billion meters) lost precision when rendered as f32 GPU coordinates.

**Decision**: Calculate positions relative to camera in f64 on CPU, then convert to f32 for GPU.

**Rationale**:
- f32 has only ~7 significant digits, losing precision past ~16km from origin
- By subtracting camera position in f64 first, relative positions stay small
- Small moons like Phobos (11km radius at 228 billion meters) render correctly

**Implementation**:
- Camera.position stored as [f64; 2]
- Vertex positions calculated as (world_pos - camera_pos) in f64, then cast to f32
- Shader receives camera-relative positions, no longer subtracts camera.position

---

## DEC-010: Triangle-Based Circle Rendering

**Date**: 2026-02-15
**Status**: Accepted

**Context**: Considered using lyon for 2D vector rendering.

**Decision**: Use triangle fans for filled circles, with dynamic segment count based on screen size.

**Rationale**:
- Simpler dependency chain (no lyon required)
- Full control over geometry generation
- Dynamic segments (64-4096) based on circumference in pixels
- 4x MSAA provides smooth edges

**Implementation**:
- Center vertex + edge vertices in triangle fan
- Segment count = circumference_pixels / 3, clamped to 64-4096
- 4x MSAA in render pipeline for antialiasing

---

## DEC-011: Conditional Orbit Line Visibility

**Date**: 2026-02-15
**Status**: Accepted

**Context**: Orbit lines clutter the view when zoomed in on a body.

**Decision**: Show orbit lines only when the body is small enough on screen.

**Rationale**:
- When zoomed in on a planet, its orbit is not useful context
- Moon orbits are useful when viewing a planetary system
- Planet orbits are useful when viewing the solar system

**Implementation**:
- Planet orbits (parent = Sun): visible when body < 5 pixels
- Moon orbits (parent ≠ Sun): visible when body < 100 pixels
- Calculated per frame based on current zoom level

---

## DEC-012: egui for UI Overlays

**Date**: 2026-02-15
**Status**: Accepted

**Context**: Need to display text labels for celestial bodies.

**Decision**: Use egui with egui-wgpu and egui-winit for UI overlays.

**Rationale**:
- Immediate mode UI integrates well with game loop
- egui-wgpu provides seamless wgpu integration
- Handles high-DPI displays via scale_factor
- Can be extended for HUD, menus, etc.

**Implementation**:
- egui rendered in separate pass after main geometry (no MSAA)
- Body labels shown on hover using painter.text()
- Screen coordinates converted from world via camera transform

---

## DEC-013: Velocity Verlet Integration

**Date**: 2026-02-16
**Status**: Accepted

**Context**: Need numerical integration for active ship physics (thrust applied).

**Decision**: Use Velocity Verlet integration instead of RK4.

**Rationale**:
- Symplectic integrator preserves orbital energy better
- Second-order accuracy with simple implementation
- No intermediate velocity storage needed
- Widely used in orbital mechanics simulations

**Implementation**:
- Position update: x += v*dt + 0.5*a*dt²
- New acceleration: a_new from gravity at new position
- Velocity update: v += 0.5*(a + a_new)*dt

---

## DEC-014: On-Rails Time Warp

**Date**: 2026-02-16
**Status**: Accepted

**Context**: High time warp (up to 1 billion x) causes accumulated integration error and performance issues.

**Decision**: Use on-rails propagation during time warp (>1x).

**Rationale**:
- Keplerian orbits are exact (no numerical error)
- O(1) computation regardless of warp factor
- Position calculated analytically from orbital elements + elapsed time
- Switch back to active physics when thrust applied

**Implementation**:
- Ship stores orbital elements when entering on-rails mode
- Position derived from: mean_anomaly = M0 + n*t, solve Kepler, get position
- SOI transitions detected analytically from orbit parameters
- Auto-reduce warp when approaching SOI boundary (< 0.5 seconds)

---

## DEC-015: Patched Conics Trajectory Prediction

**Date**: 2026-02-16
**Status**: Accepted

**Context**: Players need to see their future trajectory, including SOI transitions.

**Decision**: Implement patched conics trajectory prediction showing multiple segments across SOI boundaries.

**Rationale**:
- Shows complete trajectory through multiple SOIs
- Each segment has its own orbital elements
- Matches KSP's trajectory display approach
- Enables maneuver planning in future phases

**Implementation**:
- Predict current orbit in current SOI
- Find SOI entry/exit points analytically
- On SOI transition: convert frame, recalculate orbit
- Recursively predict future segments
- Render each segment in its parent body's frame

---

## Template for New Decisions

```markdown
## DEC-XXX: [Title]

**Date**: YYYY-MM-DD
**Status**: Proposed | Accepted | Superseded by DEC-XXX

**Context**: [What prompted this decision]

**Decision**: [What was decided]

**Rationale**: [Why this choice]

**Alternatives Considered**:
- [Alternative 1]: [Why rejected]
- [Alternative 2]: [Why rejected]

**Trade-offs**:
- (-) [Downside]
- (+) [Upside]
```
