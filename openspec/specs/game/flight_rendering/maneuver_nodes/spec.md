# Maneuver Nodes

Maneuver node creation, editing, dragging, burning, and predicted trajectory display.

## Creation

### Requirement: Maneuver node creation via orbit click

Clicking near the ship's orbit line in map view SHALL detect the closest orbit point and show a "Create Maneuver Node" popup button. Only the first segment (index 0) supports node placement.

#### Scenario: Orbit click detection
- Sample 256 points along the first trajectory segment
- If any point is within 15 pixels of click position, store as `pending_orbit_click`

#### Scenario: Node creation details
- ManeuverNode created with: unique auto-incrementing id, orbit parameters copied from segment
- `delta_v` and `remaining_delta_v` = ManeuverDeltaV::default() (prograde: 0.0, radial_out: 0.0)
- Automatically selected (`selected_maneuver_node = Some(id)`)

## Rendering

### Requirement: Maneuver node rendered as diamond marker

Each maneuver node SHALL be drawn as a filled diamond on the egui foreground layer.

#### Scenario: Diamond geometry
- 4 vertices: `(x, y-size)`, `(x+size, y)`, `(x, y+size)`, `(x-size, y)`
- Selected: size = 10.0, fill = `rgb(255, 200, 100)` (bright gold)
- Unselected: size = 8.0, fill = `rgb(200, 150, 50)` (dimmer gold)
- Stroke = 1.5px white

#### Scenario: Delta-v label on hover or select
- When selected or mouse within 15 pixels: show "{remaining_dv:.0} m/s" at `(x, y-18)`, font size 10, white

### Requirement: Maneuver node click detection

Clicking within 20 pixels of a maneuver node marker SHALL select it.

## Editing Panel

### Requirement: Maneuver node editing panel

When a maneuver node is selected, an egui right panel named "maneuver_panel" (width 200) SHALL show editing controls.

#### Scenario: Panel contents
- "Maneuver Node" heading, "Remaining Dv: {total:.1} m/s"
- Prograde/Retrograde slider: range -100 to +100, snap-back to 0
- Radial Out/In slider: range -100 to +100, snap-back to 0
- Delete and Close buttons

#### Scenario: Non-linear slider scaling
- normalized = `delta / 100.0`
- curved = `sign(normalized) * |normalized|^2.0` (quadratic)
- change per frame = `curved * 16.67` m/s
- Full deflection ~1000 m/s/s, small deflection ~1 m/s/s

### Requirement: Maneuver node deletion

Clicking "Delete" SHALL remove the node and set `selected_maneuver_node = None`.

### Requirement: Closing maneuver panel

Clicking "Close" SHALL set `selected_maneuver_node = None` without deleting.

## Dragging

### Requirement: Maneuver node dragging along orbit

Nodes can be dragged along their stored orbit to change position. Sample 360 points along the orbit and set `true_anomaly` to the closest point's value.

## Position Calculation

### Requirement: Maneuver node world position

Node world position SHALL be computed from stored orbital parameters and current parent body position.

#### Scenario: Elliptical position
- `p = a * (1 - e^2)`, `r = p / (1 + e * cos(ta))`
- `world_x = parent_x + r * cos(ta + arg_peri)`

#### Scenario: Hyperbolic position
- `p = |a| * (e^2 - 1)`, `denom = 1 + e * cos(ta)`
- If `denom <= 0.001`, fallback `r = |a|`

## Delta-V Calculation

### Requirement: Maneuver node total delta-v

Total delta-v magnitude SHALL be `sqrt(prograde^2 + radial_out^2)`, computed separately for `delta_v` (trajectory prediction) and `remaining_delta_v` (burn display).

### Requirement: Prograde and radial unit vectors at node

Velocity at node position SHALL be computed via vis-viva equation. Prograde unit = normalized velocity vector. Radial out unit = `[prograde_y, -prograde_x]` (perpendicular, pointing away from parent).

## Burn Application

### Requirement: Maneuver burn application

During a burn with autopilot in ManeuverNode mode, `apply_burn_to_maneuver` SHALL project the ship's thrust onto the maneuver's coordinate system and reduce `remaining_delta_v`.

#### Scenario: Burn projection
- prograde_contribution = `dot(burn_dir, prograde_unit) * delta_v_magnitude`
- radial_contribution = `dot(burn_dir, radial_unit) * delta_v_magnitude`

#### Scenario: Remaining delta-v reduction
- Remaining prograde > 0 and contribution > 0: reduce toward 0
- Remaining prograde < 0 and contribution < 0: increase toward 0
- Burn direction not matching sign: component not affected
- Same rules for radial_out independently
