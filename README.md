# Sunscatter

A 2D space exploration and colony-building game with **1:1 real-scale** orbital mechanics. Build rockets, scatter across the stars, and establish colonies on distant worlds. Unlike Kerbal Space Program's 1/10 scale planets, this simulation uses actual solar system values - real masses, real distances, real orbital velocities.

## 1:1 Real Scale

- **Earth orbital velocity**: 7.8 km/s (not KSP's 3.4 km/s)
- **Moon distance**: 384,400 km (real value)
- **Earth radius**: 6,371 km (real value)
- **All physics**: Exact real-world values

## Project Status

**Phase**: 1 - Core Engine (Complete)
**Current**: Flyable ship with patched conics trajectory prediction

### What Works
- Flyable spaceship in Low Earth Orbit (400 km altitude)
- 1:1 real-scale solar system (Sun, Earth, Moon)
- Accurate Keplerian orbital mechanics
- Velocity Verlet physics integration
- Patched conics trajectory prediction across SOI boundaries
- SOI transitions with precise frame conversion
- On-rails time warp up to 1 billion x
- Auto time warp reduction near SOI boundaries
- Hyperbolic orbit rendering (prograde and retrograde)
- HUD with velocity, altitude, throttle, orbital info
- Pan/zoom camera with body tracking

## Quick Start

```bash
# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build and run
cd ~/sunscatter
cargo run
```

## Controls

| Action | Input |
|--------|-------|
| Increase throttle | W |
| Decrease throttle | S |
| Full throttle | Z |
| Cut throttle | X |
| Rotate left | A |
| Rotate right | D |
| Focus on ship | ` (backtick) |
| Pan camera | Left mouse drag |
| Zoom | Scroll wheel |
| Focus on body | Double-click |
| Time warp | Click buttons at top |

## Documentation

| Document | Purpose |
|----------|---------|
| [docs/VISION.md](docs/VISION.md) | Project goals and scope |
| [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) | Feature specs with MVP boundaries |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Technical decisions and data structures |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Development phases and current status |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Decision log with rationale |

## Project Structure

```
sunscatter/
├── src/
│   ├── main.rs           # Entry point, game loop
│   ├── lib.rs            # Library root
│   ├── bodies.rs         # Celestial bodies, orbits
│   ├── ship/             # Ship module
│   │   ├── mod.rs        # Ship struct, physics
│   │   ├── orbit.rs      # Orbital calculations
│   │   ├── patched_conics.rs  # Trajectory prediction
│   │   └── soi.rs        # SOI transitions
│   └── render/           # Rendering module
│       ├── mod.rs        # Re-exports
│       ├── camera.rs     # Camera system
│       ├── state.rs      # Render state, wgpu
│       └── types.rs      # Render data types
├── data/                 # Game data files (RON)
├── docs/                 # Documentation
└── Cargo.toml
```

## Tech Stack

- **Language**: Rust
- **Rendering**: wgpu (triangle fans, 4x MSAA)
- **Windowing**: winit
- **UI**: egui
- **Serialization**: serde + ron
