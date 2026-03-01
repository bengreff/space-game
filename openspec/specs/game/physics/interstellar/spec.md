# Interstellar Parts

Interstellar engines, reactors, shields, radiators, and fuel tanks for relativistic spaceflight.

## Part Category

### Requirement: Interstellar category

Interstellar engines and shields SHALL use `category: Interstellar`. Interstellar reactors (Prometheus, Vulcan, Stellarator, Tokamak, Penning, Ixion) use `category: Electricity` — they are power sources and belong with batteries, solar panels, RTGs, and small fission reactors in the Electricity tab. The Interstellar tab appears in the editor parts palette alongside other categories and displays parts in a flat list without size sub-grouping.

### Requirement: Engine rendering for Interstellar category

Engine rendering fallbacks (procedural engine details) SHALL trigger for parts with `engine.is_some()` in both `Propulsion` and `Interstellar` categories. Sprite-based rendering takes priority when sprites are available in the atlas.

### Requirement: Engine sprites in atlas

Interstellar engine sprites SHALL be included in the sprite atlas (not excluded). Sprite filenames in `data/sprites/engines/` match part definition IDs. Per-engine plume sprites in `data/sprites/plumes/` are looked up by stripping the `engine_` prefix from the part ID, with shared plumes for engine families (e.g., `daedalus` for both stage 1 and 2, `zpinch` for both probe and advanced).

## Data Model

### Requirement: Propellant types

`Propellant` enum SHALL include interstellar variants: `FusionFuel` (D+He3), `Antimatter`, `NuclearPulse` (Orion pulse units). Each maps to a corresponding `FuelType` variant.

### Requirement: Fuel types

`FuelType` enum SHALL include interstellar variants with `propellant_per_grid_square()` values:
- `FusionFuel`: (0.0, 30.0) — ~120 kg/m³ cryogenic D+He3, no oxidizer
- `Antimatter`: (0.0, 5.0) — mostly containment mass, milligrams of actual antimatter
- `NuclearPulse`: (0.0, 500.0) — heavy fissile pulse units

Fuel resource names: `fusion_fuel`, `antimatter`, `nuclear_pulse`.

### Requirement: ReactorData

`PartDefinition` SHALL support `reactor: Option<ReactorData>`. `ReactorData { output_watts: f64 }` defines constant power output in Watts. Reactors generate power using the same pattern as RTGs — constant output, no fuel consumption.

### Requirement: ShieldData

`PartDefinition` SHALL support `shield: Option<ShieldData>`. `ShieldData` contains:
- `shield_type: ShieldType` — enum with variants `Whipple`, `FRES`, `Geodesic`
- `max_velocity_c: f64` — maximum rated velocity as fraction of c
- `power_base_watts: f64` — base power consumption (0 for passive Whipple)

## Interstellar Engines

All interstellar engines are vacuum-only (`thrust_asl: 0`, `isp_asl: 0`), `shape: Trapezoid`, `category: Interstellar`, `size: XL`.

### Nuclear Pulse (Tier 1 — Fission)

| ID | Name | Grid | Mass (t) | Thrust (kN) | Isp (s) | Propellant |
|----|------|------|----------|-------------|---------|------------|
| engine_orion_pulse | Orion Pulse Drive | 70x52 | 7000 | 60,000 | 4,079 | NuclearPulse |

### Fusion (Tier 2)

| ID | Name | Grid | Mass (t) | Thrust (kN) | Isp (s) | Propellant |
|----|------|------|----------|-------------|---------|------------|
| engine_daedalus_s1 | Daedalus Stage 1 | 60x48 | 4500 | 7,000 | 1,019,716 | FusionFuel |
| engine_daedalus_s2 | Daedalus Stage 2 | 44x34 | 2000 | 2,000 | 1,019,716 | FusionFuel |
| engine_zpinch_probe | Z-Pinch Probe | 10x18 | 600 | 800 | 815,773 | FusionFuel |
| engine_zpinch_advanced | Z-Pinch Advanced | 16x26 | 2500 | 5,000 | 2,039,432 | FusionFuel |

### Antimatter (Tier 3)

| ID | Name | Grid | Mass (t) | Thrust (kN) | Isp (s) | Propellant |
|----|------|------|----------|-------------|---------|------------|
| engine_amcat_fusion | AM-Cat Fusion | 22x16 | 2000 | 4,000 | 3,059,148 | Antimatter |
| engine_am_torch | Antimatter Torch | 8x12 | 800 | 2,000 | 15,295,740 | Antimatter |
| engine_gamma_conversion | Gamma Converter | 34x26 | 1800 | 1,300 | 26,002,758 | Antimatter |

## Interstellar Reactors

