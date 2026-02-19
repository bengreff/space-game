# Flight Camera

Camera position, zoom, body tracking, and coordinate conversions for flight mode.

## Camera State

### Requirement: Camera position uses f64 precision

The camera position is stored as `[f64; 2]` to prevent precision loss at large distances (billions of meters from origin). The GPU uniform receives a truncated `[f32; 2]` copy, but all camera math (panning, tracking, coordinate conversion) operates in f64.

### Requirement: Camera initial state

A new Camera SHALL initialize with: position `[0.0, 0.0]`, zoom `1.0`, `is_dragging = false`, `last_mouse_pos = [0.0, 0.0]`.

### Requirement: Camera zoom clamping

Zoom SHALL be clamped to `[0.00001, 1e10]` on `zoom_by()` and `[0.001, 1e10]` on `zoom_at()`.

### Requirement: Zoom at world position preserves cursor position

When zooming at a world position, the camera SHALL reposition so that the world coordinate under the cursor remains stationary on screen: `position[i] = world_pos[i] - (world_pos[i] - old_pos[i]) * (old_zoom / new_zoom)`.

### Requirement: Camera panning

The `pan(dx, dy)` method SHALL adjust position by `(-dx / zoom, +dy / zoom)`, accounting for the screen-to-world direction flip on the Y axis.

### Requirement: Camera aspect ratio updates on resize

When the window is resized, `camera.aspect_ratio = new_width / new_height` SHALL be recalculated.

## Coordinate Conversion

### Requirement: Screen to world coordinate conversion

`screen_to_world` SHALL convert pixel coordinates to world coordinates:
1. NDC: `ndc_x = (screen_x / width) * 2 - 1`, `ndc_y = 1 - (screen_y / height) * 2`
2. World: `world_x = ndc_x * aspect_ratio / zoom + position[0]`, `world_y = ndc_y / zoom + position[1]`

### Requirement: World to screen coordinate conversion

`world_to_screen` SHALL convert world coordinates to pixel coordinates:
1. View: `view_x = (world_x - position[0]) * zoom`, `view_y = (world_y - position[1]) * zoom`
2. NDC: `ndc_x = view_x / aspect_ratio`, `ndc_y = view_y`
3. Screen: `screen_x = (ndc_x + 1) * 0.5 * width`, `screen_y = (1 - ndc_y) * 0.5 * height`

## Body Tracking

### Requirement: Camera body tracking

The camera can track a celestial body by index. When tracking, the camera position SHALL be set to the tracked body's world position each frame via `update_tracking()`.

### Requirement: Focus on body

Double-clicking a body calls `focus_on_body(index)`, which centers the camera on the body's world position and sets `tracked_body = Some(index)`.

## View Modes

### Requirement: Ship view vs map view threshold

The ship is in "map view" (indicator visible) when its screen size is less than 5 pixels. It is in "ship view" when its screen size is 5 pixels or larger. Ship pixel size is calculated as `ship_data.size * pixels_per_world_unit * 2.0`, where `pixels_per_world_unit = camera.zoom * screen_height / 2.0`.

#### Scenario: Map view
- **WHEN** `ship_size * pixels_per_world_unit * 2.0 < 5.0`
- **THEN** the ship triangle indicator is drawn, orbit lines are visible

#### Scenario: Ship view
- **WHEN** `ship_size * pixels_per_world_unit * 2.0 >= 5.0`
- **THEN** ship parts are rendered, trees/launchpad are visible, orbit lines are hidden
