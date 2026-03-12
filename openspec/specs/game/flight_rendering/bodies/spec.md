# Body Rendering

Planet/moon rendering, atmosphere, surface details, orbit lines, and launchpad.

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

### Requirement: Body texture support

Bodies MAY have PNG textures loaded from `data/textures/bodies/<name>.png` (lowercase body name). Textures are loaded at startup, resized to 1024x1024, and stored in a `texture_2d_array` bound at `@group(1)`.

#### Scenario: Textured body rendering
- **WHEN** a body has a loaded texture
- **THEN** vertices use `Vertex::textured()` with UV coordinates mapping the circle to `[0,1]x[0,1]`
- **AND** center vertex UV = `(0.5, 0.5)`, edge vertex at angle θ: UV = `(0.5 + 0.5·cos(θ), 0.5 - 0.5·sin(θ))`
- **AND** the texture array layer index is stored in `color.a`
- **AND** the fragment shader samples the texture when `uv.x + uv.y > 0`

#### Scenario: Disc edge bleeding
- **WHEN** a texture is loaded
- **THEN** edge colors are bled outward before GPU upload: every pixel within 4px of or beyond the disc edge is replaced with the color sampled from 5px inside the disc at the same radial angle
- **AND** this prevents bilinear filtering from blending valid edge colors with the black pixels outside the polar projection disc

#### Scenario: No texture fallback
- **WHEN** no texture file exists for a body
- **THEN** the body renders as a flat-colored circle (unchanged behavior)
- **AND** a 1-layer dummy texture is always created to keep the bind group valid

#### Scenario: Arc mode texturing
- **WHEN** body is in arc rendering mode (zoomed in)
- **THEN** UV coordinates use the same formula based on the body-centric angle

### Requirement: Body minimum visibility threshold

Bodies smaller than 1 pixel on screen SHALL NOT be drawn as filled circles. Bodies are visible when `body_pixel_radius * 2.0 >= 1.0`.

## Body Indicators

### Requirement: Body indicator ring for small bodies

When a body is smaller than 5 pixels on screen (`body_pixels < 5.0`), a ring indicator SHALL be drawn at a fixed screen size of 16 pixels radius.

#### Scenario: Indicator ring rendering
- **GIVEN** a body with `body_pixels < 5.0`
- **THEN** outer radius = `16.0 / pixels_per_world_unit`, inner radius = `outer_radius * 0.7`, 4 segments
- **AND** outer vertex color = body color (or warm amber `[0.7, 0.5, 0.3, 1.0]` if body color RGB sum < 0.1), inner vertex color = dimmed to 30% brightness at 50% alpha

#### Scenario: Hidden bodies skip indicator
- **GIVEN** a body with radius = 0.0 (hidden in galaxy view)
- **THEN** no indicator ring is drawn, and the body is not clickable or hoverable (BodyData has radius=0, indicator_radius=0)

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

### Requirement: SOI-based orbit line filtering

When zoomed into a body (its circle extends beyond the viewport), orbit lines from parent SOIs are hidden to reduce visual clutter. A "view SOI body" is computed as the smallest body that fills the screen and is near the camera.

#### Scenario: View SOI body computation
- **GIVEN** camera zoom and position
- **THEN** iterate all bodies; a body qualifies if:
  - its on-screen diameter > 50% of screen height (body circle is prominent on screen)
  - camera is within the body's SOI radius (camera is in the body's gravitational domain)
- **AND** among qualifying bodies, pick the one with smallest physical radius (innermost/deepest SOI)
- **AND** in galaxy view, no view SOI body is computed (returns None)

#### Scenario: Body orbit filtering by view SOI
- **WHEN** a view SOI body is active
- **THEN** only show orbit lines for bodies whose orbit parent is the view SOI body or a descendant of it
- **AND** this filter is applied before the pixel-threshold filter

#### Scenario: Ship trajectory segment filtering by view SOI
- **WHEN** a view SOI body is active
- **THEN** patched conic trajectory segments whose parent body is not within the view SOI are hidden
- **AND** the first visible segment is treated as `is_first_segment` (with live true anomaly trimming)
- **AND** maneuver node prediction segments are filtered the same way

#### Scenario: Ship trajectory segment filtering by pixel threshold
- **WHEN** any child body of a trajectory segment's parent body is large enough on screen that its orbit line is hidden by the pixel-threshold rule (>= 5px for planets, >= 100px for moons)
- **AND** the segment's parent body is NOT the ship's current SOI body
- **THEN** that trajectory segment SHALL be hidden
- **AND** this applies to patched conic segments, maneuver prediction segments, and background vessel orbits
- **AND** this filter operates independently of the view SOI filter (both must pass)

Segments at the ship's own SOI level are exempt — those hide via the ship_pixels < 5.0 check (ship view vs map view threshold) instead. This ensures the ship's current orbit disappears when the ship triangle disappears, while higher-level orbits disappear when the intervening body's orbit line disappears.

#### Scenario: Background vessel orbit filtering by view SOI
- **WHEN** a view SOI body is active
- **THEN** inactive vessel orbit lines whose parent body is not within the view SOI are hidden
- **AND** vessel orbits are also hidden when the parent body has a zoomed-in child (pixel threshold filter)

#### Scenario: No filtering outside flight mode
- **WHEN** in tracking station, title screen, or main menu
- **THEN** no SOI-based filtering is applied (all orbits shown per existing rules)

## Launchpad

### Requirement: Launchpad rendered on Earth in ship view

A trapezoidal launchpad SHALL be rendered on Earth (body index 3) at surface angle PI/2. Height = 10.0m, top width = 100.0m, bottom width = 120.0m, color = `[0.5, 0.5, 0.5, 1.0]`.
