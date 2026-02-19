# Sunscatter

A 2D space game (KSP-like) with 1:1 real-scale orbital mechanics. Build rockets in the editor, launch from Earth, and fly using patched-conic trajectory prediction.

## Workflow Orchestration

### 1. Plan Mode Default
- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- If something goes sideways, STOP and re-plan immediately - don't keep pushing
- Use plan mode for verification steps, not just building
- Write detailed specs upfront to reduce ambiguity

### 2. Subagent Strategy to keep main context window clean
- Offload research, exploration, and parallel analysis to subagents
- For complex problems, throw more compute at it via subagents
- One task per subagent for focused execution

### 3. Self-Improvement Loop
- After ANY correction from the user: update 'tasks/lessons.md' with the pattern
- Write rules for yourself that prevent the same mistake
- Ruthlessly iterate on these lessons until mistake rate drops
- Review lessons at session start for relevant project

### 4. Verification Before Done
- Never mark a task complete without proving it works
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Run tests, check logs, demonstrate correctness

### 5. Demand Elegance (Balanced)
- For non-trivial changes: pause and ask "is there a more elegant way?"
- If a fix feels hacky: "Knowing everything I know now, implement the elegant solution"
- Skip this for simple, obvious fixes - don't over-engineer
- Challenge your own work before presenting it

### 6. Autonomous Bug Fixing
- When given a bug report: just fix it. Don't ask for hand-holding
- Point at logs, errors, failing tests -> then resolve them
- Zero context switching required from the user
- Go fix failing CI tests without being told how

## Spec Management
- **All changes must update specs**: Every code change should have corresponding requirements in an `openspec/specs/` spec file. If a spec doesn't exist yet for the area being changed, add requirements to the closest existing spec.
- **Merge into existing specs**: When syncing delta specs from an opsx change (design/proposal), merge new requirements into the pre-existing spec files under `openspec/specs/game/`. Only create a new spec file/folder if the change introduces something entirely new that doesn't fit any existing spec.
- **Spec structure**: `openspec/specs/game/<domain>/<capability>/spec.md`. Domains: `editor`, `editor_rendering`, `flight_rendering`, `orbits`. See existing files for the pattern.

## Task Management
1. **Plan First**: Write plan to 'tasks/todo.md' with checkable items
2. **Verify Plan**: Check in before starting implementation
3. **Track Progress**: Mark items complete as you go
4. **Explain Changes**: High-level summary at each step
5. **Document Results**: Add review to 'tasks/todo.md'
6. **Capture Lessons**: Update 'tasks/lessons.md' after corrections

## Core Principles
- **Simplicity First**: Make every change as simple as possible. Impact minimal code.
- **No Laziness**: Find root causes. No temporary fixes. Senior developer standards.
- **Minimal Impact**: Changes should only touch what's necessary. Avoid introducing bugs.

## Tech Stack

- **Language**: Rust (edition 2021)
- **Rendering**: wgpu 0.19 (triangle fans, 4x MSAA, custom vertex shaders)
- **Windowing**: winit 0.29
- **UI**: egui 0.27 (immediate mode, via egui-wgpu/egui-winit)
- **Serialization**: serde + ron 0.8
- **No game engine** — custom physics loop, wgpu rendering, egui UI

## Game Modes

Two modes, toggled via `GameMode` enum in `src/game.rs`:
- **Editor** (`GameMode::Editor`): Build vessels on a grid, save/load blueprints
- **Flight** (`GameMode::Flight`): Fly vessels with orbital mechanics, staging, maneuver nodes

## Project Structure

