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

---

## Interstellar Systems

### Design Philosophy

Interstellar-scale systems span three technology tiers: **fission** (Orion-era, ~0.1c), **fusion** (Daedalus/Z-Pinch era, ~0.3c), and **antimatter** (Type 2 civilization, 0.5–0.85c). Each tier has engines, reactors, shields, and radiators. The design bottleneck shifts at each tier: fission is mass-constrained, fusion is thermally-constrained, antimatter is fuel-logistics-constrained.

---

### Interstellar Engines

| Engine | Fuel | Ve (km/s) | Thrust (MN) | Fuel Flow (kg/s) | Engine Mass (t) | Total Reaction Power | Dir. Eff. | Waste Heat |
|---|---|---|---|---|---|---|---|---|
| Orion Pulse | U-235/Pu-239 + Li-6D | 40 | 60 | 1,500 | 7,000 | 3 TW | 98% | 60 GW |
| Daedalus – Stage 1 | D + He-3 (1:1) | 10,000 | 7 | 0.7 | 4,000 | 35 TW | 97% | 1.05 TW |
| Daedalus – Stage 2 | D + He-3 (1:1) | 10,000 | 2 | 0.2 | 1,800 | 10 TW | 97.5% | 250 GW |
| Z-Pinch (Probe) | D + He-3 (1:1) | 8,000 | 0.8 | 0.1 | 900 | 4 TW | 96% | 160 GW |
| Z-Pinch (Advanced p-B11) | p + B-11 (1:11) | 20,000 | 5 | 0.25 | 3,500 | 25 TW | 98% | 500 GW |
| AM-Cat Fusion | D + He-3 + μg anti-H | 30,000 | 4 | 0.133 | 2,500 | 60 TW | 99.5% | 300 GW |
| Antimatter Torch (0.5c) | H + anti-H (1:1) | 150,000 | 2 | 0.0133 | 1,200 | 300 TW | 99.9% | 300 GW |
| Gamma Conversion (0.85c) | H + anti-H (1:1) + pair stage | 255,000 | 1.3 | 0.0051 | 1,800 | 300 TW | 99.95% | 150 GW |

### Interstellar Engine Sprite Dimensions

Interstellar engines are 60–90× denser per grid square than chemical engines (5,000–9,000 kg/grid-sq vs 50–130 kg/grid-sq). This reflects reactor cores, superconducting magnets, and radiation shielding vs thin-walled aluminum nozzle bells.

| Engine | Mass (t) | Width (grid) | Height (grid) | Width (m) | Height (m) | Top Width (grid) | Density (kg/sq) |
|---|---|---|---|---|---|---|---|
| Z-Pinch (Probe) | 900 | 11 | 15 | 5.5 | 7.5 | 7 | 5,455 |
| Antimatter Torch | 1,200 | 12 | 16 | 6.0 | 8.0 | 7 | 6,250 |
| Daedalus S2 | 1,800 | 14 | 19 | 7.0 | 9.5 | 9 | 6,767 |
| Gamma Conversion | 1,800 | 13 | 20 | 6.5 | 10.0 | 7 | 6,923 |
| AM-Cat Fusion | 2,500 | 16 | 21 | 8.0 | 10.5 | 10 | 7,440 |
| Z-Pinch (Advanced) | 3,500 | 18 | 24 | 9.0 | 12.0 | 12 | 8,102 |
| Daedalus S1 | 4,000 | 20 | 26 | 10.0 | 13.0 | 12 | 7,692 |
| Orion Pulse | 7,000 | 27 | 30 | 13.5 | 15.0 | 14 | 8,642 |

