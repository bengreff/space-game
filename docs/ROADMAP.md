# Development Roadmap

## Phase Overview

| Phase | Focus | Duration |
|-------|-------|----------|
| 1 | Core Engine | Weeks 1-6 |
| 2 | Playable Foundation | Weeks 7-14 |
| 3 | Parts & Flight | Weeks 15-22 |
| 4 | Extended Content | Weeks 23-32 |
| 5 | Expansion Systems | Weeks 33-44 |

---

## Phase 1: Core Engine (Weeks 1-6)

### Week 1-2: Rendering Foundation
- [x] wgpu + winit boilerplate
- [x] Basic render loop (clear screen, present)
- [x] Draw circles (planets) - using triangle fans + 4x MSAA (not lyon)
- [x] Camera system: pan with mouse drag
- [x] Camera system: zoom with scroll wheel
- [x] Coordinate frame transforms (world to screen)

### Week 3-4: Orbital Mechanics
- [x] Orbital elements struct
- [x] Kepler equation solver (mean → true anomaly)
- [x] Position from orbital elements and time
- [ ] Velocity from orbital elements and time
- [ ] State vectors ↔ orbital elements conversion
- [x] Basic SOI detection

### Week 5-6: Basic Flight
- [ ] Vessel struct with position/velocity
- [ ] Numerical integration (RK4 or similar)
- [ ] Gravity calculation
- [ ] Thrust application
- [ ] Basic time warp (variable timestep)
- [ ] Simple HUD with egui (velocity, altitude)

**Phase 1 Milestone**: A point-mass vessel can orbit a planet, apply thrust, and transfer to another body.

---

## Phase 2: Playable Foundation (Weeks 7-14)

### Week 7-8: Map View
- [ ] Flight view / map view toggle
- [x] Render orbit as ellipse/hyperbola
- [x] Multiple zoom levels (vessel → system scale)
- [x] Click to select/focus celestial body (double-click)
- [x] Camera follows focused body

### Week 9-10: Maneuver Planning
- [ ] Place maneuver node on orbit
- [ ] Maneuver node: prograde/retrograde delta-v
- [ ] Predict trajectory after maneuver burn
- [ ] Execute maneuver (auto-burn or manual)
- [ ] SOI encounter prediction (basic)

### Week 11-12: Vehicle Editor
- [ ] Editor mode vs flight mode
- [ ] Part catalog (load from data files)
- [ ] Grid-based part placement
- [ ] Part attachment validation
- [ ] Delete parts
- [ ] Save blueprint to file
- [ ] Load blueprint from file

### Week 13-14: Core Game Loop
- [ ] Launch from surface position
- [ ] Staging system
- [ ] Fuel consumption
- [ ] Multiple vessels in world
- [ ] Switch between vessels
- [ ] Save/load game state
- [ ] Basic landing detection

**Phase 2 Milestone**: Build a rocket, launch, orbit, land, save, reload.

---

## Phase 3: Parts & Flight (Weeks 15-22)

### Week 15-16: Part System
- [ ] Part definitions from data files
- [ ] Engine properties (thrust, Isp, fuel consumption)
- [ ] Tank properties (resource capacity)
- [ ] Command pod (control source, crew capacity)
- [ ] Decouplers (staging activation)

### Week 17-18: Resource System
- [ ] Resource definitions
- [ ] Resource storage per vessel
- [ ] Fuel flow from tanks to engines
- [ ] Electric charge (generation, consumption)
- [ ] Resource display in UI

### Week 19-20: Celestial Bodies
- [ ] Data-driven body definitions
- [ ] Home system (6-8 bodies)
- [ ] Body hierarchy (moons)
- [ ] Atmosphere basics (drag)
- [ ] Body info panel in UI

### Week 21-22: Flight Polish
- [ ] Navball or heading indicator
- [ ] Throttle control
- [ ] SAS (stability assist, basic)
- [ ] Orbit info display (Ap, Pe)
- [ ] Better landing physics

**Phase 3 Milestone**: Full part variety, functional resource system, complete home system.

---

## Phase 4: Extended Content (Weeks 23-32)

### Week 23-24: Planet Scaling
- [ ] Scale factor config option
- [ ] Apply scale to body radius, distance, gravity
- [ ] Test with 0.1x, 1x, 10x scales
- [ ] Atmosphere scaling

