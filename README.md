# Sunscatter

A 2D spaceflight game with **1:1 real-scale** orbital mechanics and interstellar ambitions. Build rockets, launch from Earth, fly to distant planets, and push toward the stars using patched-conic trajectory prediction, relativistic physics, and a 4-component Milky Way gravity model. All physics use actual solar system values — real masses, real distances, real orbital velocities.

## What Makes It Different

- **Real scale**: Earth LEO at 7.8 km/s, Moon at 384,400 km, no scaled-down physics
- **Galactic scope**: 20 celestial bodies from Sagittarius A\* to Martian moons, with enclosed galactic mass modeling
- **Relativistic flight**: Speed-of-light limiting, Lorentz thrust reduction, gravitational time dilation near compact objects, split ship/Earth clocks
- **No game engine**: Custom Rust physics loop, wgpu rendering, egui UI — built for precision at every layer

## Current State

The core gameplay loop is complete and playable end-to-end: design vessels, launch from Earth, orbit, plan maneuvers, transfer between planets, and land.

### Orbital Mechanics
- Velocity Verlet integration with configurable sub-stepping
- Patched conics trajectory prediction across SOI boundaries (with piecewise subdivision for galactic mass profiles)
- SOI transitions with binary-search frame conversion
- On-rails Keplerian propagation (16 warp levels, 1x to 1 trillion x)
- Maneuver nodes: create on any orbit segment, drag to adjust, execute with autopilot and auto-warp
- Porkchop plot transfer planner with Lambert solver
- Closest approach indicators for navigation targets
- Autopilot: Prograde, Retrograde, Radial In/Out, Maneuver Node

### Vehicle Editor
- 148 parts across 8 categories (Pods, Engines, Fuel Tanks, Structural, Aerodynamic, Utility, Electricity, Interstellar)
- 5 part sizes: Tiny (0.5m) to XL (6.5m)
- 6 propellant families: Kerolox, Methalox, Hydrolox, Fusion Fuel, Antimatter, Nuclear Pulse
- Sprite-based rendering with procedural fallbacks (engine nozzles, pod windows, decoupler adapters)
- Mirror symmetry, part rotation, ghost preview with overlap detection
- Payload fairings: click-to-build symmetric shells
- Drag-and-drop staging panel with per-stage delta-v (Tsiolkovsky equation)
- Blueprint save/load in RON format

### Flight Systems
- Staging: engine ignition, decoupler separation, fairing jettison
- Fuel zones with drain priority (asparagus/onion staging)
- Resources: fuel, monopropellant, electric charge (battery, solar, RTG, reactor)
- Aerodynamic drag (orientation-dependent) and per-part thermal model
- Fairing shielding, heat shields, parachutes
- RCS translation/rotation with per-nozzle activation
- Terrain and vessel collision detection
- Multi-vessel tracking with on-rails propagation, vessel switching, debris cleanup

### Interstellar
- Interstellar engines: Daedalus, Z-Pinch, AM-Cat Fusion, Bussard Ramjet, Orion Nuclear Pulse
- Reactors from MW to TW output (Prometheus, Vulcan, Stellarator, Tokamak, Penning, Ixion)
- Shields: Whipple (passive), FRES (electromagnetic), Geodesic (force field)
- Dual-propellant engines (e.g., Fusion Fuel + Antimatter catalyst)
- Velocity display switches to %c above 1% light speed
- Lorentz gamma display, ship proper time vs. Earth coordinate time

### Solar System
20 bodies with full orbital mechanics:

| Region | Bodies |
|--------|--------|
| Galactic center | Sagittarius A\* (SMBH with 4-component enclosed mass model) |
| Inner system | Sun, Mercury, Venus, Earth, Moon, Mars, Phobos, Deimos |
| Outer system | Jupiter + Io/Europa/Ganymede/Callisto, Saturn + Titan/Iapetus/Rhea/Dione, Uranus, Neptune |

