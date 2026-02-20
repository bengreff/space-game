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

Each of the 16 engine IDs SHALL have a unique combination of: combustion chamber size/position, ring count, gimbal actuators, turbopump, and gas generator. IDs: `engine_hummingbird`, `engine_gecko`, `engine_firefly`, `engine_wolf`, `engine_falcon`, `engine_wren`, `engine_owl`, `engine_viper`, `engine_bear`, `engine_eagle`, `engine_panther`, `engine_crane`, `engine_mammoth`, `engine_whale`, `engine_bison`, `engine_titan`.

### Requirement: Unknown engine fallback

Unrecognized engine IDs SHALL render with: default combustion chamber (Y offset `0.75`, width `0.8`, height `0.25`), ring count 8 (first stage) or 4 (upper stage), gimbal actuators only if `gimbal_range > 2.0`.

### Requirement: Engine alpha modulation

All engine detail colors SHALL support alpha modulation: `0.5` for ghost previews, `1.0` for placed parts.

## Pod Rendering

### Requirement: Pod body and window

Pods SHALL render as a dark grey trapezoid body (`[0.15, 0.15, 0.18, 1.0]`) with a white circular window (`[0.9, 0.9, 0.95, 1.0]`) centered horizontally at `y + half_h * 0.2`. Window radius = `min(half_w, half_h) * 0.25`.

### Requirement: Window as filled circle

The pod window SHALL be rendered as a 12-segment triangle fan.

### Requirement: Built-in RCS nozzle bumps

When a pod definition has `rcs: Some(...)`, two small triangular nozzles SHALL protrude from the left and right edges of the pod trapezoid at 80% of the pod height. Each nozzle is a triangle with its base on the pod edge (half-width 0.04m) and tip pointing outward (length 0.08m), matching the style of standalone RCS nozzles. Nozzle X position SHALL be interpolated along the trapezoid edge at that height. Color: `RCS_NOZZLE_COLOR` (`[0.12, 0.12, 0.14, 1.0]`).

## Decoupler Rendering

### Requirement: Decoupler ring band

Decouplers SHALL render as a horizontal band from hitbox bottom upward by visual height. Color: `[0.25, 0.25, 0.28, 1.0]`.

### Requirement: Decoupler adapter trapezoid

A second pass SHALL draw adapter trapezoids connecting decouplers to the closest aligned fuel tank **or command pod** above (same center X within 0.01 tolerance). Bottom edge matches decoupler width, top edge matches tank/pod width.

#### Scenario: Adapter with size transition
- **GIVEN** medium decoupler (2.5m) with small tank (1.5m) above
- **THEN** bottom edge 2.5m, top edge 1.5m (tapered)

#### Scenario: Adapter to command pod
- **GIVEN** decoupler with command pod directly above (same center X)
- **THEN** adapter trapezoid drawn connecting decoupler ring to pod bottom

#### Scenario: No aligned tank or pod above
- **WHEN** no tank or pod shares the decoupler's center X or is above the ring
- **THEN** no adapter drawn

### Requirement: Adapter frustum detail lines

The adapter SHALL render detail lines as frustum projections. Line count = `round(grid_width * 3)`. Each line X = `center_x + half_width * sin(theta)` for evenly-spaced theta in `[-PI/2, PI/2]`. Half-thickness = 0.008 world units. Color: `[0.18, 0.18, 0.20, alpha]`.

### Requirement: Ghost decoupler adapter preview

Ghost previews of decouplers SHALL also render the adapter trapezoid by checking against existing placed parts.

## RCS Thruster Rendering

### Requirement: RCS thin side-mount shape

RCS thrusters SHALL be rendered as thin side-mount parts. Small RCS (RV-1) uses `grid_height: 0.75` for a more compact visual, while medium RCS (RV-3) uses `grid_height: 1.0`. Both share `grid_width: 0.5` and a 1x1 hitbox. The visual sprite SHALL be offset to the side of the hitbox: right-mount (default) offsets to the right side, left-mount (`is_mirrored: true`) offsets to the left side. The offset SHALL be `sign * (hitbox_half_w - visual_half_w)` where sign is 1.0 for right-mount, -1.0 for left-mount.

### Requirement: RCS body rectangle

The RCS body SHALL be a dark grey rectangle (`[0.20, 0.20, 0.22, 1.0]`) covering 80% of the visual extents in both dimensions.

### Requirement: RCS three directional nozzles

Each RCS thruster SHALL render 3 triangular nozzles pointing outward:
- **Lateral nozzle**: Points away from the vessel (left for right-mount, right for left-mount)
- **Top nozzle**: Points upward from the body
- **Bottom nozzle**: Points downward from the body

No nozzle SHALL be rendered on the attachment face (the face against the vessel body). Nozzle color: `[0.12, 0.12, 0.14, 1.0]`.

### Requirement: RCS mirror variants

RCS parts SHALL come in mirror pairs via `mirror_def_id`. Right-mount parts (default, `is_mirrored: false`) have nozzles pointing left/up/down. Left-mount parts (`is_mirrored: true`) have nozzles pointing right/up/down. The `RcsData` struct SHALL include an `is_mirrored: bool` field (serde-default false).

#### Scenario: Right-mount RCS placement
- **WHEN** `rcs_small` is placed (right-mount, `is_mirrored: false`)
- **THEN** the sprite offsets to the right side of the hitbox with nozzles pointing left, up, and down

#### Scenario: Left-mount RCS mirror
- **WHEN** `rcs_small_left` is the mirror variant (`is_mirrored: true`)
- **THEN** the sprite offsets to the left side of the hitbox with nozzles pointing right, up, and down

### Requirement: RCS plume rendering

When RCS nozzles are active during rotation, white plume rectangles (`[0.95, 0.95, 1.0, 0.85]`) SHALL extend outward from each firing nozzle tip. Plume length SHALL be 1.5x the nozzle length, plume width SHALL be 60% of nozzle base width. Plumes SHALL only appear on nozzles whose torque contribution matches the desired rotation direction.

### Requirement: Per-nozzle activation logic

Each RCS nozzle SHALL fire when its torque contribution matches the desired rotation direction OR when translation demands it. The final activation is the union of rotation-driven and translation-driven activations.

**Rotation-driven activation** — torque is computed as the 2D cross product of the part position vector (relative to COM) and the nozzle force direction:
- **Lateral nozzle**: Torque sign = `sign * ry` (where sign is 1.0 for right-mount, -1.0 for left-mount)
- **Up nozzle**: Torque sign = `-rx`
- **Down nozzle**: Torque sign = `rx`
- A nozzle fires when its torque sign matches the desired rotation direction sign.

**Translation-driven activation**:
- **Forward** (`translate[0] > 0`): down nozzles fire on all RCS parts
- **Backward** (`translate[0] < 0`): up nozzles fire on all RCS parts
- **Left** (`translate[1] < 0`): right-mount (non-mirrored) lateral nozzles fire
- **Right** (`translate[1] > 0`): left-mount (mirrored) lateral nozzles fire

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
