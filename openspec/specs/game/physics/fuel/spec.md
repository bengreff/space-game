# Fuel System

Fuel consumption, fuel zones, and drain priority for asparagus/onion staging.

## Fuel Zones

### Requirement: Zone-based fuel sharing
Fuel is shared across all tanks within a fuel zone. Zones are computed by flood-filling the weld adjacency graph. Non-crossfeed decouplers act as barriers: they are assigned to a zone but don't propagate fuel flow to their neighbors. Crossfeed-enabled decouplers allow fuel to flow through them, merging tanks on both sides into a single zone.

### Requirement: Engine fuel consumption
Engines consume fuel proportionally from all available tanks in their zone. Each engine computes its demand per-frame based on mass flow rate, throttle, and dt. Oxygen and fuel are drained separately. Engines without available fuel are deactivated.

## Drain Priority (Asparagus/Onion Staging)

### Requirement: Priority computation
Each part is assigned a drain priority based on its reachability from the vessel root when crossfeed-enabled decouplers are removed. For each crossfeed-enabled decoupler D in stage S: temporarily remove D from the weld graph, BFS from the vessel root, and parts NOT reachable get `priority = min(existing_priority, S)`. Parts always reachable from root (no crossfeed decoupler between them and root) keep `priority = usize::MAX`.

### Requirement: Priority-based draining
Within each fuel zone, find the minimum priority among tanks that have fuel. Only drain from tanks at that minimum priority level (proportionally among those tanks). When those tanks are empty, the next priority level starts draining. This applies to both oxygen and fuel draining.

### Requirement: Delta-v calculation respects drain priority
The per-stage delta-v calculation uses the same drain priority system. Only fuel from minimum-priority tanks in engine zones counts as burnable fuel for the Tsiolkovsky equation. After computing a stage's delta-v, only the minimum-priority fuel is zeroed before simulating the next stage.

### Requirement: Asparagus staging behavior
In a typical asparagus/onion configuration (center tank + radial crossfeed decouplers + side tanks), side tanks drain first while the center tank stays full. When the radial decouplers fire, the empty side tanks are jettisoned. The center tank then provides fuel for the remaining stages.

## Implementation
- `compute_fuel_zones()`, `compute_drain_priorities()` in `src/parts/vessel.rs`
- `compute_fuel_zones_simulated()`, `compute_drain_priorities_simulated()` for delta-v calculation
- `consume_fuel()` Phase 3 uses priorities for proportional draining
- `calculate_stage_delta_v()` steps 5 and 8 use priorities for burnable fuel computation
