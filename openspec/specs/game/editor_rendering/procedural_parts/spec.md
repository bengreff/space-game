# Procedural Parts

Procedural rendering of engines, pods, and decouplers with per-engine visual identity, adapter trapezoids, and exhaust plumes.

## Engine Rendering

### Requirement: Engine nozzle bell base shape

Engines SHALL be rendered as a trapezoid (nozzle bell) using the part's full width at the bottom and `top_width` at the top. The nozzle color SHALL depend on whether the engine is a "first stage" engine (ASL thrust > 70% of vacuum thrust): first stage engines use dark nozzle color `[0.08, 0.08, 0.10, 1.0]`, upper stage engines use lighter nozzle color `[0.15, 0.15, 0.17, 1.0]`.

#### Scenario: First stage engine detection
- **WHEN** an engine has `thrust_asl > thrust_vac * 0.7`
- **THEN** it SHALL be classified as a first stage engine and use the dark nozzle color

#### Scenario: Upper stage engine detection
- **WHEN** an engine has `thrust_asl <= thrust_vac * 0.7`
- **THEN** it SHALL be classified as an upper stage engine and use the lighter nozzle color

### Requirement: Engine combustion chamber

Each engine SHALL render a rectangular combustion chamber near the top of the nozzle. The chamber position and size vary per engine ID, expressed as fractions of the nozzle's half-dimensions. The chamber color SHALL be `[0.18, 0.18, 0.20, 1.0]`.

### Requirement: Nozzle cooling rings

Each engine SHALL render horizontal ring bands across the nozzle. Rings are evenly distributed from the bottom upward (covering the lower 70% of the nozzle height). Ring count varies per engine ID (2 to 14). Ring width at each Y position SHALL be interpolated between bottom and top width. Ring thickness SHALL be `half_h * 0.04`. Ring color: dark `[0.14, 0.14, 0.16, 1.0]` for first stage, light `[0.22, 0.22, 0.24, 1.0]` for upper stage.

#### Scenario: Ring interpolation
- **GIVEN** `half_w = 1.0` at bottom and `half_top_w = 0.5` at top
- **WHEN** ring is at height fraction `t = 0.35`
- **THEN** ring width = `half_w + (half_top_w - half_w) * t = 0.825`

### Requirement: Gimbal actuator brackets

Engines with gimbal range > 2.0 degrees MAY render gimbal actuator brackets as angled triangles on left and right sides of the nozzle, extending outward by `half_w * 0.3`. Per-engine-ID.

### Requirement: Turbopump housing

Some engines SHALL render a rectangular turbopump box near the top of the engine. Color: `[0.20, 0.20, 0.22, 1.0]`. Per-engine-ID.

### Requirement: Gas generator box

Some engines SHALL render a rectangular gas generator box on one side (offset `half_top_w * 0.85-0.9`). Color: `[0.16, 0.16, 0.18, 1.0]`. Per-engine-ID.

### Requirement: Per-engine visual identity

Each of the 16 engine IDs SHALL have a unique combination of: combustion chamber size/position, ring count, gimbal actuators, turbopump, and gas generator. IDs: `engine_hummingbird`, `engine_gecko`, `engine_firefly`, `engine_sparrow`, `engine_wolf`, `engine_falcon`, `engine_owl`, `engine_viper`, `engine_bear`, `engine_eagle`, `engine_panther`, `engine_crane`, `engine_mammoth`, `engine_whale`, `engine_bison`, `engine_titan`.

### Requirement: Unknown engine fallback

Unrecognized engine IDs SHALL render with: default combustion chamber (Y offset `0.75`, width `0.8`, height `0.25`), ring count 8 (first stage) or 4 (upper stage), gimbal actuators only if `gimbal_range > 2.0`.

### Requirement: Engine alpha modulation

All engine detail colors SHALL support alpha modulation: `0.5` for ghost previews, `1.0` for placed parts.

## Pod Rendering

### Requirement: Pod body and window

Pods SHALL render as a dark grey trapezoid body (`[0.15, 0.15, 0.18, 1.0]`) with a white circular window (`[0.9, 0.9, 0.95, 1.0]`) centered horizontally at `y + half_h * 0.2`. Window radius = `min(half_w, half_h) * 0.25`.

### Requirement: Window as filled circle

The pod window SHALL be rendered as a 12-segment triangle fan.

## Decoupler Rendering

### Requirement: Decoupler ring band

Decouplers SHALL render as a horizontal band from hitbox bottom upward by visual height. Color: `[0.25, 0.25, 0.28, 1.0]`.

### Requirement: Decoupler adapter trapezoid

A second pass SHALL draw adapter trapezoids connecting decouplers to the closest aligned fuel tank above (same center X within 0.01 tolerance). Bottom edge matches decoupler width, top edge matches tank width.

#### Scenario: Adapter with size transition
- **GIVEN** medium decoupler (2.5m) with small tank (1.5m) above
- **THEN** bottom edge 2.5m, top edge 1.5m (tapered)

#### Scenario: No aligned tank above
- **WHEN** no tank shares the decoupler's center X or is above the ring
- **THEN** no adapter drawn

### Requirement: Adapter frustum detail lines

The adapter SHALL render detail lines as frustum projections. Line count = `round(grid_width * 3)`. Each line X = `center_x + half_width * sin(theta)` for evenly-spaced theta in `[-PI/2, PI/2]`. Half-thickness = 0.008 world units. Color: `[0.18, 0.18, 0.20, alpha]`.

### Requirement: Ghost decoupler adapter preview

Ghost previews of decouplers SHALL also render the adapter trapezoid by checking against existing placed parts.

## Highlight Overlays

### Requirement: Procedural part selection/hover overlay

For procedural parts, highlighting SHALL be a semi-transparent overlay on top:

| State        | Overlay Color             |
|-------------|--------------------------|
| Selected    | `[0.5, 0.7, 1.0, 0.3]`  |
| Hovered     | `[0.55, 0.55, 0.6, 0.2]` |
| Invalid drag | `[0.9, 0.2, 0.2, 0.4]`  |

### Requirement: Ghost validity overlay for procedural parts

Ghost previews SHALL render procedural details at `alpha = 0.5`, then overlay green `[0.3, 0.9, 0.3, 0.25]` or red `[0.9, 0.3, 0.3, 0.25]` based on validity.

## Exhaust Plume

### Requirement: Engine exhaust plume geometry

`generate_engine_plume_vertices` SHALL draw two nested triangles from the nozzle exit. Outer red (`[1.0, 0.2, 0.0, 0.9]`): full width, length = `diameter * 2.0 * throttle`. Inner yellow (`[1.0, 0.9, 0.1, 1.0]`): 60% width, 40% length. No plume when throttle = 0.0.

#### Scenario: Full throttle plume
- **GIVEN** engine width 1.5m at throttle 1.0
- **THEN** red plume 3.0m long, yellow plume 0.9m wide and 1.2m long

#### Scenario: Partial throttle
- **GIVEN** throttle 0.5
- **THEN** plume length halved
