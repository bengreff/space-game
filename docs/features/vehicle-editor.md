# Feature: Vehicle Editor

## Overview

The vehicle editor allows players to construct spacecraft from discrete parts. This is where creativity and engineering happen.

## Core Mechanics

### Grid-Based Placement

Parts snap to a 2D grid:
- Grid unit: configurable (e.g., 0.25m)
- Parts have defined attachment points
- Parts must connect to existing structure

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
            id: "engine_small",
            name: "RE-L10 Liquid Engine",
            category: Propulsion,
            mass: 0.5,
            cost: 200,

            engine: Some((
                thrust: 20.0,      // kN
                isp_vac: 310.0,    // seconds
                isp_asl: 280.0,    // seconds
                fuel_consumption: 0.065,  // units/s at full thrust
            )),

            attachment_top: Some((size: Small, position: (0.0, 0.5))),
            attachment_bottom: None,
            radial_attachment: true,

            resource_capacity: {},
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
