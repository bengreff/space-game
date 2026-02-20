# Ship Rendering

Ship triangle indicator, part-based rendering in ship view, exhaust plumes, and orbit lines.

## Ship Indicator

### Requirement: Ship triangle indicator in map view

When the ship is smaller than 5 pixels on screen, a fixed-size triangle indicator SHALL be drawn at 16 pixels screen radius, pointing in the direction of ship rotation.

#### Scenario: Indicator triangle geometry
- Nose vertex at angle R from center, distance = `indicator_size`
- Back_left vertex at angle `R + PI * 0.8`, distance = `indicator_size * 0.6`
- Back_right vertex at angle `R - PI * 0.8`, distance = `indicator_size * 0.6`
- `indicator_size = 16.0 / pixels_per_world_unit`

#### Scenario: Indicator outline effect
- Outer triangle at full indicator_size with ship color
- Inner triangle at `indicator_size * 0.6` with darkened color `[color[0]*0.3, color[1]*0.3, color[2]*0.3, color[3]]`

### Requirement: Ship rendered at actual world size when partially visible

When `ship_pixels >= 1.0` and `ship_pixels < 5.0`, the ship SHALL be rendered at actual world scale AND the fixed-size indicator SHALL also be drawn on top.

### Requirement: Ship default color and size

Default ship color SHALL be `[1.0, 0.2, 0.2, 1.0]` (bright red). Default ship size (no vessel loaded) SHALL be `SHIP_SIZE = 10.0` meters.

## Part-Based Rendering

### Requirement: Part-based rendering in ship view

When the ship has parts and part definitions are available and `ship_pixels >= 1.0`, individual parts SHALL be rendered at their positions relative to the vessel center of mass.

#### Scenario: Part vertex transformation pipeline
1. Scale local position by `render_scale` (= SCALE * BODY_SCALE = 1e-9)
2. Rotate by `visual_rotation = ship.rotation - PI/2` (editor Y-up maps to physics rotation where 0 = +X)
3. Translate to vessel world position relative to camera

#### Scenario: Engine gimbal rotation
- After scaling but before vessel rotation, rotate engine part vertices by gimbal_angle in part-local space

### Requirement: Ship fallback triangle when no vessel parts

When no vessel parts are available, the ship SHALL be rendered as a simple triangle at actual world size using the same nose/back geometry as the indicator.

### Requirement: Decoupler adapter fairings in second pass

Decoupler adapter fairings SHALL be rendered in a separate pass after all base parts for proper layering.

## Exhaust Plumes

### Requirement: Engine exhaust plumes during thrust

When an engine is active and throttle > 0, an exhaust plume SHALL be rendered below the engine nozzle.

#### Scenario: Plume geometry
- `plume_length = nozzle_width * 2.0 * throttle`
- Nozzle exit at `y - half_height`

#### Scenario: Outer plume
- Red triangle `[1.0, 0.2, 0.0, 0.9]`, full nozzle width, full plume length

#### Scenario: Inner plume
- Yellow triangle `[1.0, 0.9, 0.1, 1.0]`, 60% width, 40% length

## RCS Plumes

### Requirement: RCS plume rendering in flight

When RCS nozzles are active (vessel is rotating or translating via manual input or autopilot), white plume rectangles SHALL be rendered extending outward from each firing nozzle. Plumes are generated in part-local space at origin (0,0) and transformed with the rest of the part vertices (scale, gimbal rotation if applicable, vessel rotation).

#### Scenario: RCS plume activation from rotation
- **WHEN** the player presses Q (rotate left, `rcs_direction = 1.0`) and an RCS part is above the center of mass
- **THEN** nozzles whose torque matches the positive rotation direction SHALL show white plumes

#### Scenario: RCS plume activation from translation
- **WHEN** the player presses W (translate forward, `rcs_translate[0] > 0`)
- **THEN** down nozzles on all RCS parts SHALL fire (push ship forward=up)
- **WHEN** S pressed (translate backward, `rcs_translate[0] < 0`): up nozzles fire
- **WHEN** D pressed (translate right, `rcs_translate[1] > 0`): right-mount lateral nozzles fire (exhaust left, push right)
- **WHEN** A pressed (translate left, `rcs_translate[1] < 0`): left-mount lateral nozzles fire (exhaust right, push left)

#### Scenario: Combined rotation and translation
- **WHEN** both rotation and translation inputs are active simultaneously
- **THEN** nozzle activation SHALL be the union of rotation-driven and translation-driven activations

#### Scenario: No plumes when idle
- **WHEN** `rcs_direction = 0.0` AND `rcs_translate = [0.0, 0.0]` (no RCS input)
- **THEN** no RCS plumes SHALL be rendered

#### Scenario: Background vessel RCS
- **WHEN** rendering an inactive/background vessel
- **THEN** `rcs_nozzle_state` SHALL be `None` (no plumes)

