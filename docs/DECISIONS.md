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

## DEC-008: 1/4 Scale Physics (KSP-Style)

**Date**: 2026-02-15
**Status**: Accepted

**Context**: Real-scale solar system requires ~9.4 km/s to reach Earth orbit, which is challenging for gameplay. KSP uses ~1/10 scale to reduce this to ~3.4 km/s.

**Decision**: Use true 1/4 scale physics with PHYSICS_SCALE = 0.25 in bodies.rs.

**Rationale**:
- Delta-v requirements are halved (~4.7 km/s to Earth orbit)
- Surface gravity remains ~1g (mass scaled by 1/16, radius by 1/4)
- Difficulty is between KSP (~3.4 km/s) and real life (~9.4 km/s)
- Orbital distances remain real (familiar solar system layout)
- Visual BODY_SCALE = 4.0 restores apparent body sizes

**Implementation**:
- Body radii: real_radius × 0.25
- Body masses: real_mass × 0.0625 (0.25²)
- Orbital semi-major axes: real_sma × 0.25
- Surface gravity: g = GM/r² → unchanged (both M and r² scale by 1/16)
- Orbital velocity: v = √(GM/r) → v × 0.5 (half of real)
- Orbital periods: T = 2π√(a³/GM) → T × 0.5 (half of real)
- Visual BODY_SCALE = 4.0 in main.rs for display

**Physics Results**:
| Property | Real | 1/4 Scale |
|----------|------|-----------|
| Earth LEO velocity | 7.8 km/s | 3.9 km/s |
| Earth escape velocity | 11.2 km/s | 5.6 km/s |
| Moon orbital period | 27.3 days | 13.6 days |
| Earth orbital period | 365 days | 182 days |

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
