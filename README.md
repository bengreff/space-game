# Sunscatter

A 2D spaceflight simulator with **1:1 real-scale** orbital mechanics. Build rockets in the vehicle editor, launch from Earth, and fly to the Moon, Mars, or anywhere in the solar system using patched-conic trajectory prediction. Unlike Kerbal Space Program's 1/10 scale planets, this simulation uses actual solar system values — real masses, real distances, real orbital velocities.

**The core gameplay loop is complete**: design, build, launch, orbit, plan maneuvers, transfer between planets, and land. Everything needed to fly missions from Earth to other bodies and back.

## 1:1 Real Scale

- **Earth orbital velocity**: 7.8 km/s (not KSP's 3.4 km/s)
- **Moon distance**: 384,400 km (real value)
- **Earth radius**: 6,371 km (real value)
- **20 celestial bodies**: Sun, Mercury, Venus, Earth, Moon, Mars, Jupiter + 4 Galilean moons, Saturn + 4 moons, Uranus, Neptune
- **All physics**: Exact real-world values with `G = 6.67430e-11`

## Features

**Vehicle Editor**
- Part palette with 6 categories: Pods, Engines, Fuel Tanks, Structural, Aerodynamic, Utility
- 5 part sizes: Tiny, Small, Medium, Large, XL
- 16 unique engines across 3 propellant types (Kerolox/Methalox/Hydrolox)
- Procedural part rendering: engine nozzles with cooling rings, pod windows, decoupler adapters
- Payload fairings: click-to-build symmetric shells that protect parts from aerodynamic heating
- Grid-based placement with overlap detection, ghost preview, and fairing boundary validation
- Mirror symmetry mode for symmetric builds
- Drag-and-drop staging panel with per-stage delta-v calculation
- Part info panel with fuel type selection, fill/empty toggle, crossfeed control
- Blueprint save/load (RON format)

**Orbital Mechanics**
- Velocity Verlet physics integration with sub-stepping
- Patched conics trajectory prediction across SOI boundaries
- SOI transitions with precise binary-search frame conversion
- On-rails Keplerian propagation at high time warp (up to 1,000,000,000x)
- Maneuver node creation on any orbit segment (current trajectory or post-maneuver predictions)
- Maneuver node editing, dragging, and execution with auto-warp-to-node
- Transfer planner for interplanetary missions with phase angle computation
- Closest approach indicators for navigation targets

**Flight Systems**
- Part-based vessel rendering with engine exhaust and RCS plumes
- Staging activation: engine ignition, decoupler separation, fairing jettison (two-half debris)
- Fuel zones with drain priority — asparagus/onion staging drains outer tanks first
- RCS translation and rotation with per-nozzle activation
- Autopilot (SAS): Prograde, Retrograde, Radial In/Out, Maneuver Node hold
- Aerodynamic drag (orientation-dependent) and heating with per-part thermal model
- Fairing shielding protects enclosed parts from aerodynamic heating
- Heat shields with high thermal tolerance
- Terrain and vessel collision detection
- Vessel recovery system

**HUD and Navigation**
- Velocity, altitude, orbital info, throttle/fuel bars, staging panel
- Apoapsis/periapsis markers with altitude labels
- Body hover labels, click-to-focus, camera tracking
- Multi-vessel tracking station with vessel switching
- Textured celestial body rendering

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
| Throttle up/down | W / S (also: forward/back translation with RCS) |
| Full / cut throttle | Z / X |
| Rotate left/right | Q / E |
| RCS translate left/right | A / D |
| Stage | Space |
| Focus on ship | ` (backtick) |
| Pan camera | Left mouse drag |
| Zoom | Scroll wheel |
| Focus on body | Double-click body |
| Time warp | Click warp buttons |
| Create maneuver node | Click on orbit line |

### Editor Mode

| Action | Input |
|--------|-------|
| Place part / fairing vertex | Left click |
| Select placed part | Left click on part |
| Delete part | Delete / Backspace |
| Deselect / exit fairing build | Escape or right-click |
| Undo fairing vertex | Right-click (in fairing build mode) |
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
│   ├── transfer.rs      # Transfer planner, phase angle computation
│   └── soi.rs           # SOI transition detection, frame conversion
├── parts/               # Part definitions and vessel systems
│   ├── definition.rs    # PartDefinition, EngineData, TankData, PodData, FairingData
│   ├── blueprint.rs     # VesselBlueprint, PlacedPart, FairingShape, mirror symmetry
│   ├── registry.rs      # Blueprint save/load (RON files)
│   └── vessel.rs        # FlightVessel, fuel zones, drain priority, staging, delta-v
├── editor/              # Vehicle editor
│   ├── state.rs         # EditorState, placement, dragging, fairing build mode
│   ├── ui.rs            # egui UI: toolbar, palette, staging, part info
│   └── render.rs        # Grid, parts, ghost preview, fairing shells, procedural details
└── render/              # Flight rendering and HUD
    ├── camera.rs        # Camera (position, zoom, body tracking)
    ├── geometry.rs      # Circle, ring, ship triangle primitives
    ├── maneuver.rs      # Maneuver node management (create, drag, burn on any orbit)
    ├── types.rs         # Render data types, Vertex struct
    └── state.rs         # wgpu state, flight HUD, body/orbit/ship rendering

data/
├── parts/               # Part definitions (RON)
│   ├── engines.ron      # 16 engines
│   ├── pods.ron         # Command pods
│   ├── tanks.ron        # Fuel tanks
│   ├── structural.ron   # Decouplers and fairings
│   └── aerodynamic.ron  # Heat shields, nose cones, RCS thrusters
└── blueprints/          # User-saved vessel designs

openspec/specs/game/     # Requirements specs
├── spec.md              # Game-wide shared requirements
├── editor/              # Editor logic (parts, persistence, staging)
├── editor_rendering/    # Editor GUI and part drawing
├── flight_rendering/    # Flight HUD, bodies, ship, maneuver nodes
├── orbits/              # Orbital mechanics, patched conics
└── physics/             # Drag, heating, fuel system, collisions
```

## Tech Stack

- **Language**: Rust (edition 2021)
- **Rendering**: wgpu 0.19 (colored triangles + body textures, 4x MSAA, custom vertex shaders)
- **Windowing**: winit 0.29
- **UI**: egui 0.27 (immediate mode)
- **Serialization**: serde + ron 0.8
- **No game engine** — custom physics loop, wgpu rendering, egui UI