### Requirement: Pod RCS plume rendering

When a pod has built-in RCS (`def.rcs.is_some()` AND `def.category == Pods`), plumes SHALL be rendered from bilateral nozzle positions near the top of the pod (80% up, at the trapezoid edges). The `lateral` flag fires the left nozzle (exhaust left) and `lateral_mirrored` fires the right nozzle (exhaust right). Pod RCS plumes use `generate_pod_rcs_plume_vertices()` instead of the standalone RCS plume function.

## Orbit Lines

### Requirement: Ship orbit line displayed in map view

The ship's orbit line SHALL only be visible when `ship_pixels < 5.0` (map view). It supports multiple patched conic segments.

#### Scenario: Orbit line width and color
- Line width = `0.002 / camera.zoom`
- Outer edge = segment color, inner edge = `[color[0]*0.5, color[1]*0.5, color[2]*0.5, color[3]*0.7]`

#### Scenario: Segment color
- Base color = `[ship_color[0]*0.6, ship_color[1]*0.6, ship_color[2]*0.6, alpha]`
- First segment alpha = 0.7, subsequent = 0.4

#### Scenario: Elliptical orbit rendering
- Parametrize using eccentric anomaly, segment count = `max(16, floor((|angle_span| / TAU) * 512))`

#### Scenario: Hyperbolic trajectory rendering
- 1024 sample points, skip points within `HYPERBOLIC_SKIP_MARGIN = 0.005` rad of asymptote

### Requirement: Predicted trajectory rendering (post-maneuver)

Predicted trajectories from maneuver nodes SHALL be drawn as green lines `[0.0, 1.0, 0.0, alpha]` where alpha = 0.9 (first segment), 0.6 (subsequent). Line width = `0.0015 / camera.zoom`.

## Apoapsis and Periapsis Markers

### Requirement: Ap/Pe markers on orbit lines

Filled circle markers SHALL be drawn at apoapsis and periapsis points.

#### Scenario: Main orbit markers
- Radius = `0.008 / camera.zoom`, 16 segments
- Periapsis: `[0.3, 0.8, 1.0, alpha]` (cyan), Apoapsis: `[1.0, 0.6, 0.2, alpha]` (orange)
- First segment alpha = 1.0, subsequent = 0.6

#### Scenario: Predicted trajectory markers
- Radius = `0.006 / camera.zoom`, 12 segments
- Periapsis: `[0.2, 0.7, 0.9, alpha]`, Apoapsis: `[0.9, 0.5, 0.1, alpha]`
- First segment alpha = 0.7, subsequent = 0.5

#### Scenario: Partial orbit marker visibility
- Only show periapsis if true anomaly 0 is within the traversed arc
- Only show apoapsis if true anomaly PI is within the traversed arc
- Full orbits always show both markers

### Requirement: Ap/Pe label overlays

egui text labels "Ap" and "Pe" SHALL be rendered at each marker's screen position.

#### Scenario: Label formatting
- "Ap" at `(x, y - 12)`, font size 11, color `rgb(255, 153, 51)` (orange)
- "Pe" at `(x, y - 12)`, font size 11, color `rgb(77, 204, 255)` (cyan)

#### Scenario: Altitude on hover
- Within 20 pixels of marker: show altitude at `(x, y + 14)`, font size 10
- Format: `>= 1e9` -> "X.XX Gm", `>= 1e6` -> "X.X Mm", `>= 1e3` -> "X.X km", else "X m"

## Prograde Direction Arrow

### Requirement: Prograde arrow in ship view

When zoomed into the ship (`!needs_indicator`, i.e. ship_pixels >= 5.0) and velocity > 0.1 m/s, a green chevron arrow SHALL be drawn pointing in the prograde (velocity) direction.

#### Scenario: Arrow geometry
- Color: `[0.3, 1.0, 0.3, 0.7]` (green, semi-transparent)
- Position: offset from ship center by `size * 1.5` in the prograde direction
- Size: `size * 0.4` arm length, `size * 0.04` arm thickness
- Two quad arms forming a chevron (">") pointing in velocity direction
- Arms spread at ~0.5 radians (~29 degrees) from the prograde axis

#### Scenario: Arrow direction
- Direction is based on `velocity_direction` (normalized velocity unit vector), independent of ship rotation
- Arrow is drawn in world space and scales naturally with zoom

#### Scenario: Arrow not shown
- **WHEN** `needs_indicator` is true (map view) **THEN** no prograde arrow is drawn
- **WHEN** velocity < 0.1 m/s **THEN** `velocity_direction` is `[0, 0]` and no arrow is drawn

## Flight Part Selection

### Requirement: Flight part click detection

Clicking on a part in ship view SHALL select it by converting click position to vessel-local coordinates (un-scale, un-rotate) and testing against part hitboxes.