Atmospheres on Earth, Venus, Mars, Jupiter, Titan. Accretion disc on Sgr A\*.

### Game Infrastructure
- Title screen, main menu, tracking station
- Save/load with auto-save (5 min) and quicksave slots
- Pause overlay with save/load/quit

## Planned

- **Colonies**: Deploy habitats, extract resources, refine fuel, build infrastructure, grow food, manage crew and power
- **Career mode**: Tech tree progression, part unlocks, funds
- **Docking**: Port alignment, vessel merge/split, resource transfer
- **Crew system**: Capacity, life support, transfer
- **Multiple star systems**: Galaxy-level SOI hierarchy, interstellar SOI transitions
- **Spaceplanes**: Wings with lift, control surfaces, wheels, runways

## Quick Start

```bash
cargo run
```

Requires [Rust](https://rustup.rs). No other dependencies.

## Controls

### Flight

| Action | Input |
|--------|-------|
| Throttle up/down | Left Shift / Left Ctrl |
| Full / cut throttle | Z / X |
| Rotate | Q / E |
| RCS translate | W/S (fore/aft), A/D (lateral) |
| Toggle RCS | R |
| Stage | Space |
| Switch vessel | \[ / \] |
| Focus ship | Backtick |
| Focus body / vessel | Double-click |
| Maneuver node | Click orbit line |
| Pan / Zoom | Left drag / Scroll |
| Time warp | Warp buttons (top bar) |

### Editor

| Action | Input |
|--------|-------|
| Place part | Left click |
| Select / Drag | Left click on part |
| Delete | Delete / Backspace |
| Rotate part | R |
| Deselect | Escape / Right-click |
| Pan / Zoom | Arrow keys or drag / Scroll |

## Tech Stack

| Layer | Choice |
|-------|--------|
| Language | Rust 2021 |
| Rendering | wgpu 0.19 (4x MSAA, custom vertex shaders) |
| UI | egui 0.27 |
| Windowing | winit 0.29 |
| Math | glam 0.25 (f64 precision throughout) |
| Serialization | serde + ron 0.8 |

No game engine. Custom physics loop for full control over orbital mechanics integration, time warp, and frame conversion.

## Project Structure

```
src/
  main.rs                # Event loop, render orchestration, input dispatch
  game.rs                # Game state machine, vessel management, save/load
  bodies.rs              # 20 celestial bodies, Kepler solver, galactic mass model
  ship/
    mod.rs               # Velocity Verlet integration, thrust, autopilot, relativity
    orbit.rs             # State vectors <-> orbital elements
    patched_conics.rs    # Trajectory prediction, galactic mass subdivision
    transfer.rs          # Lambert solver, porkchop plots
    soi.rs               # SOI transitions, frame conversion, on-rails
  parts/
    definition.rs        # 148 part definitions, engines, tanks, pods, interstellar
    blueprint.rs         # Vessel blueprints, mirror symmetry, serialization
    registry.rs          # Blueprint save/load
    vessel.rs            # Flight vessel, fuel zones, staging, delta-v
  editor/
    state.rs             # Part placement, dragging, symmetry
    ui.rs                # Parts palette, staging panel, part info
    render.rs            # Grid, parts, ghost preview, procedural details
  render/
    state.rs             # wgpu pipeline, flight HUD, body/orbit/ship rendering
    camera.rs            # Camera position, zoom, body tracking
    maneuver.rs          # Maneuver node create/drag/burn
    sprites.rs           # Sprite atlas loading
    types.rs             # Render data structures, vertex format

data/
  parts/                 # 22 RON files defining 148 parts
  sprites/               # Engine and plume sprite atlas
  blueprints/            # User-saved vessel designs
  saves/                 # Save games
  bodies/                # Body definitions (RON)
```

## License

- **Source code** (`src/`, `tools/`, `Cargo.toml`, `build.rs`): [MIT License](LICENSE-MIT)
- **Game assets** (`data/`, `assets/`): [All Rights Reserved](LICENSE-ASSETS)
