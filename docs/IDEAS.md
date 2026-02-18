# Game Ideas

## Engine Concepts

### Design Philosophy

Engines are categorized by size (nozzle diameter), fuel type, and purpose. Each engine has trade-offs between thrust, efficiency (ISP), mass, and gimbal range.

**Fuel Types:**
- **LOX/RP-1** - Kerosene, moderate ISP, high density, good for first stages
- **LOX/Methane (CH4)** - Balanced ISP and density, reusable-friendly
- **LOX/Hydrogen (LH2)** - Highest ISP, low density, best for upper stages

**Size Categories (1 grid square = 0.5m):**
- **Tiny** - 0.3-0.5m nozzle, 1 grid square wide (0.5m)
- **Small** - 1-1.5m nozzle, 3 grid squares wide (1.5m)
- **Medium** - 1.5-2.5m nozzle, 5 grid squares wide (2.5m)
- **Large** - 2.5-4.5m nozzle, 9 grid squares wide (4.5m)

---

## TINY Engines (1 grid wide = 0.5m)

*Small engines for probes, satellites, and precision maneuvering*

### Hummingbird (LOX/Methane)
- **Inspired by:** Small pressure-fed methalox concepts
- **Thrust:** 15 kN vac / 12 kN ASL
- **ISP:** 330s vac / 295s ASL
- **Mass:** 25 kg
- **Gimbal:** ±8°
- **Purpose:** Probe main engine, small lander, highly restartable
- **Character:** Efficient and delicate, good control authority for its size

### Gecko (LOX/RP-1)
- **Inspired by:** Rocket Lab Rutherford
- **Thrust:** 28 kN vac / 22 kN ASL
- **ISP:** 315s vac / 280s ASL
- **Mass:** 40 kg
- **Gimbal:** ±4°
- **Purpose:** Small stage clusters, high TWR for size
- **Character:** Designed for clusters with differential throttling, minimal gimbal

### Firefly (LOX/Hydrogen)
- **Inspired by:** Small hydrolox upper stage concepts
- **Thrust:** 8 kN vac / 5 kN ASL
- **ISP:** 445s vac / 360s ASL
- **Mass:** 30 kg
- **Gimbal:** None (fixed)
- **Purpose:** Deep space probes, high delta-V missions
- **Character:** Simple fixed nozzle, relies on RCS for attitude control

### Sparrow (LOX/RP-1)
- **Inspired by:** Throttleable descent engines
- **Thrust:** 20 kN vac / 16 kN ASL
- **ISP:** 305s vac / 270s ASL
- **Mass:** 35 kg
- **Gimbal:** ±15°
- **Purpose:** Small lander propulsion, precision hovering
- **Character:** Extreme gimbal range compensates for single-engine landing

---

## SMALL Engines (3 grid wide = 1.5m)

*Versatile engines for upper stages, small boosters, and medium craft*

### Wolf (LOX/RP-1)
- **Inspired by:** SpaceX Merlin 1D
- **Thrust:** 850 kN vac / 750 kN ASL
- **ISP:** 315s vac / 285s ASL
- **Mass:** 500 kg
- **Gimbal:** ±8°
- **Purpose:** First stage main engine, reusable booster core
- **Character:** Versatile workhorse with good gimbal for landing and ascent

### Falcon (LOX/Methane)
- **Inspired by:** Scaled-down Raptor
- **Thrust:** 600 kN vac / 520 kN ASL
- **ISP:** 355s vac / 320s ASL
- **Mass:** 450 kg
- **Gimbal:** ±12°
- **Purpose:** Reusable upper stage, propulsive landing
- **Character:** Excellent gimbal authority, optimized for powered descent

### Owl (LOX/Hydrogen)
- **Inspired by:** Aerojet RL-10 family
- **Thrust:** 180 kN vac / 110 kN ASL
- **ISP:** 460s vac / 370s ASL
- **Mass:** 280 kg
- **Gimbal:** ±4°
- **Purpose:** Upper stage, interplanetary injection
- **Character:** Expander cycle limits gimbal hardware, prioritizes efficiency

### Viper (LOX/RP-1)
- **Inspired by:** NK-33 (compact powerhouse philosophy)
- **Thrust:** 1100 kN vac / 950 kN ASL
- **ISP:** 325s vac / 295s ASL
- **Mass:** 700 kg
- **Gimbal:** None (fixed)
- **Purpose:** Dense high-thrust first stage, booster clusters
- **Character:** Trades gimbal for extreme thrust density, use in clusters or with fins

---

## MEDIUM Engines (5 grid wide = 2.5m)

*Main engines for medium-lift vehicles and heavy upper stages*

### Bear (LOX/Methane)
- **Inspired by:** Blue Origin BE-4
- **Thrust:** 2200 kN vac / 1900 kN ASL
- **ISP:** 345s vac / 310s ASL
- **Mass:** 1800 kg
- **Gimbal:** ±8°
- **Purpose:** Main first stage engine for medium-heavy lift
- **Character:** Reliable all-rounder with solid gimbal performance

### Eagle (LOX/Hydrogen)
- **Inspired by:** Aerojet RS-25 (scaled down)
- **Thrust:** 1400 kN vac / 1100 kN ASL
- **ISP:** 450s vac / 365s ASL
- **Mass:** 2200 kg
- **Gimbal:** ±10.5°
- **Purpose:** Hydrogen core stage, high-performance second stage
- **Character:** RS-25 heritage gimbal system, excellent control

