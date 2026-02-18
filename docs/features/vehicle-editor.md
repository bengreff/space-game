# Feature: Vehicle Editor

## Overview

The vehicle editor allows players to construct spacecraft from discrete parts. This is where creativity and engineering happen.

## Implementation Status

### Completed
- [x] Editor mode vs flight mode toggle (E key)
- [x] Part definitions loaded from RON files
- [x] Parts palette with category filtering
- [x] Grid-based placement (0.5m grid)
- [x] Ghost preview with validity indication
- [x] Part overlap detection (AABB collision)
- [x] Click to place/select parts
- [x] Drag to reposition placed parts
- [x] Delete selected parts
- [x] Save/load blueprints to RON files
- [x] Procedural part rendering (engines, pods)
- [x] Fuel display in tonnes when >= 1000 kg

### Pending
- [ ] Symmetry mode (horizontal mirror)
- [ ] Staging assignment UI
- [ ] Part connection validation (attachment points)
- [ ] Undo/redo

## Core Mechanics

### Grid-Based Placement

Parts snap to a 2D grid:
- Grid unit: 0.5m (configurable)
- Parts have defined visual and hitbox dimensions
- Parts cannot overlap (AABB collision detection)

### Part Attachment

```rust
struct AttachmentPoint {
    local_position: Vec2,  // Position on part
    direction: Vec2,       // Which way it faces
    size: AttachmentSize,  // Small, Medium, Large
}

enum AttachmentSize {
    Small,
    Medium,
    Large,
    Radial,  // Can attach to side of any part
}
```

Rules:
- Stack attachments (top/bottom) must match size
- Radial attachments can go anywhere
- Connection forms a tree from root part

### Symmetry

MVP: Horizontal mirror only

```rust
fn place_with_symmetry(part: Part, position: Vec2, symmetry: Symmetry) -> Vec<Part> {
    match symmetry {
        Symmetry::None => vec![part],
        Symmetry::Mirror => {
            let mirrored = part.clone();
            mirrored.position.x = -position.x;
            mirrored.rotation = -part.rotation;
            vec![part, mirrored]
        }
    }
}
```

### Staging

Parts are assigned to stages in editor:
- Decouplers/separators: activate to decouple
- Engines: activate to ignite
- Parachutes: activate to deploy

Stage order determines activation sequence (highest first).

## UI Layout

```
┌────────────────────────────────────────────────────────────┐
│  [Parts]  [Staging]  [Save]  [Load]  [Launch]              │
├───────────┬────────────────────────────────────────────────┤
│           │                                                │
│  PARTS    │              BUILD AREA                        │
│  LIST     │                                                │
│           │              (grid + vessel)                   │
│  [Engines]│                                                │
│  [Tanks]  │                                                │
│  [Struct] │                                                │
│  [Aero]   │                                                │
│  [Util]   │                                                │
│           │                                                │
├───────────┴────────────────────────────────────────────────┤
│  Part Info: [Selected Part Name]  Mass: X  Cost: Y        │
└────────────────────────────────────────────────────────────┘
```

## Data Structures

### Part Definition (in data files)

```rust
// data/parts/engines.ron
(
    parts: [
        (
            id: "engine_wolf",
            name: "Wolf Engine",
            description: "A reusable kerolox engine...",
            category: Propulsion,
            mass: 0.47,
            cost: 850,
            size: Small,
            shape: Trapezoid,
            grid_width: 2.5,      // Visual width in grid units
            grid_height: 3.0,     // Visual height
            top_width: Some(1.0), // Top width for trapezoid
            hitbox_width: Some(3),
            hitbox_height: Some(3),
            engine: Some((
                thrust_vac: 845.0,   // kN vacuum
                thrust_asl: 760.0,   // kN sea level
                isp_vac: 311.0,      // seconds
                isp_asl: 282.0,
                propellant: Kerolox,
                gimbal_range: 5.0,
            )),
            resources: {},
        ),
    ]
)

// data/parts/pods.ron
(
    parts: [
        (
            id: "pod_small",
            name: "Small Command Pod",
            category: Pods,
            shape: Trapezoid,
            grid_width: 3.0,
            grid_height: 3.0,
            top_width: Some(0.5),
            pod: Some((
                crew_capacity: 1,
                torque: 5.0,
            )),
        ),
    ]
)
```

### Vessel Blueprint

```rust
#[derive(Serialize, Deserialize)]
struct Blueprint {
    name: String,
    parts: Vec<PlacedPart>,
    staging: Vec<Stage>,
}

struct PlacedPart {
    definition_id: String,
    position: Vec2,
    rotation: f32,
}
```

## Operations

### Place Part
1. Select part from palette
2. Move to position (snaps to grid)
3. Validate attachment (must connect to existing)
4. Click to place
5. Auto-add to staging (engines last, decouplers first)

### Delete Part
1. Select placed part
2. Press delete
3. Check if removing breaks vessel in two
4. If would break: ask confirmation or prevent

### Rotate Part
- Before placement: scroll wheel or R key
- After placement: select and rotate

### Stage Assignment
- Drag parts between stages
- Drag stages to reorder
- Parts can be in multiple stages (engine + decoupler same stage)

## Validation

Before launch, validate:
- [ ] Has exactly one root part
- [ ] Root part is command capable (pod or probe)
- [ ] All parts connected (no floating parts)
- [ ] Has at least one engine (warning only)
- [ ] Has propellant (warning only)

## File Format

Blueprints saved as RON:

```rust
// blueprints/my_rocket.ron
(
    name: "My First Rocket",
    created: "2025-02-15T10:30:00Z",
    parts: [
        (def: "pod_mk1", position: (0.0, 3.0), rotation: 0.0),
        (def: "tank_medium", position: (0.0, 1.5), rotation: 0.0),
        (def: "engine_medium", position: (0.0, 0.0), rotation: 0.0),
    ],
    staging: [
        (parts: [2]),  // Stage 0: Engine
    ],
)
```

## Edge Cases

- **Cyclic attachment**: Prevent loops in connection graph
- **Overlapping parts**: Either allow or detect/prevent
- **Symmetry breaks**: What if one mirror part can attach but other can't?
- **Empty vessel**: Prevent launch with no parts
