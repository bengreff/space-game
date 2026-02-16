# Technical Architecture

## Technology Stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Language | Rust | Performance, safety, good ecosystem |
| Rendering | wgpu | Modern graphics API, cross-platform |
| Geometry | Triangle fans | Simple, no external deps for circles |
| Antialiasing | 4x MSAA | Smooth edges without shader complexity |
| Windowing | winit 0.29 | Standard Rust windowing |
| UI | egui + egui-wgpu | Immediate mode, wgpu integration |
| Serialization | serde + ron | Human-readable saves, easy debugging |
| Async | pollster | Simple block_on for wgpu async |

### Why No Game Engine

- Full control over physics loop (critical for orbital mechanics)
- Bevy's ECS would fight custom orbital state management
- 2D rendering is simple enough without engine overhead
- Avoid engine update churn during development

---

## Coordinate Systems

```
┌─────────────────────────────────────────────────────────────────┐
│ Frame             │ Units         │ Use Case                   │
├─────────────────────────────────────────────────────────────────┤
│ Local/Part        │ meters        │ Part attachment, rendering │
│ Vessel            │ meters        │ Physics simulation         │
│ Body-relative     │ meters (f64)  │ Orbits, surface position   │
│ System-relative   │ AU (f64)      │ Interplanetary view        │
│ Galaxy-relative   │ light-years   │ Interstellar travel        │
└─────────────────────────────────────────────────────────────────┘
```

### Critical Rule
**All position values use `f64`**. This prevents precision loss at large distances (billions of meters).

### Frame Transforms
Always transform explicitly between frames. Never mix frames in calculations.

```rust
fn body_to_system(pos: DVec2, body: &CelestialBody, time: f64) -> DVec2 {
    let body_pos = body.position_at(time);
    pos + body_pos
}
```

---

## Core Data Structures

### Celestial Bodies

```rust
pub type BodyId = u32;

pub struct CelestialBody {
    pub id: BodyId,
    pub name: String,
    pub parent: Option<BodyId>,
    pub orbit: Option<OrbitalElements>,  // None for stars/galaxy center
    pub physical: PhysicalProperties,
    pub soi_radius: f64,  // Sphere of influence
}

pub struct PhysicalProperties {
    pub mass: f64,              // kg
    pub radius: f64,            // m
    pub rotation_period: f64,   // seconds
    pub atmosphere: Option<Atmosphere>,
}

pub struct Atmosphere {
    pub surface_pressure: f64,  // Pa
    pub scale_height: f64,      // m
    pub drag_coefficient: f64,
}

pub struct OrbitalElements {
    pub semi_major_axis: f64,      // m
    pub eccentricity: f64,
    pub argument_of_periapsis: f64, // radians
    pub mean_anomaly_at_epoch: f64, // radians
    pub epoch: f64,                 // seconds
}
```

### Vessels

```rust
pub type VesselId = u32;
pub type PartIndex = usize;

pub struct Vessel {
    pub id: VesselId,
    pub name: String,
    pub parts: Vec<Part>,
    pub root_part: PartIndex,
    pub connections: Vec<Connection>,  // Part connectivity graph
    pub staging: Vec<Stage>,
    pub current_stage: usize,
    pub resources: ResourceStorage,
    pub crew: u32,
    pub state: VesselState,
}

pub enum VesselState {
    Building,  // In editor
    Landed { body: BodyId, latitude: f64, longitude: f64 },
    Flying { body: BodyId, position: DVec2, velocity: DVec2 },
    Orbiting { body: BodyId, orbit: OrbitalElements },
}

pub struct Part {
    pub definition_id: PartDefId,  // Reference to part catalog
    pub position: Vec2,            // Relative to root part
    pub rotation: f32,             // Radians
    pub resources: ResourceStorage,
    pub enabled: bool,
}

pub struct Stage {
    pub parts: Vec<PartIndex>,  // Parts activated in this stage
}

pub struct Connection {
    pub from: PartIndex,
    pub to: PartIndex,
    pub attachment_point: Vec2,
}
```

### Resources

