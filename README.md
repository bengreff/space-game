# Sunscatter

A 2D space game with **1:1 real-scale** orbital mechanics. Build rockets in the vehicle editor, launch from Earth, and fly using patched-conic trajectory prediction. Unlike Kerbal Space Program's 1/10 scale planets, this simulation uses actual solar system values - real masses, real distances, real orbital velocities.

## 1:1 Real Scale

- **Earth orbital velocity**: 7.8 km/s (not KSP's 3.4 km/s)
- **Moon distance**: 384,400 km (real value)
- **Earth radius**: 6,371 km (real value)
- **20 celestial bodies**: Sun, Mercury, Venus, Earth, Moon, Mars, Jupiter + 4 Galilean moons, Saturn + 4 moons, Uranus, Neptune
- **All physics**: Exact real-world values with `G = 6.67430e-11`

## What Works

**Vehicle Editor**
- Part palette with 6 categories: Pods, Engines, Fuel Tanks, Structural, Aerodynamic, Utility
- 16 unique engines across 4 sizes (Tiny/Small/Medium/Large) and 3 propellant types (Kerolox/Methalox/Hydrolox)
- Procedural part rendering: engine nozzles with cooling rings, pod windows, decoupler adapters
- Grid-based placement with overlap detection and ghost preview
- Mirror symmetry mode for symmetric builds
- Drag-and-drop staging panel with per-stage delta-v calculation
- Part info panel with fuel type selection, fill/empty toggle, crossfeed control
- Blueprint save/load (RON format)

**Flight Mode**
- Velocity Verlet physics integration with sub-stepping
- Patched conics trajectory prediction across SOI boundaries
- SOI transitions with precise binary-search frame conversion
- On-rails Keplerian propagation at high time warp (up to 1 billion x)
- Maneuver node creation, editing, and execution with predicted trajectories
- Part-based vessel rendering with engine exhaust plumes
- Staging activation (engine ignition, decoupler separation)
- Fuel consumption across all non-decoupled tanks
- Autopilot (SAS): Prograde, Retrograde, Radial In/Out, Maneuver Node hold
- HUD: velocity, altitude, orbital info, throttle/fuel bars, staging panel
- Procedural surface scenery (trees, launchpad on Earth)
- Body hover labels, click-to-focus, camera tracking

## Quick Start

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and run
cargo run
```

## Controls

### Flight Mode

| Action | Input |
|--------|-------|
| Throttle up/down | W / S |
| Full / cut throttle | Z / X |
| Rotate left/right | A / D |
| Focus on ship | ` (backtick) |
| Pan camera | Left mouse drag |
| Zoom | Scroll wheel |
| Focus on body | Double-click body |
| Time warp | Click warp buttons |
| Create maneuver node | Click on orbit line |

### Editor Mode

| Action | Input |
|--------|-------|
| Place part | Left click (with part selected) |
| Select placed part | Left click on part |
| Delete part | Delete / Backspace |
| Deselect | Escape or right-click |
| Toggle symmetry | R |
| Pan camera | Arrow keys or drag |
| Zoom | Scroll wheel |

## Project Structure

```
src/
├── main.rs              # Entry point, event loop, render orchestration
├── game.rs              # Game state, GameMode, launch_from_editor()
├── bodies.rs            # CelestialBody, Orbit, SolarSystem (20 bodies)
├── ship/                # Ship physics
│   ├── mod.rs           # Velocity Verlet integration, thrust, rotation, autopilot
│   ├── orbit.rs         # State vectors <-> orbital elements
│   ├── patched_conics.rs # Trajectory prediction across SOI boundaries
│   └── soi.rs           # SOI transition detection, frame conversion
├── parts/               # Part definitions and vessel systems
│   ├── definition.rs    # PartDefinition, EngineData, TankData, PodData
│   ├── blueprint.rs     # VesselBlueprint, PlacedPart, mirror symmetry
│   ├── registry.rs      # Blueprint save/load (RON files)
│   └── vessel.rs        # FlightVessel, fuel consumption, staging, delta-v
├── editor/              # Vehicle editor
│   ├── state.rs         # EditorState, placement, dragging, deletion
│   ├── ui.rs            # egui UI: toolbar, palette, staging, part info
│   └── render.rs        # Grid, parts, ghost preview, procedural details
└── render/              # Flight rendering and HUD
    ├── camera.rs        # Camera (position, zoom, body tracking)
    ├── geometry.rs      # Circle, ring, ship triangle primitives
    ├── maneuver.rs      # Maneuver node management
    ├── types.rs         # Render data types, Vertex struct
    └── state.rs         # wgpu state, flight HUD, body/orbit/ship rendering

data/
├── parts/               # Part definitions (RON)
│   ├── engines.ron      # 16 engines
│   ├── pods.ron         # Command pods
│   ├── tanks.ron        # Fuel tanks
│   └── structural.ron   # Decouplers
└── blueprints/          # User-saved vessel designs

openspec/specs/game/     # Requirements specs
├── spec.md              # Game-wide shared requirements
├── editor/              # Editor logic (parts, persistence, staging)
├── editor_rendering/    # Editor GUI and part drawing
├── flight_rendering/    # Flight HUD, bodies, ship, maneuver nodes
└── orbits/              # Physics, celestial bodies, patched conics
```

## Tech Stack

- **Language**: Rust (edition 2021)
- **Rendering**: wgpu 0.19 (colored triangles, 4x MSAA, custom vertex shaders)
- **Windowing**: winit 0.29
- **UI**: egui 0.27 (immediate mode)
- **Serialization**: serde + ron 0.8
- **No game engine** - custom physics loop, wgpu rendering, egui UI
