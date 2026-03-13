# Colony System — Implementation Roadmap

This document organizes the full colony/economy system (see `colonies.md`) into layered implementation plans. Each plan is a starting point — scope, decisions, and references — not a detailed spec. Plans within a layer can be implemented in parallel. Layers follow a dependency graph (see bottom of document) — some layers are parallel (e.g., Layers 1 and 2 both depend only on Layer 0).

---

## Resolved Design Decisions

These decisions were made upfront to avoid ambiguity during implementation.

| Decision | Resolution |
|----------|-----------|
| **Colony UI** | New `GameMode::ColonyManagement` — dedicated full-screen mode, separate from TrackingStation |
| **Colony overview** | Both: summary dashboard listing all colonies with key stats, plus per-colony detail view |
| **Time warp simulation** | Batch ticks with adaptive granularity. Cap at ~1,000 ticks/frame; batch size scales with warp level (1 day at low warp, up to ~200 days/tick at max 10¹²× warp). See Plan 1.2 |
| **Colony establishment** | UI button after landing — ship lands normally, player clicks "Establish Colony" to confirm |
| **Trade ships** | Fully separate from flight ships. Built from blueprints but never physically simulated |
| **Tech tree storage** | RON data file (`data/tech/tree.ron`). Easy to iterate without recompiling |
| **Crew starvation** | 30-day grace period, then crew die. Real consequences for neglect |
| **Construction resources** | Locked (reserved) from inventory when building is queued |
| **Part material costs** | Computed at load time from mass × category percentage table. Not stored in RON |
| **Body resource data** | Added to `CelestialBody` in `bodies.rs` (mineable_resources, atmospheric_resources, habitability_score) |
| **Robot time split** | Maintenance always first, remaining capacity goes to construction queue |
| **Maintenance timing** | Continuous — 1/30th of monthly cost consumed per day. Smooth with 1-day batch sim |
| **Gas giant stations** | Same `Colony` struct with `is_orbital_station: bool` flag. Habitability = 0. Placed in orbit for delta-v calculations |
| **Railgun** | Placeholder building only — future use for orbital megastructures (Dyson Spheres). Not functional in this roadmap |
| **Exoplanets** | 10-20 predefined systems + procedural beyond that. Not implemented in this plan |
| **Notifications** | Toast popups + persistent inbox. **Warp-stopping** alerts for: colony food/unavailable-resource depletion, inactive ship SOI change, ship food depletion, ship/colony power loss, company funds zero. All others (science, contracts, trade routes) go to inbox only |
| **Interstellar colonies** | No light-speed observation delay — player manages all colonies in real time. Light-speed delay only affects **tech propagation**: interstellar colonies can use a newly unlocked tech only after distance/c delay from Earth |

---

## Layer 0: Foundation Data Model

Everything downstream depends on these data structures existing and being serializable. No UI, no simulation — just types and persistence.

### Plan 0.1: Resource & Economy Types

**Scope**: Define all 26 resource types, resource inventories, money, and the player company.

**What it covers**:
- `ResourceType` enum (26 variants from colonies.md §1: Metal Ore, Regolith, Water, ..., Food)
- `ResourceInventory` — HashMap<ResourceType, f64> for tracking kg of each resource
- `Money` type (f64 or i64 cents) on the Game struct
- `Company` struct — money balance, R&D budget setting
- Serde derives for all types, RON serialization

**Decisions**: Power is a separate stat, not a resource type. f64 kg for all resource quantities.

**References**: colonies.md §1 (Resources), §8 (Economy — Company)

**Critical files**: `src/game.rs` (Game struct), new `src/colony/` module

---

### Plan 0.2: Colony & Building Data Model

**Scope**: Define colony state, building types, building instances, and the per-body data needed for colonies.

