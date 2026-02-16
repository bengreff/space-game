# Space Game

A 2D space exploration game with realistic orbital mechanics, inspired by Kerbal Space Program and Spaceflight Simulator.

## Project Status

**Phase**: 1 - Core Engine (Complete)
**Current**: Flyable ship with patched conics trajectory prediction

### What Works
- Flyable spaceship in Low Earth Orbit
- Real-scale solar system (Sun, Earth, Moon)
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
cd ~/space-game
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
space-game/
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
