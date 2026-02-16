# Feature: Colonies

## Overview

Colonies are persistent bases on celestial body surfaces that extract and process resources. They enable sustainable space exploration and interplanetary logistics.

## Core Mechanics

### Colony Establishment

1. Land a vessel containing a Colony Hub part
2. Trigger "Deploy Colony" action
3. Vessel is converted to a Colony entity
4. Colony appears on body surface at that location

```rust
fn deploy_colony(vessel: Vessel, bodies: &Bodies) -> Colony {
    let body = vessel.current_body;
    let position = vessel.surface_position();  // lat/lon

    Colony {
        id: generate_id(),
        name: format!("{} Colony", bodies.get(body).name),
        body,
        position,
        resources: vessel.resources,  // Inherit vessel resources
        modules: extract_modules(&vessel),
        extraction_rate: calculate_extraction_rate(&vessel),
    }
}
```

### Resource Extraction

Colonies extract Ore passively over time:

```rust
fn colony_tick(colony: &mut Colony, dt: f64) {
    let ore_gained = colony.extraction_rate * dt;
    colony.resources.add(ResourceId::Ore, ore_gained);
}
```

Extraction rate depends on:
- Number of Drill modules
- Body type (some bodies richer than others)

### Resource Conversion

Colonies convert Ore to Fuel:

```rust
fn process_resources(colony: &mut Colony, dt: f64) {
    let conversion_rate = colony.conversion_rate;  // ore/second
    let ore_available = colony.resources.get(ResourceId::Ore);
    let ore_to_convert = (conversion_rate * dt).min(ore_available);

    if colony.resources.consume(ResourceId::Ore, ore_to_convert) {
        let fuel_produced = ore_to_convert * ORE_TO_FUEL_RATIO;
        colony.resources.add(ResourceId::Fuel, fuel_produced);
    }
}
```

### Refueling Vessels

When a vessel lands near a colony:

1. UI shows "Refuel from Colony" button
2. Transfer resources from colony to vessel
3. Transfer limited by colony storage and vessel capacity

```rust
fn refuel_vessel(colony: &mut Colony, vessel: &mut Vessel) {
    for resource in [ResourceId::Fuel, ResourceId::LifeSupport] {
        let available = colony.resources.get(resource);
        let vessel_space = vessel.resources.remaining_capacity(resource);
        let transfer = available.min(vessel_space);

        colony.resources.consume(resource, transfer);
        vessel.resources.add(resource, transfer);
    }
}
```

## Colony Modules

Modules determine colony capabilities:

| Module | Function | Resource Effect |
|--------|----------|-----------------|
| Colony Hub | Required, provides storage | Storage: 1000 ore, 500 fuel |
| Drill | Extracts ore | +0.1 ore/second per drill |
| Converter | Processes ore to fuel | +0.05 ore→fuel/second |
| Solar Array | Generates power | Enables processing |
| Storage Tank | Additional capacity | +500 ore or fuel |

MVP: All modules bundled into single deployable part. Later: separate parts.

## Background Simulation

Colonies continue operating when not focused:

```rust
fn update_colonies(game: &mut GameState, dt: f64) {
    for colony in game.colonies.values_mut() {
        colony_tick(colony, dt);
        process_resources(colony, dt);
    }
}
```

This runs every game tick, even during time warp.

## Data Structures

```rust
pub struct Colony {
    pub id: ColonyId,
    pub name: String,
    pub body: BodyId,
    pub position: (f64, f64),  // latitude, longitude
    pub resources: ResourceStorage,
    pub extraction_rate: f64,   // ore per second
    pub conversion_rate: f64,   // ore processed per second
    pub established_time: f64,  // game time when created
}

// For save/load
#[derive(Serialize, Deserialize)]
pub struct ColonySaveData {
    pub id: ColonyId,
    pub name: String,
    pub body: BodyId,
    pub position: (f64, f64),
    pub resources: HashMap<ResourceId, f64>,
    pub extraction_rate: f64,
    pub conversion_rate: f64,
    pub established_time: f64,
}
```

## UI

### Colony List Panel

```
┌─ Colonies ─────────────────────┐
│ ▸ Mun Colony                   │
│   Ore: 1234 │ Fuel: 567        │
│   [Focus] [Rename]             │
│                                │
│ ▸ Duna Outpost                 │
│   Ore: 89 │ Fuel: 12           │
│   [Focus] [Rename]             │
└────────────────────────────────┘
```

### Colony Detail View

When focused on body with colony:

```
┌─ Mun Colony ───────────────────┐
│ Location: 12.3°N, 45.6°E       │
│ Established: Year 1, Day 45    │
│                                │
│ Resources:                     │
│   Ore:  ████████░░ 1234/2000   │
│   Fuel: █████░░░░░  567/1000   │
│                                │
│ Production:                    │
│   Extraction: 0.3 ore/s        │
│   Conversion: 0.15 fuel/s      │
│                                │
│ [Transfer to Vessel]           │
└────────────────────────────────┘
```

## MVP Scope

### Include
- Single colony module part (all-in-one)
- Ore extraction (passive)
- Ore → Fuel conversion
- Refuel landed vessels
- Colony list in UI
- Background simulation during warp

### Exclude (Later)
- Multiple building types
- Construction of new modules
- Colony growth/population
- Supply routes between colonies
- Manufacturing parts
- Launching vessels from colonies

## Edge Cases

- **Multiple colonies on same body**: Allow, each has own position
- **Colony on body with no ore**: Extraction rate = 0, show warning
- **Time warp resource overflow**: Cap at storage capacity
- **Vessel lands on colony**: Vessel is separate, show refuel option