### Panther (LOX/RP-1)
- **Inspired by:** NPO Energomash RD-180 (single chamber)
- **Thrust:** 2000 kN vac / 1750 kN ASL
- **ISP:** 335s vac / 305s ASL
- **Mass:** 2500 kg
- **Gimbal:** ±6°
- **Purpose:** Heavy-lift booster, high-thrust core
- **Character:** Moderate gimbal, relies on raw power over finesse

### Crane (LOX/Methane)
- **Inspired by:** SpaceX Raptor Vacuum variant
- **Thrust:** 1500 kN vac / 1200 kN ASL (throttles to 20%)
- **ISP:** 365s vac / 325s ASL
- **Mass:** 1400 kg
- **Gimbal:** ±15°
- **Purpose:** Large lander, precision powered descent
- **Character:** Maximum gimbal range for landing, lower thrust but supreme control

---

## LARGE Engines (9 grid wide = 4.5m)

*Heavy-lift engines for super-heavy vehicles and massive payloads*

### Mammoth (LOX/RP-1)
- **Inspired by:** Rocketdyne F-1
- **Thrust:** 7500 kN vac / 6700 kN ASL
- **ISP:** 305s vac / 265s ASL
- **Mass:** 8000 kg
- **Gimbal:** ±6°
- **Purpose:** Super-heavy first stage, moon rocket scale
- **Character:** Size limits gimbal range, but 6° at this thrust is plenty

### Whale (LOX/Hydrogen)
- **Inspired by:** RS-68 / J-2X concepts
- **Thrust:** 3500 kN vac / 2800 kN ASL
- **ISP:** 445s vac / 360s ASL
- **Mass:** 5500 kg
- **Gimbal:** ±8°
- **Purpose:** Heavy-lift hydrogen core, SLS-class vehicles
- **Character:** Good gimbal for hydrogen stage control

### Bison (LOX/Methane)
- **Inspired by:** Super-scaled Raptor concept
- **Thrust:** 5000 kN vac / 4300 kN ASL
- **ISP:** 350s vac / 315s ASL
- **Mass:** 4200 kg
- **Gimbal:** ±12°
- **Purpose:** Reusable super-heavy booster
- **Character:** Wide gimbal for booster landing and recovery

### Titan (LOX/RP-1)
- **Inspired by:** Large vacuum-optimized kerolox (M-1 concepts)
- **Thrust:** 4500 kN vac / 3200 kN ASL
- **ISP:** 340s vac / 290s ASL
- **Mass:** 5000 kg
- **Gimbal:** None (fixed)
- **Purpose:** Large upper stage, trans-lunar injection
- **Character:** Extended vacuum nozzle prevents gimbal, pair with RCS

---

## Summary Table

| Size | Engine | Fuel | Thrust (vac) | ISP (vac) | Mass | TWR | Gimbal |
|------|--------|------|--------------|-----------|------|-----|--------|
| **Tiny** | Hummingbird | CH4 | 15 kN | 330s | 25 kg | 61 | ±8° |
| | Gecko | RP-1 | 28 kN | 315s | 40 kg | 71 | ±4° |
| | Firefly | LH2 | 8 kN | 445s | 30 kg | 27 | None |
| | Sparrow | RP-1 | 20 kN | 305s | 35 kg | 58 | ±15° |
| **Small** | Wolf | RP-1 | 850 kN | 315s | 500 kg | 173 | ±8° |
| | Falcon | CH4 | 600 kN | 355s | 450 kg | 136 | ±12° |
| | Owl | LH2 | 180 kN | 460s | 280 kg | 66 | ±4° |
| | Viper | RP-1 | 1100 kN | 325s | 700 kg | 160 | None |
| **Medium** | Bear | CH4 | 2200 kN | 345s | 1800 kg | 125 | ±8° |
| | Eagle | LH2 | 1400 kN | 450s | 2200 kg | 65 | ±10.5° |
| | Panther | RP-1 | 2000 kN | 335s | 2500 kg | 82 | ±6° |
| | Crane | CH4 | 1500 kN | 365s | 1400 kg | 109 | ±15° |
| **Large** | Mammoth | RP-1 | 7500 kN | 305s | 8000 kg | 96 | ±6° |
| | Whale | LH2 | 3500 kN | 445s | 5500 kg | 65 | ±8° |
| | Bison | CH4 | 5000 kN | 350s | 4200 kg | 121 | ±12° |
| | Titan | RP-1 | 4500 kN | 340s | 5000 kg | 92 | None |

---

## Gimbal Balance Philosophy

**No Gimbal (Fixed) Engines:**
- **Firefly** - Simple deep space probe engine, mass savings, use RCS
- **Viper** - Extreme thrust density trade-off, designed for clusters
- **Titan** - Extended vacuum nozzle physically prevents gimbal

**Low Gimbal (±4-6°):**
- **Gecko** - Cluster-oriented, differential throttle for steering
- **Owl** - Expander cycle packaging constraints
- **Panther** - Brute force over finesse
- **Mammoth** - Physical size limits gimbal actuators

**Standard Gimbal (±8°):**
- **Hummingbird, Wolf, Bear, Whale** - Versatile all-rounders

**High Gimbal (±10-15°):**
- **Sparrow, Falcon, Crane, Bison** - Landing specialists need wide gimbal
- **Eagle** - Heritage RS-25 gimbal system