```
src/
├── main.rs              # Entry point, event loop, render orchestration (1086 lines)
│                        # render_flight_frame(), render_editor_frame()
│                        # Input handling, flight/editor mouse+keyboard dispatch
├── lib.rs               # Module re-exports
├── game.rs              # Game struct (central state), GameMode, FlightState
│                        # launch_from_editor(), save/load blueprint coordination
├── bodies.rs            # CelestialBody, Orbit, SolarSystem (hardcoded Sun/Earth/Moon/Mars)
│                        # Kepler equation solver, orbital position/velocity from time
│
├── ship/                # Ship physics (the point-mass that flies)
│   ├── mod.rs           # Ship struct, Velocity Verlet integration, thrust/rotation
│   │                    # ShipState enum (Flying/Landed), AutopilotTarget
│   │                    # VesselPhysicsData (bridges FlightVessel -> Ship physics)
│   ├── orbit.rs         # State vectors <-> orbital elements conversion
│   ├── patched_conics.rs # Trajectory prediction across SOI boundaries
│   └── soi.rs           # SOI transition detection, frame conversion, on-rails mode
│
├── parts/               # Part definition and vessel systems
│   ├── definition.rs    # PartDefinition, PartCategory, PartShape, PartSize
│   │                    # EngineData, TankData, PodData, DecouplerData
│   │                    # Propellant enum (Kerolox/Methalox/Hydrolox), FuelType
│   │                    # PartDefinitions registry (loads from RON)
│   ├── blueprint.rs     # VesselBlueprint, PlacedPart (serializable vessel design)
│   │                    # blueprint_to_parts / parts_to_blueprint conversion
│   ├── registry.rs      # BlueprintRegistry (save/load from data/blueprints/)
│   └── vessel.rs        # FlightVessel, FlightPart (runtime vessel with physics)
│                        # from_blueprint(), fuel consumption, staging activation
│                        # calculate_delta_v(), calculate_stage_delta_v()
│                        # Terrain/launchpad collision detection
│
├── editor/              # Vehicle editor
│   ├── state.rs         # EditorState (parts HashMap, stages, camera, ghost preview)
│   │                    # Part placement, dragging, deletion, stats calculation
│   │                    # calculate_stage_delta_v() for editor parts
│   ├── ui.rs            # egui UI: toolbar, stats bar, parts palette, staging panel
│   │                    # Part info panel, save/load dialogs
│   │                    # Drag-and-drop staging (StagingDrag enum)
│   └── render.rs        # Vertex generation for editor scene
│                        # Grid, placed parts, ghost preview, engine/pod/decoupler details
│                        # Procedural rendering (engine chambers, pod windows, fairings)
│                        # screen_to_world(), world_to_screen(), part_at_screen_pos()
│
└── render/              # Flight rendering and HUD
    ├── camera.rs        # Camera struct (position, zoom, body tracking)
    ├── geometry.rs      # create_circle(), create_ring(), create_ship_triangle()
    ├── maneuver.rs      # Maneuver node management (create/delete/drag/burn)
    ├── types.rs         # ShipRenderData, ShipPartRenderData, StagedPartInfo
    │                    # ManeuverNode, ManeuverDeltaV, OrbitRenderData
    │                    # Vertex struct (position + color, bytemuck)
    └── state.rs         # RenderState: wgpu setup, MSAA pipeline (3649 lines - largest file)
                         # Flight HUD (egui): velocity, altitude, orbit info, fuel, staging
                         # Body/orbit/ship rendering, maneuver node UI
                         # Flight staging panel with drag-and-drop (FlightStageDrag)
                         # render_editor() and render_flight() entry points
```

```
data/
├── parts/
│   ├── engines.ron      # 16 engines (Tiny to Large, Kerolox/Methalox/Hydrolox)
│   ├── pods.ron         # Command pods
│   ├── tanks.ron        # Fuel tanks (multiple sizes per category)
│   └── structural.ron   # Decouplers (4 sizes: tiny/small/medium/large)
└── blueprints/          # User-saved vessel designs (RON files)
```

## Key Architecture Details

### Solar System
- Bodies are **hardcoded** in `src/bodies.rs`. The `data/bodies/` RON files are stale/unused.
- **Earth** is body index 3 (`LAUNCHPAD_BODY_INDEX` in game.rs) and is the home world.
- Real-world values: Earth radius 6,371 km, orbital velocity 7.8 km/s, Moon at 384,400 km.
- Gravitational constant `G = 6.67430e-11`.

### Coordinate Systems
- All positions use **f64** to prevent precision loss at large distances.
- **Grid square** = 0.5m x 0.5m (`GRID_SQUARE_SIZE` in definition.rs).
- Part sizes: Tiny=1, Small=3, Medium=5, Large=9 grid squares wide.
- Editor uses camera-relative coordinates for rendering; world coordinates for placement logic.
- Flight rendering scales by `SCALE * BODY_SCALE` constants in main.rs.

