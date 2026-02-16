# Space Game

A 2D space exploration game combining Spaceflight Simulator's accessible mechanics with KSP 2's scope: multiple star systems, colonies, and interstellar travel.

## Project Status

**Phase**: 1 - Core Engine
**Current**: Solar system visualization with orbital mechanics complete

### What Works
- Full solar system: Sun, 8 planets, and 12 major moons
- 1/4 scale physics (KSP-style): ~4.7 km/s to Earth orbit
- Accurate Keplerian orbital mechanics (half real orbital periods)
- Pan/zoom camera with smooth body tracking
- Double-click to focus and follow any body
- Orbit lines with conditional visibility

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
| Pan camera | Left mouse drag |
| Zoom | Scroll wheel |
| Focus on body | Double-click |
| Exit | Close window |

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
├── src/               # Rust source code
├── data/              # Game data files (RON format)
│   ├── bodies/        # Celestial body definitions
│   └── parts/         # Part definitions
├── docs/              # Documentation
│   └── features/      # Detailed feature specs
├── Cargo.toml         # Rust dependencies
└── README.md
```

## Tech Stack

- **Language**: Rust
- **Rendering**: wgpu + lyon
- **Windowing**: winit
- **UI**: egui
- **Serialization**: serde + ron