**Visual shape concepts:**
- **Orion Pulse** (27×30, top 14): Anvil-shaped. Massive pusher plate at bottom (full width), shock absorber column above, narrower magazine section, mount adapter at top. Unique silhouette — wider at bottom than top.
- **Daedalus S1/S2** (20×26 / 14×19): Parabolic magnetic nozzle bell with copper toroidal coils. Bulging reactor section above the nozzle.
- **Z-Pinch Probe/Advanced** (11×15 / 18×24): Cylindrical body with prominent Z-pinch coil rings along the full length. Short flared nozzle. Industrial, utilitarian.
- **AM-Cat Fusion** (16×21): Two-section silhouette. Fusion reactor bulge with antimatter injection collar, feeding into moderate bell nozzle.
- **Antimatter Torch** (12×16): Sleek, narrow, nearly cylindrical. Magnetic confinement rings. Minimal bell flare.
- **Gamma Conversion** (13×20): Three distinct segmented sections — annihilation reactor, pair production chamber, gamma reflector array. Most exotic appearance.

**Engine waste heat notes:**
- Fission/nuclear pulse (2–4% of total): Neutron radiation scatters broadly, bulky shielding absorbs significant heat.
- Fusion (2.5–3%): Magnetic confinement directs most charged particles into exhaust, but neutrons and bremsstrahlung leak.
- Antimatter (0.05–0.1%): Annihilation products are charged and magnetically directable. Very little leaks into structure.

---

### Power Generation

Reactors are separate from engines. Each tier's output is sized to match FRES shield power requirements at the cruise speed that tier enables.

| Reactor | Power Output | Waste Heat | Mass | Fuel | Fuel Rate (100%) | Conversion Eff. | Specific Power |
|---|---|---|---|---|---|---|---|
| Fission (Small) | 5 GW | 7.5 GW | 250 t | U-235 | 0.56 kg/hr | 40% | 20 MW/t |
| Fission (Large) | 20 GW | 30 GW | 800 t | U-235 | 2.25 kg/hr | 40% | 25 MW/t |
| Fusion (Small) | 100 GW | 43 GW | 400 t | D + He-3 | 1.5 kg/hr | 70% | 250 MW/t |
| Fusion (Large) | 500 GW | 214 GW | 1,200 t | D + He-3 | 7.3 kg/hr | 70% | 417 MW/t |
| Antimatter (Small) | 2 TW | 350 GW | 800 t | H + anti-H | 94 g/hr | 85% | 2.5 GW/t |
| Antimatter (Large) | 8 TW | 1.4 TW | 2,500 t | H + anti-H | 376 g/hr | 85% | 3.2 GW/t |

**Reactor–shield pairings:**
- Fission (5–20 GW): Powers FRES at 0.1c (small FRES = 3 GW, large = 12 GW)
- Fusion (100–500 GW): Powers FRES at 0.3c (small = 81 GW, large = 324 GW) or small FRES at 0.5c (375 GW)
- Antimatter (2–8 TW): Powers FRES at 0.85c (small = 1.84 TW, large = 7.36 TW)

**Physics basis:**
- Fission (40% eff.): Advanced gas-core fission with Brayton cycle. U-235 releases ~80 TJ/kg.
- Fusion (70% eff.): D+He-3 ICF or magnetic confinement with direct electrostatic conversion. D-He3 releases ~350 TJ/kg. Aneutronic reaction enables direct charged-particle capture.
- Antimatter (85% eff.): Controlled p-antiproton annihilation with magnetic nozzle capture of charged pion/muon decay products. 90 PJ/kg of total reactant. Fuel rate splits 50/50 matter/antimatter.

---

### Shielding Systems

| Shield | Size | Diameter | Mass | Rated Max Velocity | Power @0.1c | @0.3c | @0.5c | @0.85c |
|---|---|---|---|---|---|---|---|---|
| Passive Whipple | Small | 50 m | 300 t | 0.1c | 0 | — | — | — |
| Passive Whipple | Medium | 100 m | 1,200 t | 0.1c | 0 | — | — | — |
| Passive Whipple | Large | 200 m | 4,800 t | 0.1c | 0 | — | — | — |
| Active FRES | Small | 50 m | 300 t | 0.85c (power-limited) | 3 GW | 81 GW | 375 GW | 1.84 TW |
| Active FRES | Medium | 100 m | 750 t | 0.85c (power-limited) | 6 GW | 162 GW | 750 GW | 3.68 TW |
| Active FRES | Large | 200 m | 2,400 t | 0.85c (power-limited) | 12 GW | 324 GW | 1.5 TW | 7.36 TW |
| Geodesic Deflector | Small | 50 m | 800 t | 0.85c | 50 GW | 58 GW | 65 GW | 78 GW |
| Geodesic Deflector | Medium | 100 m | 2,000 t | 0.85c | 100 GW | 116 GW | 130 GW | 156 GW |
| Geodesic Deflector | Large | 200 m | 6,000 t | 0.85c | 200 GW | 232 GW | 260 GW | 312 GW |