### Ship vs Vessel
- **Ship** (`src/ship/mod.rs`): The physics point-mass. Handles position, velocity, rotation, gravity, thrust integration, on-rails mode, autopilot. Always exists.
- **FlightVessel** (`src/parts/vessel.rs`): The parts-based vessel. Optional (`game.flight.vessel: Option<FlightVessel>`). Contains parts, fuel, staging. Created when launching from editor.
- Ship and FlightVessel are synced in `game.update()`: vessel position/rotation mirrors ship.

### Part System
- **PartDefinition**: Loaded from RON files. Has mass (tonnes), dimensions, optional EngineData/TankData/PodData/DecouplerData.
- **PlacedPart**: Editor representation with position, fuel_type, tank_filled state.
- **FlightPart**: Runtime part with resources HashMap, engine state, decoupled flag.
- **Three hitbox types**: Build/flight hitbox (collision), welding hitbox (5% larger, for connections), visual sprite (what's drawn).
- Grid alignment: odd dimensions snap to square centers, even to grid lines.

### Staging
- Editor: `Vec<Vec<PlacedPartId>>` — stages contain part IDs.
- Flight: `Vec<Vec<usize>>` — stages contain part indices.
- Stage 0 fires first. `activate_next_stage()` enables engines and fires decouplers.
- Decouplers mark themselves + all parts below (by Y position) as `decoupled`.
- Decoupled parts are excluded from mass, thrust, rendering, and fuel consumption.
- UI uses egui drag-and-drop (`dnd_drag_source`/`dnd_drop_zone`) for reordering.

### Fuel System
- Fuel is **shared** across all non-decoupled tanks (no crossfeed control).
- Three propellant types: Kerolox (RP-1), Methalox (CH4), Hydrolox (LH2).
- Each tank stores LOX + one fuel type. Capacity scales linearly with grid area.
- Engines consume fuel proportionally from all available tanks.
- Tank dry mass: 35 kg per grid square.

### Rendering Pipeline
- wgpu with custom vertex shader (`shader.wgsl`), 4x MSAA.
- `Vertex { position: [f32; 2], color: [f32; 4] }` — everything is colored triangles.
- Editor: `generate_part_vertices()` builds vertex arrays each frame.
- Flight: parts rendered via `generate_part_shape_vertices()`, then fairings in a second pass.
- Procedural details: engines get chambers/nozzles, pods get windows, decouplers get rings + adapter trapezoids with frustum-projection detail lines.

### Delta-V Calculation
- Per-stage: Tsiolkovsky equation `Δv = Isp * g0 * ln(wet/dry)`.
- Simulates staging sequentially: fire decouplers, enable engines, burn all fuel, next stage.
- Isp is thrust-weighted average of active engines (vacuum).
- Displayed per-stage (green text) in staging panel and as total in stats bar.

## Terminology

- **Ship view**: Zoomed in close enough that the ship's triangle icon is invisible.
- **Map view**: Zoomed out enough that the ship's triangle icon is visible.
- **On-rails**: High time warp mode using Keplerian propagation instead of numerical integration.
- **SOI**: Sphere of Influence — boundary where a body's gravity dominates.
- **Patched conics**: Trajectory prediction that stitches Keplerian orbits across SOI boundaries.

## Common Patterns

- `main.rs` orchestrates: it calls `render_flight_frame()` or `render_editor_frame()` based on game mode, handles input dispatch, and bridges data between Game/RenderState.
- `render/state.rs` is the largest file (~3650 lines). It owns the wgpu state, egui context, and all flight UI rendering. Changes to flight UI happen here.
- Editor UI is in `editor/ui.rs`. Editor vertex generation is in `editor/render.rs`.
- Data flows: EditorState -> Blueprint -> FlightVessel (on launch). FlightVessel -> ShipRenderData -> RenderState (each frame for display).
- `RenderState` has request fields (e.g., `staging_reorder`, `engine_toggle_request`) that main.rs reads and applies to the vessel each frame.
