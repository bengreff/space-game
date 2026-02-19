# Body Rendering

Planet/moon rendering, atmosphere, surface details, orbit lines, and scenery (trees, launchpad).

## Body Circle Rendering

### Requirement: Body rendering uses triangle fan with adaptive segments

Each celestial body is rendered as a filled circle using a triangle fan. The number of segments scales with on-screen circumference: `raw_segments = circumference_pixels / 3.0`, clamped to `[64, 4096]` and rounded to the nearest even number (bitwise AND with `!1`).

#### Scenario: Full circle rendering
- **WHEN** `raw_segments <= 4096`
- **THEN** render a full triangle fan with center vertex at body position with body color

#### Scenario: Arc rendering for extremely zoomed-in bodies
- **WHEN** `raw_segments > 4096` (extremely zoomed in)
- **THEN** render only the visible arc with 4096 segments
- **AND** calculate visible half-angle as `asin(min(viewport_diagonal / distance_to_body, 1.0))`, minimum `0.005 * TAU`
- **AND** center the arc on the angle from body center to camera
- **AND** use f64 precision for arc edge vertex computation

### Requirement: Body minimum visibility threshold

Bodies smaller than 1 pixel on screen SHALL NOT be drawn as filled circles. Bodies are visible when `body_pixel_radius * 2.0 >= 1.0`.

## Body Indicators

### Requirement: Body indicator ring for small bodies

When a body is smaller than 5 pixels on screen (`body_pixels < 5.0`), a ring indicator SHALL be drawn at a fixed screen size of 16 pixels radius.

#### Scenario: Indicator ring rendering
- **GIVEN** a body with `body_pixels < 5.0`
- **THEN** outer radius = `16.0 / pixels_per_world_unit`, inner radius = `outer_radius * 0.7`, 64 segments
- **AND** outer vertex color = body color, inner vertex color = `[color[0]*0.3, color[1]*0.3, color[2]*0.3, color[3]*0.5]`

### Requirement: Body hover detection and label display

When the mouse cursor is over a body (within its radius or indicator radius), the body's name SHALL be displayed as a white label 20 pixels above the body center, proportional font at size 12, on an egui foreground layer. The closest body to cursor wins when areas overlap.

### Requirement: Body click detection for focus

Clicking on a body (within its radius or indicator radius) SHALL trigger `focus_on_body`, centering the camera and starting body tracking.

## Atmosphere

### Requirement: Atmosphere rendering as a gradient ring

Bodies with atmosphere data SHALL have an atmosphere ring drawn behind all other geometry. The ring spans from `body_radius * scale` (inner) to `(body_radius + atmo_height) * scale` (outer).

#### Scenario: Atmosphere color gradient
- **GIVEN** atmosphere color `[r, g, b]`
- **THEN** inner edge color = `[r, g, b, 1.0]`, outer edge color = `[0.0, 0.0, 0.0, 1.0]`

#### Scenario: Atmosphere segment count
- **WHEN** `raw_segments <= 4096`, render full ring
- **WHEN** `raw_segments > 4096`, render only visible arc with 4096 segments

#### Scenario: Atmosphere skipped when too small
- **WHEN** `outer_pixel_radius < 1.0`
- **THEN** atmosphere is not rendered

## Orbit Lines

### Requirement: Body orbit lines

Orbit lines for celestial bodies SHALL be drawn as thick lines (dual-vertex strips) with `line_width = 0.002 / camera.zoom` and 256 segments.

#### Scenario: Orbit line vertex colors
- **GIVEN** orbit color C
- **THEN** outer edge = C, inner edge = `[C[0]*0.5, C[1]*0.5, C[2]*0.5, C[3]*0.7]`

#### Scenario: Orbit line color from body
- **GIVEN** body color `[r, g, b, a]`
- **THEN** orbit color = `[r*0.4, g*0.4, b*0.4, 0.5]`

#### Scenario: Planet orbit line visibility
- **WHEN** a non-moon body's on-screen diameter >= 5 pixels
- **THEN** its orbit line is hidden

#### Scenario: Moon orbit line visibility
- **WHEN** a moon body's on-screen diameter >= 100 pixels
- **THEN** its orbit line is hidden

## Scenery

### Requirement: Trees rendered on body surface in ship view

Trees SHALL be rendered as procedural scenery on the nearest body's surface when in ship view. Up to 750 trees centered around the camera's angular position.

#### Scenario: Tree spacing
- Trees spaced approximately every 33 meters along surface (angular step = `33.0 / radius` radians)

#### Scenario: Tree position jitter
- Deterministic hash of tree index produces three values (hash1, hash2, hash3) in [0, 1)
- Angular position = `N * angle_step + (hash1 - 0.5) * angle_step * 0.8`

#### Scenario: Tree dimensions
- Base: trunk_width = 1.0m, trunk_height = 7.0m, canopy_radius = 3.0m
- size_factor = `0.5 + hash2` (0.5 to 1.5)
- trunk_width scaled by `0.7 + hash3 * 0.6`
- Canopy is 8-segment filled circle at top of trunk

#### Scenario: Tree colors
- Trunk: `[0.35 + hash2*0.2, 0.20 + hash3*0.15, 0.08, 1.0]`
- Canopy: `[0.10 + hash3*0.1, 0.40 + hash1*0.3, 0.10 + hash2*0.1, 1.0]`

#### Scenario: Launchpad exclusion zone
- On Earth (body index 3), trees SHALL NOT be drawn within angular distance `(LAUNCHPAD_BOTTOM_WIDTH * 0.5 / radius) + angle_step` of `LAUNCHPAD_SURFACE_ANGLE` (PI/2)

### Requirement: Launchpad rendered on Earth in ship view

A trapezoidal launchpad SHALL be rendered on Earth (body index 3) at surface angle PI/2. Height = 10.0m, top width = 100.0m, bottom width = 120.0m, color = `[0.5, 0.5, 0.5, 1.0]`.