**What it covers**:
- `Colony` struct — body_index, buildings vec, resource inventory, crew count, power balance, construction queue
- `BuildingType` enum (Habitat, BasicGreenhouse, AdvancedGreenhouse, SmallSolarFarm, MediumSolarFarm, LargeSolarFarm, FissionReactor, FusionReactor, Mine, AtmosphericCollector, Factory, Launchpad, Railgun, LightConstructionRobot, ConstructionRobot, ScienceLab, Stockpile, FoodStorage, ParticleAcceleratorMk1-4)
- `BuildingInstance` — type, assigned resource (for mines), assigned recipe (for factories), operational status, degradation level. (Tech tiers are global per tech line, not per-building — see Design Flaw #12)
- `ColonyManager` — Vec<Colony>, lookup by body_index
- Body resource availability data (colonies.md §2 — which bodies have which minable resources)
- Habitability scores per body (colonies.md §3)

**Decisions**: Body resource availability on `CelestialBody` in `bodies.rs`. Gas giant orbital stations use same Colony struct with `is_orbital_station: bool` flag (habitability = 0, placed in orbit for delta-v).

**References**: colonies.md §2 (Resource Availability), §3 (Habitability), §4 (Colony Buildings)

**Critical files**: `src/bodies.rs` (CelestialBody), new `src/colony/mod.rs`, new `src/colony/buildings.rs`

---

### Plan 0.3: Tech Tree & Science Data Model

**Scope**: Define tech tree structure, science tracking, and tech unlock state.

**What it covers**:
- `TechNode` — id, name, era, cost, prerequisites, unlocks (parts, buildings, capabilities)
- `TechTree` — all nodes from colonies.md §10, current unlock state per node
- `TechLine` — efficiency upgrade lines (11 lines × 15 tiers each), current tier per line
- `ScienceState` — cumulative discovery science, cumulative R&D science, cumulative lab science, R&D budget
- R&D exhaustion pool math: `effective_rate = base_rate × (1 - cumulative / 50,000)`
- Discovery tracking: which milestones/bodies have been visited/landed

**Decisions**: Tech tree loaded from RON data file (`data/tech/tree.ron`). Editor part palette filtered by unlock state — `PartDefinitions` loads everything but UI only shows unlocked parts.

**References**: colonies.md §8 (Science), §10 (Tech Tree)

**Critical files**: `src/game.rs`, new `src/colony/tech.rs`, `src/parts/definition.rs` (add tech_node field to PartDefinition)

---

### Plan 0.4: Save System Extension

**Scope**: Extend SaveGame to persist all new state.

**What it covers**:
- Add to SaveGame: colonies, company/money, science state, tech tree state, trade routes, fleet
- Version bump strategy (current save version + migration)
- Ensure backward compatibility — old saves load without colonies

**Decisions**: Use `#[serde(default)]` for all new fields so old saves deserialize cleanly. Monitor save file size — thousands of buildings could get large but RON should handle it.

**References**: `src/save.rs` (existing SaveGame struct)

**Critical files**: `src/save.rs`, `src/game.rs`

---

## Layer 1: Colony Core Loop

Basic colony operations: create a colony, build things, produce resources, consume food. No trade routes, no tech tree gating, no UI polish.

### Plan 1.1: Colony Establishment

**Scope**: Land a ship with a Colony Module on a body → colony is created.

**What it covers**:
- Colony Module as a new ship part (cargo category, 15,000 kg)
- Detection: ship lands on a body without a colony + has Colony Module → trigger establishment
- Colony initialization from Colony Module contents (Habitat, 100kW solar, 3000kg food, 100,000kg storage)
- Part definition in RON data file
- Block colony creation on gas giants (surface) — separate "orbital station" path needed later

**Decisions**: Establishment via UI button — ship lands, player clicks "Establish Colony" to confirm. Colony Module consumed from FlightVessel (removed from parts list, mass recalculated). Check `ShipState::Landed` + Colony Module presence to enable the button.

**References**: colonies.md §6 (Colony Establishment)

**Critical files**: `src/parts/vessel.rs` (FlightVessel), `src/ship/mod.rs` (ShipState), `src/game.rs`

---

### Plan 1.2: Resource Production & Consumption Simulation

**Scope**: The tick-based colony simulation loop — mines produce, factories process, greenhouses grow food, crew consume food, power balances.

**What it covers**:
- Colony update function called each game tick (or batched for time warp)
- Mine output: 2,000 kg/day of assigned resource
- Factory processing: run assigned recipe, consume inputs, produce outputs per batch timing
- Greenhouse food production: scales with water fill level
- Food consumption: 0.5 kg/crew/day
- Power generation (solar farms, reactors) vs power consumption (all buildings)
- Solar power distance scaling: `output × (1 AU / distance)²` — use body's orbital semi-major axis
- Maintenance cycle: every 30 days, consume maintenance resources if available
- Building degradation when maintenance or power is insufficient

**Decisions**: Batch ticks with adaptive granularity. Base unit = 1 day. At extreme warp levels (max is 10¹²×), cap at ~1,000 ticks per frame and increase batch size proportionally: at 10¹²× warp with 60fps, each tick covers ~200 days. Factory recipes: compute throughput per tick-period (e.g., Metal Smelting = 400 kg/day × days_per_tick) rather than simulating individual batches. Resource depletion: proportional allocation when multiple buildings compete for same input. Recipe availability gated by tech tiers from the start — Factories can only run recipes the player has unlocked.

**References**: colonies.md §4 (all building stats), §5 (Factory Recipes)

**Critical files**: new `src/colony/simulation.rs`

---

### Plan 1.3: Building Construction Queue

**Scope**: Queue buildings for construction, Construction Robots assemble them over time.

**What it covers**:
- Construction queue: ordered list of buildings to build
- Each queued building: type, resources reserved from inventory, assembly progress (kg assembled)
- Construction Robot assembly: 20 t/day (full) or 5 t/day (light) per robot
- Multiple robots can work on the same building
- Resource validation: can't queue if insufficient resources in inventory
- Habitability multiplier on Habitat/Greenhouse costs
- Building becomes operational when assembly completes

**Decisions**: Player can reorder the queue. Resources locked when queued (reserved from inventory immediately). Robots under construction can't be used — only operational robots contribute to assembly/maintenance.

**References**: colonies.md §4 (Construction Robot, Light Construction Robot), §6 (Management Model)

**Critical files**: new `src/colony/construction.rs`

---

### Plan 1.4: Critical Notifications (Warp-Stopping)

**Scope**: Notifications that stop time warp for critical colony/ship events. Must exist as soon as colony simulation runs, or colonies silently starve during high warp.

**What it covers**:
- Notification infrastructure: queue, display, warp-stop trigger
- **Warp-stopping events**: colony food depletion, colony resource depletion (for resources unavailable on that body), ship food depletion, colony/ship power loss, company funds reaching zero, inactive ship SOI change
- Toast popup on screen when warp stops
- All notifications added to persistent inbox (Plan 7.1 expands inbox UI and adds minor notifications)

**Decisions**: Critical notifications stop warp and show on screen immediately. Minor notifications (science, contracts, trade routes) deferred to Plan 7.1.

**References**: colonies.md §6 (Maintenance — graceful degradation)

**Critical files**: `src/game.rs`, `src/render/state.rs` (toast display)

---

### Plan 1.5: Maintenance System

**Scope**: Buildings consume maintenance resources continuously (1/30th of monthly cost per day), requiring both resources and robot capacity.

**What it covers**:
- Continuous maintenance: each building consumes 1/30th of its monthly maintenance cost per simulation tick
- Each building's maintenance cost (from colonies.md §4)
- Construction Robot maintenance throughput: 60 t/day (full) or 15 t/day (light)
- If resources insufficient: proportional output degradation
- If robot capacity insufficient: proportional degradation even with resources
- Habitat/Greenhouse maintenance scales with habitability multiplier
- Graceful degradation — no catastrophic failures

**Decisions**: Continuous maintenance — 1/30th of monthly cost consumed per day. Robots always prioritize maintenance, remaining capacity goes to construction. Degradation is linear with shortfall percentage (50% shortfall = 50% output).

**References**: colonies.md §4 (per-building maintenance stats), §6 (Maintenance)

**Critical files**: `src/colony/simulation.rs`

---

### Plan 1.6: Science Lab Building

**Scope**: Science Lab as a colony building that extracts science over time.

**What it covers**:
- Science Lab building: extracts science from body on a logarithmic exhaustion curve
- Formula: `extracted(t) = 10 × landing_value × (1 − e^(−N×t/15))` where N = number of labs
- Multiple labs on same body share the exhaustion pool but extract faster
- Per-body extraction tracking (cumulative science extracted so far)
- Lab science batched with colony simulation ticks

**Decisions**: Lab science extracted continuously, batched with colony simulation ticks. Exoplanets deferred — not in this implementation plan.

**References**: colonies.md §4 (Science Lab), §8 (Science — Lab)

**Critical files**: `src/colony/simulation.rs`

---

## Layer 2: Economy & Science

Money, science, tech tree unlocks, and ship part costs. Bridges the colony system with the existing editor/flight systems.

### Plan 2.1: Money System

**Scope**: Track money, charge for ship construction, implement contracts.

**What it covers**:
- Company money on Game struct — displayed in editor and flight HUD
- Ship construction cost: computed from part material breakdowns × Earth prices (colonies.md §7)
- Launching a ship from Earth costs money (sum of all part costs + fuel costs)
- Contract system: list of available contracts with conditions and payouts
- Contract completion detection (altitude checks, orbit detection, landing detection, payload mass)
- Starting cash: $25M

**Decisions**: Ship cost shown in editor stats bar alongside delta-v. Contract detection uses existing flight state (altitude from ship position, orbit state from orbital elements, body index from SOI). Tourism contracts check pod crew capacity.

**References**: colonies.md §7 (Ship Part Resource Costs), §8 (Economy — Company, Contracts)

**Critical files**: `src/editor/state.rs` (stats), `src/editor/ui.rs` (display), `src/render/state.rs` (flight HUD), `src/game.rs`

---

### Plan 2.2: Science & Tech Tree System

**Scope**: Science accumulation from all three sources, tech tree UI, and unlock gating.

**What it covers**:
- **Discovery science**: detect milestones during flight and award science
  - Earth milestones: first suborbital (100km), first orbit, geostationary altitude
  - Per-body discoveries: first orbit, first landing (distance-scaled rewards from §8 formula)
  - "First orbit" = ship enters stable orbit (eccentricity < 1) around a body for the first time
  - Track per-body flags for first orbit/landing (new fields on Game)
- **R&D science**: continuous income based on budget setting and exhaustion pool
- **Lab science**: fed from Plan 1.5's Science Lab simulation
- **Tech tree system**: unlock nodes by spending science, gating parts/buildings/recipes
- Parts palette filtered by unlocked tech nodes
- Efficiency upgrade tier purchasing (11 lines × 15 tiers)

**Decisions**: Tech tree UI accessible from the new ColonyManagement mode and from the editor. Default unlocked parts: all PartDefinitions load, but editor palette and colony build menu filter by unlock state.

**References**: colonies.md §8 (Science — all subsections), §10 (Tech Tree — all nodes)

**Critical files**: `src/parts/definition.rs`, `src/editor/ui.rs`, new `src/colony/tech.rs`, `src/render/state.rs`

---

### Plan 2.3: Ship Part Resource Costs

**Scope**: Add material breakdown data to parts, compute costs, display in editor.

**What it covers**:
- Per-category material breakdowns (Metal/HTA/Elec/Super/PI percentages from §7)
- Add to PartDefinition: material cost fields (derived from mass × category percentages)
- Earth cost formula: `Metal×$100 + HTA×$1000 + Elec×$10000 + Super×$50000 + PI×$200000`
- Editor stats bar: show per-stage and total cost alongside delta-v
- Colony construction: consume actual resources from colony inventory instead of money

**Decisions**: Compute material costs at load time from mass × category percentages. "Build at Earth" = money (sum of resource costs × Earth prices). "Build at colony" = consume actual resources from colony inventory.

**References**: colonies.md §7 (Ship Part Resource Costs — all tables and formulas)

**Critical files**: `src/parts/definition.rs`, `src/editor/state.rs`, `src/editor/ui.rs`, data RON files

---

## Layer 3: Colony UI & Management

The player-facing colony management interface. Depends on Layer 1 (simulation) and Layer 2 (economy).

### Plan 3.1: Colony Management UI

**Scope**: The main colony management panel — view and control a colony.

**What it covers**:
- Entry points to `GameMode::ColonyManagement`: button on the main menu, button in the tracking station, and hotkey from flight mode
- Colony overview dashboard: all colonies listed with crew, power balance, food status, alerts
- Per-colony detail view (click a colony): crew, power balance, resource inventory, food supply/demand
- Building list: **aggregated by type and recipe** (e.g., "Metal Smelting × 12: 4,800 kg/day"), with expand to see individual buildings for reassignment. Must scale to 100+ buildings from the start
- Build menu: select new building, see cost, queue for construction
- Mine assignment: pick which resource to mine
- Factory recipe assignment: pick recipe from available (tech-gated) recipes
- Construction queue: ordered list with progress bars
- Greenhouse water management: add/remove water

**Decisions**: Full-screen mode (`GameMode::ColonyManagement`). Summary dashboard showing all colonies with key stats, plus per-colony detail view on click. Keep information density moderate — "hands-off" management, not spreadsheet. Overview → detail drill-down pattern.

**References**: colonies.md §6 (Colony Management Model)

**Critical files**: `src/render/state.rs` (egui UI), possibly new `src/colony/ui.rs`

---

### Plan 3.2: Trade Route & Logistics UI

**Scope**: Create, manage, and automate trade routes between colonies.

**What it covers**:
- Logistics panel: list of all trade routes with status
- Route creation flow (8 steps from colonies.md §8):
  1. Pick endpoints (source/destination colony)
  2. Pick ship blueprint
  3. Choose transfer speed (Hohmann ↔ express slider)
  4. Optional intermediate stops
  5. Set cargo manifest
  6. Set crew (if crewed)
  7. Review cost
  8. Launch timing (immediate or optimal window)
- Route summary display (the formatted route card)
- Fleet panel: all ships, locations, states, assigned routes
- Automation settings: window-based or frequency-based triggers
- Per-route config: cargo manifest, stockpile thresholds, priority

**Decisions**: Lambert solver computes delta-v between any two bodies at departure time. Blueprint validation: ship total delta-v ≥ route delta-v (per-leg for multi-hop). Speed slider: horizontal slider with Hohmann (left) to express (right), showing delta-v and flight time updating in real-time.

**References**: colonies.md §8 (Trade Routes & Logistics — all subsections)

**Critical files**: `src/render/state.rs`, new `src/colony/trade.rs`, `src/colony/logistics.rs`

---

### Plan 3.3: Tech Tree UI

**Scope**: Visual tech tree browser with node graph, unlock interaction, and efficiency tier purchasing.

**What it covers**:
- Visual node graph: 9 eras laid out horizontally, nodes with prerequisite arrows
- Node detail: name, cost, what it unlocks (parts, buildings, recipes), locked/unlocked state
- Research button: spend accumulated science to unlock a node
- Efficiency upgrade panel: 11 tech lines, current tier, cost of next tier, cumulative multiplier
- Integration with editor: part palette only shows unlocked parts
- Integration with colony: building menu only shows unlocked buildings

**Decisions**: Grid layout for node graph — eras as columns, nodes as cards with prerequisite lines drawn between them. Efficiency tiers purchasable from both tech tree UI and colony management UI. Current era highlighted with color/border styling.

**References**: colonies.md §10 (Tech Tree — all 39 nodes + 11 efficiency lines)

**Critical files**: new UI in `src/render/state.rs` or `src/colony/tech_ui.rs`

---

## Layer 4: Trade Route Mechanics

The actual simulation of trade routes — Lambert solvers, ship transit, resource transfers. Depends on Layer 3 (UI exists to create routes).

### Plan 4.1: Lambert Solver & Transfer Computation

**Scope**: Compute delta-v and flight time between any two bodies at any given time.

**What it covers**:
- Lambert solver implementation (or adapt existing patched conics code)
- Hohmann transfer as the minimum-delta-v baseline
- Lambert solutions for faster transfers (player-controlled speed slider)
- Launch window computation: delta-v as a function of departure date, synodic periods
- Surface launch/landing delta-v adjustments:
  - Gravity loss factors (airless 1.1×, thin atm 1.15×, thick atm 1.3×)
  - Landing: airless = full propulsive braking, atmospheric = free (with parachute check)
- Ship delta-v computation from blueprint (reuse editor delta-v calculator)
- Cargo capacity computation: max payload where ship delta-v ≥ route delta-v

**Decisions**: Check if `patched_conics.rs` has reusable orbit math — likely need a new Lambert solver implementation. Approximate accuracy is fine for trade routes (not simulated). Multi-hop: validate per-leg delta-v with refueling assumed at each stop.

**References**: colonies.md §8 (Transfer Mechanics, Multi-Hop Routes)

**Critical files**: `src/ship/orbit.rs`, `src/ship/patched_conics.rs`, new `src/colony/transfer.rs`

---

### Plan 4.2: Fleet & Ship Transit Simulation

**Scope**: Ships in transit between colonies — timers, arrivals, resource transfers.

**What it covers**:
- `TradeShip` struct — blueprint reference, current state (stationed/in-transit/under-construction), location, cargo, crew
- Transit simulation: ship departs, timer counts down, ship arrives
- **Fuel sourcing**: fuel for departure consumed from colony inventory (correct propellant type per blueprint). From Earth: fuel purchased with money at colonies.md §1 prices (RP-1: $1/kg, LOX: $0.50/kg, etc.)
- On arrival: cargo unloaded to destination colony inventory
- Return trip: reload per manifest, refuel from destination colony, depart when ready
- Reusable ships: ship stays at destination, can fly again
- Map display: in-transit ships as icons with progress bars (not rendered 3D)
- Multiple simultaneous ships on different routes

**Decisions**: Trade ships fully separate from flight ships. Trade ships built by spending money (Earth) or resources (colony) from a blueprint. If colony lacks fuel for departure, route pauses with notification.

**References**: colonies.md §8 (Ship Selection, Fleet Management)

**Critical files**: new `src/colony/fleet.rs`, `src/colony/trade.rs`

---

### Plan 4.3: Route Automation

**Scope**: Automatic launch triggers and resource management for trade routes.

**What it covers**:
- Window-based automation: monitor orbital alignment, auto-launch at optimal windows
- Frequency-based automation: launch every N days (same-body routes)
- Delta-v threshold mode: launch when Lambert delta-v drops below player-set value
- Pre-launch validation: fuel available, cargo available, ship stationed
- Pause with notification on validation failure
- Per-route cargo manifests (different outbound vs return)
- Minimum stockpile thresholds (don't ship below reserve level)
- Route priority when multiple routes compete for resources/ships

**Decisions**: Launch window monitoring checked during 1-day batch ticks — at high warp, windows are detected at day-level granularity (sufficient for interplanetary). Automation is opt-in per route. Priority: higher-priority routes get first claim on resources; ties broken by creation order.

**References**: colonies.md §8 (Automation)

**Critical files**: `src/colony/trade.rs`, `src/colony/logistics.rs`

---

## Layer 5: Advanced Colony Features

Particle accelerators, science labs, railguns, and the megastructure pipeline. Depends on Layers 1-4 being complete.

### Plan 5.1: Particle Accelerators

**Scope**: Mk I through Mk IV accelerators for antimatter production.

**What it covers**:
- Four accelerator tiers with stats from colonies.md §4
- Mk I-III: standard buildings with power draw and antimatter output
- Mk IV: planetary ring — scales with body circumference, per-km stats
- Antimatter production: continuous output when powered
- Power requirements: validation that colony has sufficient generation
- Multiple accelerators per body (Mk III, Mk IV)
- Construction time scaling for Mk IV (massive mass → years of robot assembly)

**Decisions**: Mk IV UI shows aggregate stats (total factories, mines, solar farms) not individual buildings. Mk IV bootstrapping is fully simulated — the player builds factories over decades; the detailed analysis in colonies.md is a reference, not a shortcut. Antimatter goes to colony resource inventory like any other resource.

**References**: colonies.md §4 (Particle Accelerators — Mk I through Mk IV + Mercury analysis)

**Critical files**: `src/colony/buildings.rs`, `src/colony/simulation.rs`

---

### Plan 5.2: *(Moved to Plans 1.6 and 2.2 — see those plans)*

---

### Plan 5.3: Railgun (Placeholder)

**Scope**: Add Railgun as a buildable building with no gameplay effect. Future use for orbital megastructure construction (Dyson Spheres, etc.).

**What it covers**:
- Railgun building type in BuildingType enum
- Can be queued and constructed like any building (consumes resources, takes robot time)
- Draws 10 MW power when built
- No functionality — no cargo launches, no trade route integration
- Tooltip/description indicates future megastructure use

**Decisions**: Placeholder only. All trade/manual launches use Launchpads. Railgun megastructure functionality is a separate future plan outside this roadmap.

**References**: colonies.md §4 (Railgun)

**Critical files**: `src/colony/buildings.rs`

---

## Layer 6: Interstellar Extensions

Interstellar colony management across light-year distances. Depends on interstellar ship parts being implemented (separate from this roadmap).

### Plan 6.1: Interstellar Colony Management

**Scope**: Managing colonies in other star systems.

**What it covers**:
- Interstellar colonies function identically to solar system colonies — full real-time management, no observation delay
- **Tech propagation delay**: when a tech node is unlocked, interstellar colonies can only use it after a delay based on their distance from Earth (light-speed communication of research data)
- Precision Instruments bootstrapping: interstellar colonies must reach Electronics Mfg Tier 12 for self-sufficiency
- No trade routes across star systems — interstellar colony ships are one-way supply missions

**Decisions**: Deferred — interstellar colonies are Layer 6, dependent on interstellar ship implementation. Design decisions will be made when that layer is reached.

**References**: colonies.md §9 (Phase 6: Fusion Interstellar), §4 (Precision Instruments challenge)

**Critical files**: `src/colony/mod.rs`, `src/colony/ui.rs`

---

### Plan 6.2: Exoplanet Science & Discovery

**Scope**: Science rewards for exploring other star systems.

**What it covers**:
- Exoplanet discovery: flat 500 orbit + 500 landing = 1,000/body
- Exoplanet labs: same mechanics as solar system labs but with larger extraction pools
- 3 exoplanet systems × ~8 bodies = ~24,000 discovery science + ~120,000 lab science
- Integration with tech tree progression (exoplanet science enables late-game unlocks)

**Decisions**: 10-20 predefined star systems + procedural generation beyond that. Not implemented in this plan — deferred to Layer 6.

**References**: colonies.md §8 (Science — Exoplanets), §9 (Progression Timeline)

**Critical files**: `src/bodies.rs` (extend for multi-system), `src/colony/tech.rs`

---

## Layer 7: Polish & Balance

Final integration, balance tuning, and quality-of-life features.

### Plan 7.1: Notification Inbox & Minor Alerts

**Scope**: Expand the notification system (Plan 1.4 handles critical warp-stopping alerts) with a persistent inbox UI and minor notifications.

**What it covers**:
- Persistent inbox UI: list of all notifications with timestamps, read/unread state, categorization
- **Minor (inbox-only) notifications**: construction complete, trade ship arrival/departure, route automation pause, science milestone, tech node unlock availability, maintenance resource shortage
- Inbox accessible from all modes (icon with unread count)
- Notification history / log

**Decisions**: Minor notifications go to inbox only (no warp stop, no toast). Critical warp-stopping events already handled in Plan 1.4.

**Critical files**: `src/render/state.rs` (UI), `src/game.rs`

---

### Plan 7.2: Balance Pass

**Scope**: Tune all economic values for fun gameplay pacing.

**What it covers**:
- Starting cash ($25M) vs early ship costs vs contract payouts — is early game viable?
- Colony establishment cost chain: Colony Module ($14.8M) + Construction Robot ($51.8M) + supply missions
- Mid-game income vs spending rate
- Tech tree progression speed (does the player unlock nodes at a satisfying pace?)
- Colony self-sufficiency timeline per body
- Resource production/consumption balance across the supply chain
- Particle accelerator progression: is each tier reachable at reasonable timescales?

**Decisions**: Values in colonies.md are starting points. Target progression timeline per colonies.md §9 (Phase 1: ~8yr, Phase 2: ~15yr, etc.). Tune based on playtesting.

**References**: colonies.md §9 (Progression Timeline), all cost tables throughout

---

### Plan 7.3: Map View Colony Visualization

**Scope**: Show colonies and trade routes on the solar system map.

**What it covers**:
- Colony indicators on colonized bodies (icon or glow)
- Trade route lines between bodies (with directionality)
- Ships in transit shown as small icons along routes
- Colony summary tooltip on hover (crew, power, key resources)
- Route summary tooltip on hover (ship name, cargo, ETA)

**Decisions**: Straight lines between bodies for trade routes on the map (orbital paths are too complex for an abstracted system). Integrate with existing body rendering in `render/state.rs` — draw route lines and ship icons in the same render pass as orbits.

**Critical files**: `src/render/state.rs`

---

## Design Flaws & Ambiguities

Issues identified in colonies.md that need resolution before or during implementation.

### 1. Time Warp & Colony Simulation

**Problem**: The game supports time warp up to extreme levels (on-rails at 100x+, potentially 100,000x+). Colony simulation involves complex interactions (recipes, food consumption, power, maintenance) that can't be meaningfully ticked at real-time granularity during high warp.

**Resolution**: Batch ticks with adaptive granularity. Base unit = 1 day. Warp levels range from 100× to 10¹²×. Cap at ~1,000 ticks per frame; batch size scales up at extreme warps (at 10¹²× with 60fps, each tick ≈ 200 days). Factory throughput computed per tick-period. Critical events (starvation, power loss) checked each tick and trigger warp-down notifications — at coarse tick granularity, events may be detected ~200 days late at max warp, which is acceptable for the timescales involved (centuries of interstellar transit).

### 2. Colony UI Access Pattern

**Problem**: Where does the player manage colonies? The game currently has Editor, Flight, and TrackingStation modes. Colony management is a substantial UI surface that doesn't fit cleanly into any existing mode.

**Resolution**: New `GameMode::ColonyManagement` — dedicated full-screen mode. Summary dashboard showing all colonies → drill into per-colony detail. Also includes tech tree access.

### 3. Trade Ship vs Flight Ship Duality

**Problem**: Trade ships are abstract (timers + resource transfers). Flight ships are physical (position, velocity, rendered). Can a player fly a trade ship manually? Can a flight ship become a trade ship?

**Resolution**: Trade ships and flight ships are separate systems. A blueprint can be used for either, but a trade ship instance is never physically simulated and a flight ship is never abstractly traded. The player builds new trade ships from blueprints using money/resources.

### 4. Storage Capacity Model

**Problem**: Colonies have limited storage (Stockpile buildings, 500,000 kg each). What happens when storage is full and a mine produces more ore, or a trade ship arrives with cargo?

**Resolution**: Production pauses when storage is full (output wasted, inputs not consumed). Trade ships wait in orbit until storage is freed. Player notification on storage overflow. Food has separate storage (Habitat food capacity + Food Storage buildings).

### 5. Crew Transfer Between Colonies

**Problem**: colonies.md doesn't mention crew births/growth. Crew must be shipped from Earth or transferred between colonies. How does crew logistics work?

**Resolution**: Crewed trade ships transfer crew. Crew at Earth is unlimited (hired for money). Colony crew is a finite resource that must be transported. Crew without food: 30-day grace period, then crew start dying. Real consequences for neglect.

### 6. Multiple Factories Same Recipe

**Problem**: Multiple factories can run the same recipe. How does the UI handle this? Individually or aggregated?

**Resolution**: Aggregate display by default (e.g., "Metal Smelting × 5: 2,000 kg/day"). Individual building list available for reassignment.

### 7. Recipe Co-location Requirements

**Problem**: Some recipes require a co-located building (Tritium Breeding needs Fission Reactor, NPU Assembly needs Fission Reactor). How is this validated?

**Resolution**: Colony-level check. When assigning a recipe to a Factory, validate that the required co-located building exists and is operational on the same colony. Grey out unavailable recipes with tooltip explanation.

### 8. Solar Power at Gas Giants

**Problem**: Solar power scales with distance from Sun using `(1 AU / distance)²`. For gas giant orbital scooping stations, what distance is used — the planet's semi-major axis?

**Resolution**: Use the body's orbital semi-major axis for the solar power calculation. This is already defined in `bodies.rs`. For moons of gas giants, use the parent body's semi-major axis (moons are close enough to their planet that the sun distance is essentially the same).

### 9. Multiple Mk IV Accelerators Per Body

**Problem**: colonies.md says "Multiple rings can be built per body — concentric rings at different altitudes or parallel rings at different latitudes." How does this work in the colony model?

**Resolution**: Mk IV is just another building that can be built multiple times. Each instance has the full ring stats. The per-km cost model handles scaling. No need for spatial positioning — just count.

### 10. Earth as Special Node

**Problem**: Earth isn't a colony — it's the player's home base. But it needs to participate in the trade route system (sending supplies to colonies, receiving material returns).

**Resolution**: Earth is a special-case "colony" with unlimited resources (purchased with money), unlimited crew (hired), infinite storage. It exists in the colony system but has no buildings, no power, no production — just a trade endpoint. Ship construction at Earth = spend money.

### 11. Atmospheric Body Landing Validation

**Problem**: Atmospheric landings are "free" (aerobraking), but the ship must have a parachute. How is this validated for trade routes?

**Resolution**: When creating a route, inspect the blueprint for parachute parts. If the destination has a significant atmosphere (Earth, Mars, Venus, Titan) and the blueprint lacks a parachute, reject the route with an error message.

### 12. Tech Tier Effects on Production

**Problem**: Building output rates start at Tier 0 base values and scale to 4.78× at Tier 15. But the tech tier system is global (per tech line), not per-building. Does upgrading Mining from Tier 5 to Tier 6 retroactively boost all existing mines?

**Resolution**: Yes — tech tiers are global multipliers. All mines everywhere immediately benefit from a Mining tech tier upgrade. This is the intended design (colonies.md §4: "Each building type has an independent technology line").

### 13. Colony Module Habitability Bypass

**Problem**: The Colony Module's Habitat is provided at base cost (1.0×), bypassing the habitability multiplier. But subsequent Habitats pay the full multiplier. This creates a discontinuity.

**Resolution**: Intentional design — the Colony Module is pre-fabricated for the destination. It's a one-time bootstrap benefit. The multiplier on subsequent Habitats reflects the difficulty of building in-situ on harsh worlds. No change needed.

---

## Dependency Graph

```
Layer 0 (Data Model)
  ├── 0.1 Resources & Economy Types
  ├── 0.2 Colony & Building Types
  ├── 0.3 Tech Tree & Science Types
  └── 0.4 Save System Extension
        │
        ├───────────────────────────────┐
        │                               │
Layer 1 (Core Loop) ← Layer 0    Layer 2 (Economy) ← Layer 0
  ├── 1.1 Colony Establishment      ├── 2.1 Money System
  ├── 1.2 Resource Simulation       ├── 2.2 Science & Tech Tree
  ├── 1.3 Construction Queue        └── 2.3 Ship Part Costs
  ├── 1.4 Critical Notifications
  ├── 1.5 Maintenance System
  └── 1.6 Science Lab Building
        │                               │
        └───────────────┬───────────────┘
                        │
Layer 3 (UI) ← Layers 1 + 2
  ├── 3.1 Colony Management UI
  ├── 3.2 Trade Route UI
  └── 3.3 Tech Tree UI
        │
Layer 4 (Trade Mechanics) ← Layer 3
  ├── 4.1 Lambert Solver
  ├── 4.2 Fleet & Transit
  └── 4.3 Route Automation
        │
Layer 5 (Advanced) ← Layers 1-4
  ├── 5.1 Particle Accelerators
  └── 5.3 Railgun (Placeholder)
        │
Layer 6 (Interstellar) ← Layer 5 + interstellar ships
  ├── 6.1 Interstellar Colony Management
  └── 6.2 Exoplanet Science
        │
Layer 7 (Polish) ← all above
  ├── 7.1 Notification Inbox
  ├── 7.2 Balance Pass
  └── 7.3 Map Visualization
```

---

*All plan scopes reference colonies.md sections. Detailed implementation specs should be written per-plan using the OpenSpec workflow before coding begins.*
