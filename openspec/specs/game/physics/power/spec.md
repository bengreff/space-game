# Power System Spec

## Overview

Electricity system providing power generation, storage, and consumption. Electricity is stored in Watt-hours (Wh) and generated/consumed in Watts (W).

## Data Model

### Dedicated Electricity Fields

Electricity uses dedicated fields on `FlightPart` (`electricity` / `max_electricity`) rather than the `resources` HashMap. This prevents Wh values from being summed as kg in mass calculations (~10 sites sum `resources.values()` as kilograms).

### Part Definition Structs

- `BatteryData { capacity_wh: f64 }` — storage capacity in Wh
- `SolarPanelData { output_1au: f64 }` — power output in Watts at 1 AU from the Sun
- `RtgData { output_watts: f64 }` — constant power output in Watts
- `EngineData.alternator_power: f64` — Watts generated when engine is running (default 0)
- `PodData.power_draw: f64` — Watts consumed by command pod (default 0)

### Part Definitions

**Batteries** (Category: Utility, Shape: Rectangle, 1 grid tall):
| ID | Size | Grid | Mass (t) | Capacity (Wh) |
|----|------|------|----------|---------------|
| battery_z1 | Tiny | 1x1 | 0.025 | 5,000 |
| battery_z3 | Small | 3x1 | 0.075 | 15,000 |
| battery_z5 | Medium | 5x1 | 0.125 | 25,000 |
| battery_z9 | Large | 9x1 | 0.225 | 45,000 |
| battery_z13 | XL | 13x1 | 0.325 | 65,000 |

**Solar Panels** (Category: Utility, 150 W per grid square at 1 AU):
| ID | Size | Grid | Mass (t) | Output @1AU (W) |
|----|------|------|----------|----------------|
| solar_sp3 | Tiny | 1x3 | 0.015 | 450 |
| solar_sp6 | Tiny | 1x6 | 0.03 | 900 |
| solar_sp12 | Small | 2x6 | 0.06 | 1,800 |
| solar_sp24 | Small | 2x12 | 0.12 | 3,600 |

**RTG** (Category: Utility, Size: Tiny):
| ID | Grid | Mass (t) | Output (W) |
|----|------|----------|-----------|
| rtg_pbnuk | 1x2 | 0.08 | 300 |

**Engine Alternators** (added to existing engines):
- Tiny engines: 50 W
- Small engines: 100 W
- Medium engines: 150 W
- Large engines: 200 W

**Pod Power Draw** (added to existing pods):
- Small Pod: 200 W
- Medium Pod: 500 W

## Flight Behavior

### Power Update (`FlightVessel::update_power`)

Called each physics frame after fuel consumption and mass recalculation.

**Generation sources** (summed across non-decoupled, non-destroyed parts):
1. Solar panels: `output_1au * (AU / sun_distance)^2` — inverse-square law
2. RTGs: constant `output_watts`
3. Engine alternators: `alternator_power` when `engine_active == true`

**Consumption sources**:
1. Pod power draw: `power_draw` from all active pods

**Net energy calculation**:
- `net_wh = (generation - consumption) * dt / 3600`
- Distributed proportionally across battery parts by capacity fraction
- Each battery clamped to `[0, max_electricity]`

### Sun Distance Computation

Ship absolute position computed via `ship.absolute_position()`. Sun is body index 1. Distance is Euclidean from ship to Sun position.

## Part Info Panels

### Flight Part Info
- **Battery**: Shows capacity and a progress bar with "current / max Wh"
- **Solar Panel**: Shows current output in Watts (distance-adjusted)
- **RTG**: Shows constant output in Watts

### Editor Part Info
- **Battery**: Shows capacity in Wh
- **Solar Panel**: Shows output at Earth distance (1 AU) in Watts
- **RTG**: Shows constant output in Watts

## Editor Display

`ShipStats` includes:
- `electricity_capacity`: total battery Wh
- `power_generation`: total solar + RTG watts
- `power_consumption`: total pod watts

Displayed in stats bar: "EC: X Wh" and "Power: +XW / -YW"

## Flight HUD

ELEC bar displayed between FUEL and HEAT bars when vessel has batteries:
- Gold/yellow fill at >30% charge
- Orange fill at 10-30%
- Red fill at <10%
- Shows stored Wh (e.g. "5.0k Wh") and "+Xw / -Yw" generation/consumption text

## Sprite Generation

Three generator functions in `generate_parts.py`:
- `generate_battery()`: Black rectangle with cell division lines, terminal indicators
- `generate_solar_panel()`: Silver frame with blue photovoltaic cell grid, hinge detail
- `generate_rtg()`: Dark grey body with red/orange heat dissipation fin stripes

## Files

- `data/parts/electrical.ron` — 10 electrical part definitions (5 batteries, 4 solar panels, 1 RTG)
- `data/parts/engines.ron` — alternator_power added to all 16 engines
- `data/parts/pods.ron` — power_draw added to both pods
- `src/parts/definition.rs` — BatteryData, SolarPanelData, RtgData structs
- `src/parts/vessel.rs` — FlightPart electricity fields, update_power(), helpers
- `src/render/types.rs` — ShipRenderData power fields
- `src/render/state.rs` — ELEC bar in flight HUD
- `src/editor/state.rs` — ShipStats power fields, calculate_stats() updates
- `src/editor/ui.rs` — Power stats in editor stats bar
- `src/main.rs` — update_power() call in game loop, ShipRenderData wiring
- `tools/sprite_gen/generate_parts.py` — Battery, solar panel, RTG sprite generators
