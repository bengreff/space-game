# Feature Requirements

## Legend

- **MVP**: Minimum to call feature "done" - implement this first
- **Later**: Post-MVP enhancements - only after MVP works
- **Never**: Explicitly out of scope - do not implement

---

## 1. Vehicle Editor

### MVP
- Grid-snap part placement
- Part palette with ~25 parts
- Delete parts
- Save/load blueprints to file
- Single symmetry mode (horizontal mirror)
- Staging assignment in editor
- Part connection validation (must connect to existing parts)

### Later
- Multiple symmetry modes (radial)
- Subassemblies (save/load partial vessels)
- Part search/filtering
- Undo/redo stack
- Copy/paste parts

### Never
- Free rotation placement
- 3D editor view
- Procedural part generation

---

## 2. Flight & Physics

### MVP
- Thrust vector application (prograde from engine orientation)
- Rotation controls (left/right)
- Fuel consumption based on engine thrust
- Staging activation (spacebar or similar)
- Patched conics orbital mechanics
- SOI transitions (enter/exit planet spheres of influence)
- Time warp: 1x, 5x, 10x, 50x, 100x, 1000x, 10000x
- Basic collision with terrain (vessel stops/explodes)
- Hybrid physics: active when thrust/rotating, on-rails otherwise

### Later
- Thermal system (reentry heating)
- Part stress/breakage
- Hybrid warp (physics warp vs. on-rails warp)
- Quicksave/quickload

### Never
- N-body physics
- Lagrange points
- Tidal forces

---

## 3. Celestial Bodies

### MVP
- Data-driven body definitions (loaded from config files)
- Home system: Star + 6-8 bodies (planets and moons)
- Second star system: Star + 3-4 bodies
- SOI hierarchy (moons orbit planets orbit star)
- Basic atmosphere: exponential density falloff, drag only
- Surface landing zones

### Later
- 3+ star systems
- Atmospheric visual effects (entry flames)
- Terrain heightmaps (non-circular surface)
- Biomes

### Never
- Procedural body generation
- Gas giant "surfaces"
- Tectonic/weather simulation

---

## 4. Planet Scaling

### MVP
- Single global scale factor (config value)
- Range: 0.1x to 10x (affects radius, orbital distance, gravity consistently)
- Applied at game start (not runtime changeable)
- Default: 0.1x (Kerbal-style scaled down)

### Later
- Per-body scaling options
- Preset configurations (Realistic, Stock, Tiny)
- Runtime scale switching (new game only)

### Never
- Runtime dynamic scaling
- Different scales for different systems

---

## 5. Interstellar Travel

### MVP
- High-ISP engine type (fusion/torch drive, Isp > 10000s)
- Extended time warp levels: 100000x, 1000000x
- Interstellar space as "galaxy" SOI (parent of all stars)
- Star system arrival/departure mechanics
- Reasonable travel times (weeks-months at high warp, not years)

### Later
- Multiple drive types (fusion, antimatter, etc.)
- Generation ship mechanics (multi-decade journeys)
- Relativistic velocity display

### Never
- Time dilation gameplay effects
- Relativistic physics
- FTL travel

---

## 6. Colonies

### MVP
- Deployable colony module (single part, landed vessel becomes colony)
- Passive ore extraction (X ore per second when deployed on body)
- Single conversion: Ore → Fuel
- Refuel landed vessels (transfer fuel from colony to vessel)
- Colonies persist when not focused (background simulation)
- Colony list in UI

### Later
- Multiple building/module types
- Colony-to-colony supply routes
- Manufacturing new parts at colonies
- Launch new vessels from colony
- Colony growth/population

### Never
- Real-time base building game
- Complex multi-resource logistics
- Colony combat/defense

---

## 7. Crew System

### MVP
- Crew count: single integer per vessel
- Crew capacity: integer property on command parts
- Life support: crew consumes "life support" resource over time
- Death condition: crew dies if life support reaches zero
- Crew transfer: move crew between docked vessels

### Later
- Named crew with portraits
- Crew skills (pilot, engineer, scientist)
- Experience gain
- Crew hiring/recruitment

### Never
- EVA (extra-vehicular activity)
- Individual crew simulation
- Crew relationships/morale

---

## 8. Spaceplanes

### MVP
- Wing parts: have lift coefficient property
- Lift calculation: lift = velocity² × sin(AoA) × coefficient × air_density
- Control surfaces: provide pitch authority in atmosphere
- Wheels: can roll on ground, support vessel weight
- Runway spawn: horizontal launch option

### Later
- Stall mechanics (lift dropoff at high AoA)
- Steerable nose wheel
- Brakes
- Multiple runway/launchpad locations

### Never
- VTOL complexity
- Realistic subsonic/supersonic transition
- Wing flex/structural dynamics

---

## 9. Career Mode (Groundwork Only)

### MVP
- Tech tree: data structure with nodes and dependencies
- Part unlock state: track which parts are unlocked
- Currency: single "funds" value
- Unlock function: spend funds to unlock tech node
- Persistence: tech state saves/loads with game

### Later
- Full tech tree UI with visualization
- Science collection mechanics
- Contracts system
- Building/facility upgrades
- Reputation system

### Never
- Multiple currencies
- Real-time economy simulation
- Competing space agencies

---

## 10. Parts Suite

### MVP Parts (~25 total)

**Command (2)**
- Capsule (crew capacity: 3)
- Probe core (crew capacity: 0)

**Propulsion (5)**
- Small engine (low thrust, medium Isp)
- Medium engine (medium thrust, medium Isp)
- Large engine (high thrust, low Isp)
- Nuclear engine (low thrust, high Isp)
- Interstellar drive (very low thrust, very high Isp)

**Fuel Tanks (4)**
- Small tank
- Medium tank
- Large tank
- Radial tank

**Structure (4)**
- Decoupler
- Stack separator
- Structural beam
- Adapter (size transition)

**Aero (3)**
- Nose cone
- Wing
- Control surface

**Landing (3)**
- Landing leg
- Wheel
- Parachute

**Utility (4)**
- Solar panel
- Battery
- Antenna
- RCS thruster block

**Colony (2)**
- Colony hub (converts vessel to colony)
- Resource drill

---

## 11. Resources

### MVP Resources

| Resource | Use |
|----------|-----|
| Fuel | Consumed by engines |
| Oxidizer | Consumed with fuel (or combined as single resource) |
| Electric Charge | Consumed by systems, generated by solar |
| Life Support | Consumed by crew over time |
| Ore | Extracted by colonies, converted to fuel |

### Later
- Monopropellant (RCS fuel)
- Specialized resources per body

### Simplification Option
Combine Fuel + Oxidizer into single "Propellant" resource to reduce complexity.

---

## 12. UI Requirements

### MVP
- Flight HUD: velocity, altitude, fuel, throttle
- Navball or heading indicator
- Part info tooltips in editor
- Resource bars
- Staging list
- Time warp indicator
- Basic settings menu

### Later
- Orbital info panel (Ap, Pe, inclination)
- dV calculator
- Alarm clock/reminders
- Custom action groups

---

## 13. Save/Load

### MVP
- Save game state to file
- Load game state from file
- Save includes: vessels, colonies, tech state, currency, game time
- Versioned save format with migration path
- Multiple save slots

### Later
- Autosave
- Quicksave/quickload hotkeys
- Save file browser