All interstellar reactors are `category: Electricity`, `shape: Rectangle`. Constant power output, no fuel consumption. Listed in the Electricity tab alongside batteries, solar panels, RTGs, and small fission reactors.

| ID | Name | Size | Grid | Mass (t) | Output |
|----|------|------|------|----------|--------|
| reactor_fission_small | Prometheus | Medium | 5x7 | 250 | 500 MW |
| reactor_fission_large | Vulcan | Large | 9x9 | 800 | 1.6 GW |
| reactor_fusion_small | Stellarator | Large | 7x7 | 400 | 10 GW |
| reactor_fusion_large | Tokamak | XL | 11x9 | 1200 | 30 GW |
| reactor_am_small | Penning Reactor | Large | 7x7 | 800 | 800 GW |
| reactor_am_large | Ixion Reactor | XL | 11x9 | 2500 | 2.5 TW |

## Shields

All shields are `category: Interstellar`, `shape: Rectangle`. Three types across three sizes.

**Whipple** — passive debris protection, max 0.1c, no power:
| ID | Size | Grid | Mass (t) |
|----|------|------|----------|
| shield_whipple_small | Large | 9x3 | 300 |
| shield_whipple_medium | XL | 13x5 | 1200 |
| shield_whipple_large | XL | 19x5 | 4800 |

**FRES** — electromagnetic, max 0.85c, requires reactor power:
| ID | Size | Grid | Mass (t) | Power |
|----|------|------|----------|-------|
| shield_fres_small | Large | 9x9 | 300 | 1 GW |
| shield_fres_medium | XL | 13x11 | 750 | 2 GW |
| shield_fres_large | XL | 17x13 | 2400 | 4 GW |

**Geodesic** — force field, max 0.85c, highest power:
| ID | Size | Grid | Mass (t) | Power |
|----|------|------|----------|-------|
| shield_geodesic_small | Large | 9x9 | 800 | 4 GW |
| shield_geodesic_medium | XL | 13x11 | 2000 | 8 GW |
| shield_geodesic_large | XL | 17x13 | 6000 | 16 GW |

## Attachment Diameters

Interstellar parts attach at their top edge. Engines (trapezoid shape) have a narrower top attachment and wider bottom exhaust. Reactors and shields (rectangle shape) have equal top and bottom width. All values in grid squares (1 grid square = 0.5m).

### Engines (Trapezoid)

| Part | Bottom (grid) | Top (grid) | Bottom (m) | Top (m) |
|------|---------------|------------|------------|---------|
| Orion Pulse Drive | 71 | 31 | 35.5 | 15.5 |
| Daedalus Stage 1 | 61 | 21 | 30.5 | 10.5 |
| Daedalus Stage 2 | 45 | 17 | 22.5 | 8.5 |
| Z-Pinch Probe | 11 | 7 | 5.5 | 3.5 |
| Z-Pinch Advanced | 17 | 11 | 8.5 | 5.5 |
| AM-Cat Fusion | 23 | 13 | 11.5 | 6.5 |
| Antimatter Torch | 9 | 7 | 4.5 | 3.5 |
| Gamma Converter | 35 | 15 | 17.5 | 7.5 |

Note: Engine hitbox widths are set to the next odd integer above `grid_width` (for grid-center alignment). Hitbox heights equal `grid_height` (no override needed since all heights are whole numbers).

### Reactors (Rectangle) — category: Electricity

| Part | Width (grid) | Width (m) |
|------|-------------|-----------|
| Prometheus | 5 | 2.5 |
| Vulcan | 9 | 4.5 |
| Stellarator | 7 | 3.5 |
| Tokamak | 11 | 5.5 |
| Penning Reactor | 7 | 3.5 |
| Ixion Reactor | 11 | 5.5 |

### Shields (Rectangle)

| Part | Width (grid) | Width (m) |
|------|-------------|-----------|
| Whipple Shield S | 9 | 4.5 |
| Whipple Shield M | 13 | 6.5 |
| Whipple Shield L | 19 | 9.5 |
| FRES Shield S | 9 | 4.5 |
| FRES Shield M | 13 | 6.5 |
| FRES Shield L | 17 | 8.5 |
| Geodesic Shield S | 9 | 4.5 |
| Geodesic Shield M | 13 | 6.5 |
| Geodesic Shield L | 17 | 8.5 |

## Files

- `src/parts/definition.rs` — ReactorData, ShieldData, ShieldType structs; Propellant/FuelType interstellar variants
- `src/parts/vessel.rs` — Reactor power generation in update_power()
- `src/editor/state.rs` — Reactor power in editor stats calculation
- `data/parts/engines_interstellar.ron` — 8 interstellar engines
- `data/parts/reactors_interstellar.ron` — 6 interstellar reactors
- `data/parts/shields.ron` — 9 shields (3 types × 3 sizes)