```rust
pub type ResourceId = u32;

pub struct ResourceStorage {
    pub amounts: HashMap<ResourceId, f64>,
    pub capacities: HashMap<ResourceId, f64>,
}

impl ResourceStorage {
    pub fn consume(&mut self, resource: ResourceId, amount: f64) -> bool {
        if let Some(current) = self.amounts.get_mut(&resource) {
            if *current >= amount {
                *current -= amount;
                return true;
            }
        }
        false
    }

    pub fn add(&mut self, resource: ResourceId, amount: f64) {
        if let Some(current) = self.amounts.get_mut(&resource) {
            let capacity = self.capacities.get(&resource).copied().unwrap_or(f64::MAX);
            *current = (*current + amount).min(capacity);
        }
    }
}
```

### Colonies

```rust
pub type ColonyId = u32;

pub struct Colony {
    pub id: ColonyId,
    pub name: String,
    pub body: BodyId,
    pub position: (f64, f64),  // lat, lon
    pub resources: ResourceStorage,
    pub extraction_rate: f64,  // ore per second
    pub conversion_rate: f64,  // ore to fuel per second
}
```

### Game State

```rust
pub struct GameState {
    pub time: f64,  // Total elapsed seconds
    pub warp_level: u32,
    pub paused: bool,

    pub bodies: HashMap<BodyId, CelestialBody>,
    pub vessels: HashMap<VesselId, Vessel>,
    pub colonies: HashMap<ColonyId, Colony>,

    pub active_vessel: Option<VesselId>,

    pub tech_state: TechState,
    pub funds: f64,
}

pub struct TechState {
    pub unlocked_nodes: HashSet<TechNodeId>,
    pub unlocked_parts: HashSet<PartDefId>,
}
```

---

## Save File Format

```rust
#[derive(Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,  // Always increment on schema change
    pub saved_at: String,  // ISO timestamp
    pub game_state: GameState,
}
```

### Version Migration

```rust
impl SaveFile {
    pub fn migrate(data: &str) -> Result<SaveFile, Error> {
        let raw: serde_json::Value = ron::from_str(data)?;
        let version = raw["version"].as_u64().unwrap_or(1) as u32;

        let migrated = match version {
            1 => migrate_v1_to_v2(raw)?,
            2 => migrate_v2_to_v3(raw)?,
            CURRENT_VERSION => raw,
            _ => return Err(Error::UnknownVersion),
        };

        ron::from_value(migrated)
    }
}
```

---

## Module Organization

### Current Structure (Implemented)

```
src/
├── main.rs                 # Entry point, event loop, render coordination
├── lib.rs                  # Library root, re-exports
│
├── bodies.rs               # CelestialBody, Orbit, SolarSystem
│                           # Keplerian orbital mechanics
│                           # Full solar system with 20 bodies
│
└── render/
    ├── mod.rs              # Re-exports RenderState, OrbitRenderData
    ├── state.rs            # RenderState, Camera, body/orbit rendering
    │                       # wgpu setup, MSAA, egui integration
    │                       # Hit testing, hover detection
    └── shader.wgsl         # Vertex/fragment shaders
                            # Camera-relative coordinate transform
```

### Planned Structure (Future)

```
src/
├── main.rs                 # Entry point, window setup
├── lib.rs                  # Library root
│
├── game/
│   ├── mod.rs
│   ├── state.rs            # GameState definition
│   ├── tick.rs             # Main game loop tick
│   ├── time.rs             # Time warp, pause
│   └── save.rs             # Save/load, migration
│
├── physics/
│   ├── mod.rs
│   ├── orbits.rs           # Kepler equation, orbital elements
│   ├── propagation.rs      # Orbit propagation
│   ├── maneuvers.rs        # Delta-v calculations
│   └── soi.rs              # SOI detection, transitions
│
├── vessel/
│   ├── mod.rs
│   ├── parts.rs            # Part definitions, catalog
│   ├── staging.rs          # Stage management
│   ├── resources.rs        # Resource flow
│   └── physics.rs          # Vessel physics (thrust, drag)
│
├── bodies/
│   ├── mod.rs
│   ├── registry.rs         # Body lookup, hierarchy
│   ├── atmosphere.rs       # Atmospheric calculations
│   └── surface.rs          # Surface positions
│
├── colony/
│   ├── mod.rs
│   ├── extraction.rs       # Resource extraction
│   └── conversion.rs       # Resource conversion
│
├── ui/
│   ├── mod.rs
│   ├── hud.rs              # Flight HUD
│   ├── map.rs              # Map view
│   ├── editor.rs           # Vehicle editor
│   └── menus.rs            # Main menu, settings
│
├── render/
│   ├── mod.rs
│   ├── setup.rs            # wgpu initialization
│   ├── camera.rs           # Camera, zoom, pan
│   ├── bodies.rs           # Planet/moon rendering
│   ├── vessels.rs          # Vessel rendering
│   └── orbits.rs           # Orbit line rendering
│
└── data/
    ├── mod.rs
    ├── loader.rs           # Load RON files
    └── definitions.rs      # Part/body definition structs
```