**Shield types:**
- **Passive Whipple:** Physical multi-layer barrier. Zero power, heavy, 0.1c max. Ablative protection only.
- **Active FRES (Field-Reinforced Electromagnetic Shield):** Electromagnetic particle deflection. Power scales as v³ (flux × momentum). Light but power-hungry at high speeds.
- **Geodesic Deflector:** Exotic-matter toroidal ring generates controlled spacetime curvature. Particles follow curved geodesics around the ship. Power is nearly flat with velocity (GR deflection angle is speed-independent; small linear correction for flux stability). Deflects everything universally — charged particles, neutral atoms, dust, even photons. Requires exotic matter only producible by antimatter-scale industrial processes. 2.5–3× heavier than FRES but ~24× more power-efficient at 0.85c. Crossover vs FRES at ~0.25c.

---

### Radiator Systems

Power radiated per m² (double-sided): Q = 2εσT⁴. Doubling operating temperature gives 16× rejection per unit area.

| Radiator | Tech Era | Operating Temp | Emissivity | Mass | Heat Rejected | Specific Rejection |
|---|---|---|---|---|---|---|
| Heat Pipe Panel | Fission | 1,200 K | 0.92 | 8 kg/m² | 216 kW/m² | 27 kW/kg |
| Liquid Metal Droplet | Fusion | 2,500 K | 0.95 | 3 kg/m² | 4.2 MW/m² | 1.4 MW/kg |
| Phononic Metamaterial | Antimatter | 6,000 K | 0.99 | 1.5 kg/m² | 145 MW/m² | 97 MW/kg |

Each tier is ~20× better specific rejection than the last.

**Radiator technologies:**
- **Heat Pipe Panel:** Sodium/lithium heat pipes in carbon-carbon composite sheets. Limited by working fluid and carbon sublimation point.
- **Liquid Metal Droplet Array:** Tin or lithium droplets sprayed into vacuum, radiating freely, magnetically recollected. No solid structure at the radiating surface eliminates melting point limits.
- **Phononic Metamaterial Emitter:** Engineered atomic lattices channel thermal phonons into coherent radiative emission. Strong-force-scale bonding maintains structure at stellar photosphere temperatures. Self-repairing lattice.

**Radiator burden by ship class:**

| Ship | Total Waste Heat | Radiator Tier | Area Needed | Radiator Mass | % of Dry Mass |
|---|---|---|---|---|---|
| Orion + Large Fission | 90 GW | Heat Pipe | 417,000 m² | 3,333 t | ~18% |
| Daedalus S1 + Large Fusion | 1,264 GW | Liquid Droplet | 301,000 m² | 903 t | ~13% |
| AM Torch + Small AM Reactor | 650 GW | Phononic | 4,500 m² | 6.7 t | <0.3% |

---

### Endgame Ship Design Pattern

The optimal high-speed interstellar ship uses two operating modes:

1. **Burn phase:** Antimatter engine + antimatter reactor + FRES (maximum power, expensive fuel, short duration)
2. **Cruise phase:** Engine off, geodesic deflector active, fusion reactor (low power, cheap fuel, years-long duration)

Example — Alpha Centauri sprint at 0.85c (5.5-year cruise):
- **FRES + AM reactor for shield:** 1.84 TW continuous → 47 g/hr antimatter → ~2.3 tonnes antimatter for shield alone
- **Geodesic deflector + fusion reactor:** 78 GW + reactor overhead → 7.3 kg/hr D+He-3 → ~352 tonnes D+He-3, zero antimatter during cruise

The antimatter budget becomes acceleration/deceleration only. Cruise is fusion-powered