### Week 25-26: Multiple Star Systems
- [ ] Second star system data file
- [ ] Galaxy-level SOI hierarchy
- [ ] Star system selection in map view
- [ ] SOI transitions between systems

### Week 27-28: Interstellar Travel
- [ ] High-Isp engine type
- [ ] Extended time warp levels (100,000x+)
- [ ] Interstellar trajectory display
- [ ] Numerical precision validation at large distances

### Week 29-30: Docking
- [ ] Docking port part
- [ ] Docking detection (proximity, alignment)
- [ ] Vessel merge on dock
- [ ] Vessel split on undock
- [ ] Resource transfer between docked vessels

### Week 31-32: Spaceplanes Foundation
- [ ] Wing parts with lift coefficient
- [ ] Basic lift calculation
- [ ] Control surfaces
- [ ] Wheel parts
- [ ] Horizontal runway spawn

**Phase 4 Milestone**: Travel between star systems, dock vessels, basic spaceplanes.

---

## Phase 5: Expansion Systems (Weeks 33-44)

### Week 33-34: Colonies MVP
- [ ] Colony hub part
- [ ] "Deploy colony" action
- [ ] Colony entity (separate from vessel)
- [ ] Passive ore extraction
- [ ] Ore → Fuel conversion
- [ ] Refuel landed vessels from colony

### Week 35-36: Crew System
- [ ] Crew count on vessels
- [ ] Crew capacity on parts
- [ ] Life support resource
- [ ] Life support consumption
- [ ] Crew death on depletion
- [ ] Crew transfer UI

### Week 37-38: Career Groundwork
- [ ] Tech tree data structure
- [ ] Tech node definitions
- [ ] Part unlock state
- [ ] Funds tracking
- [ ] Unlock UI (basic)

### Week 39-40: Spaceplanes Polish
- [ ] Improved lift model
- [ ] Better wheel physics
- [ ] Runway facilities
- [ ] Spaceplane-specific parts

### Week 41-44: Polish & Content
- [ ] Balance pass (part stats, costs)
- [ ] Additional parts
- [ ] Third star system (optional)
- [ ] Tutorial/help system
- [ ] Settings menu
- [ ] Bug fixes and optimization

**Phase 5 Milestone**: Complete game with colonies, crew, career foundation, spaceplanes.

---

## Current Status

**Phase**: 1 (Core Engine)
**Week**: ~3
**Last Updated**: 2026-02-15

### Completed
- [x] Project scope defined
- [x] Architecture planned
- [x] Documentation structure created
- [x] Rust project initialized
- [x] wgpu + winit boilerplate
- [x] Basic render loop (clear screen, present)
- [x] Draw circles (planets) with triangle fans + MSAA
- [x] Camera system: pan with mouse drag
- [x] Camera system: zoom with scroll wheel
- [x] Coordinate frame transforms (world to screen, camera-relative rendering)
- [x] Orbital elements struct
- [x] Kepler equation solver (mean → true anomaly via Newton-Raphson)
- [x] Position from orbital elements and time
- [x] Basic SOI detection (calculated but not yet used for transitions)
- [x] Full solar system: Sun, 8 planets, Earth's Moon, Mars's moons, Jupiter's 4 Galilean moons, Saturn's 4 largest moons
- [x] Double-click to focus/track celestial body
- [x] Camera follows focused body smoothly
- [x] Render orbit as ellipse (with eccentricity and argument of periapsis)
- [x] Indicator circles for small bodies when zoomed out
- [x] Hover labels using egui
- [x] Conditional orbit line visibility based on body pixel size

### In Progress
- [ ] Velocity from orbital elements and time
- [ ] State vectors ↔ orbital elements conversion

### Next Steps
1. Implement velocity calculation from orbital elements
2. Add vessel struct with position/velocity
3. Add numerical integration (RK4)
4. Add thrust application

---

## Notes

### Definition of Done per Feature
A feature is "done" when:
1. Core functionality works
2. Edge cases don't crash
3. UI for the feature exists (even if basic)
4. Saves/loads correctly

### When to Move On
Move to next phase when:
- Milestone goal achieved
- No critical bugs
- (Features don't need to be polished)