### Data Files

```
data/
├── bodies/
│   ├── home_system.ron     # Home star system
│   └── alpha_centauri.ron  # Second star system
├── parts/
│   ├── engines.ron
│   ├── tanks.ron
│   ├── structure.ron
│   └── utility.ron
├── resources.ron           # Resource definitions
└── tech_tree.ron           # Tech tree structure
```

---

## Key Algorithms

### Kepler Equation Solver

Convert mean anomaly to true anomaly (needed for position from time).

```rust
fn solve_kepler(mean_anomaly: f64, eccentricity: f64) -> f64 {
    // Newton-Raphson iteration
    let mut E = mean_anomaly;  // Eccentric anomaly
    for _ in 0..10 {
        let delta = (E - eccentricity * E.sin() - mean_anomaly)
                  / (1.0 - eccentricity * E.cos());
        E -= delta;
        if delta.abs() < 1e-10 {
            break;
        }
    }

    // Convert to true anomaly
    let true_anomaly = 2.0 * ((1.0 + eccentricity).sqrt() * (E / 2.0).tan())
        .atan2((1.0 - eccentricity).sqrt());
    true_anomaly
}
```

### SOI Check

```rust
fn check_soi_transition(vessel: &Vessel, bodies: &BodyRegistry, time: f64) -> Option<BodyId> {
    let current_body = vessel.current_body();
    let vessel_pos = vessel.position_relative_to(current_body, time);

    // Check children (entering child SOI)
    for child_id in bodies.children_of(current_body) {
        let child = bodies.get(child_id);
        let child_pos = child.position_at(time);
        let distance = (vessel_pos - child_pos).length();

        if distance < child.soi_radius {
            return Some(child_id);  // Entering child SOI
        }
    }

    // Check parent (exiting current SOI)
    let current = bodies.get(current_body);
    if vessel_pos.length() > current.soi_radius {
        return current.parent;  // Exiting to parent SOI
    }

    None  // No transition
}
```

### Time Warp Levels

```rust
const WARP_LEVELS: &[f64] = &[
    1.0,        // Real-time
    5.0,
    10.0,
    50.0,
    100.0,
    1_000.0,
    10_000.0,
    100_000.0,   // Interplanetary
    1_000_000.0, // Interstellar
];

fn max_warp_for_situation(vessel: &Vessel) -> usize {
    match vessel.state {
        VesselState::Flying { .. } => 0,     // Real-time only when in atmo
        VesselState::Landed { .. } => 7,     // Up to 100,000x
        VesselState::Orbiting { body, .. } => {
            let altitude = vessel.altitude_above(body);
            let soi = bodies.get(body).soi_radius;
            if altitude < soi * 0.1 {
                4  // Close to body, limit warp
            } else {
                8  // Full warp available
            }
        }
        _ => 0,
    }
}
```

---

## Performance Considerations

### On-Rails vs Active Physics

- **On-rails**: Vessels not being controlled use Keplerian propagation (O(1) per frame)
- **Active**: Controlled vessel uses numerical integration (more expensive)
- Switch to on-rails when: no thrust, no rotation, not in atmosphere

### Body Updates

- Bodies follow fixed Keplerian orbits
- Position calculated on-demand from time, not simulated
- No N-body means no accumulated drift

### Rendering LOD

- Distant bodies: simple circles
- Close bodies: terrain detail (if implemented)
- Orbit lines: reduce point count when zoomed out
