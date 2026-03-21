# Colony Foundation Data Model

Layer 0 of the colony system. Defines all data types for resources, buildings, colonies, tech tree, and science. No UI or simulation logic.

## Module Structure

```
src/colony/
  mod.rs          — Re-exports from submodules
  resources.rs    — ResourceType, ResourceInventory, Company
  buildings.rs    — BuildingType, FactoryRecipe, BuildingInstance, Colony, ColonyManager
  tech.rs         — TechNodeData, TechLineData, TechTree, DiscoveryTracker, ScienceState
```

## Resources (`resources.rs`)

### ResourceType
26-variant enum covering raw, processed, fuel, and consumable resources. Each variant has:
- `display_name()` — human-readable name
- `earth_price()` — `Option<f64>` $/kg purchase price on Earth. `None` for non-purchasable resources (Regolith, AtmosphericCo2, GasGiantAtmosphere, Helium3, Antimatter).

### ResourceInventory
`HashMap<ResourceType, f64>` wrapper tracking kg of each resource. Methods: `get`, `set`, `add`, `remove` (returns false if insufficient), `has_enough`, `has_enough_all`, `remove_all` (atomic), `iter`, `total_mass`. Implements `Default`, `Serialize`, `Deserialize`.

### Company
Player's financial state: `money: f64` (starting $25M), `rd_budget: f64` (starting $1M/yr).

## Buildings (`buildings.rs`)

### BuildingType
22-variant enum. Each variant provides:
- `display_name()` — human-readable name
- `build_cost()` — `Vec<(ResourceType, f64)>` base build cost in kg
- `power_draw_kw()` — power consumption (Factory power varies by recipe, returns 0)
- `power_output_kw()` — power generation (solar farms at 1 AU, reactors constant)
- `maintenance_cost_per_30d()` — resources consumed per 30 days
- `total_build_mass()` — sum of build cost
- `affected_by_habitability()` — true for Habitat, BasicGreenhouse, AdvancedGreenhouse only

Particle Accelerator Mk IV stores per-km values; simulation multiplies by body circumference.

### FactoryRecipe
15-variant enum. Each provides:
- `display_name()`, `inputs()`, `outputs()` — per-batch resource amounts
- `power_draw_kw()` — power while running
- `batch_time_hours()` — time per batch
- `requires_colocation()` — `Option<BuildingType>` (TritiumBreeding and NpuAssembly require FissionReactor)

### BuildingInstance
Runtime state for a single building: type, assigned_resource (mines), assigned_recipe (factories), operational flag, degradation (0-1), water_fill (greenhouses).

### Colony
Per-body colony state: body_index, name, buildings vec, resources inventory, crew count, food, power balance, construction queue, is_orbital_station flag, lab_science_extracted.

### ColonyManager
`Vec<Colony>` with lookup by body_index: `get_by_body()`, `get_by_body_mut()`, `has_colony()`, `add_colony()`.

## Tech Tree (`tech.rs`)

### Data File: `data/tech/tree.ron`
39 tech nodes across 9 eras, 11 efficiency lines, default unlocked parts list. Loaded as `TechTreeFile`.

### TechTree (Runtime)
Static definitions (nodes, lines) loaded from RON file at startup. Dynamic state (unlocked set, line tiers) persisted in saves.
- `is_unlocked()`, `can_unlock()`, `unlock()` — node unlock management
- `is_part_available()` — checks default parts + unlocked nodes
- `is_building_available()` — checks unlocked nodes
- `is_recipe_available()` — checks efficiency line tier gates
- `line_tier()`, `tier_cost()`, `upgrade_line()` — efficiency line management
- `tier_multiplier(tier)` — `1.11^tier` (4.78x at tier 15)
- `apply_save_state()` — restore from save data

### ScienceState
Tracks available science points and cumulative sources (discovery, R&D, lab).

### DiscoveryTracker
One-time milestone flags (first_suborbital, first_orbit, geostationary) and per-body sets (body_orbited, body_landed).

## CelestialBody Extensions (`bodies.rs`)

Three new fields on `CelestialBody`:
- `mineable_resources: Vec<ResourceType>` — resources extractable by Mine
- `atmospheric_resources: Vec<ResourceType>` — resources extractable by Atmospheric Collector
- `habitability_score: u32` — 0-100, affects Habitat/Greenhouse cost multiplier

Populated for all 21 bodies per colonies.md.

## Game Integration (`game.rs`)

Four new fields on `Game`:
- `colony_manager: ColonyManager`
- `company: Company`
- `science: ScienceState`
- `tech_tree: TechTree`

Initialized in `new()` (tech tree loaded from file) and reset in `reset_for_new_game()`.

## Save System (`save.rs`)

Five new fields on `SaveGame`, all with `#[serde(default)]`:
- `colonies: ColonyManager`
- `company: Company`
- `science: ScienceState`
- `tech_unlocked: HashSet<String>` — which nodes are researched
- `tech_line_tiers: HashMap<String, u32>` — efficiency line progress

Tech tree static data NOT saved — reloaded from `data/tech/tree.ron` at startup. Version check changed from `!=` to `>` to accept current and older save versions.
