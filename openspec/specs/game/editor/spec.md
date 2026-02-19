# Editor

Requirements that apply across all editor capabilities. The editor allows building vessels on a grid with visual feedback, camera controls, and launch capability.

## Grid

### Requirement: Grid line rendering

The editor SHALL render a background grid with minor lines every 0.5m and major lines every 2.5m (every 5 minor lines). Grid lines SHALL be drawn as thin quads with thickness `0.005 / sqrt(zoom)` world units.

### Requirement: Grid line colors

Minor grid lines SHALL use color `[0.2, 0.2, 0.3, 0.5]`. Major grid lines and origin axes SHALL use color `[0.3, 0.3, 0.4, 0.7]`.

### Requirement: Grid snapping rules

Parts with odd hitbox width (in grid squares) SHALL snap to grid square centers. Parts with even hitbox width SHALL snap to grid lines. The same rule applies independently to height. Snapping uses hitbox dimensions, not visual dimensions.

## Camera

### Requirement: Camera defaults

The camera SHALL initialize with offset `[0.25, 0.25]` (center of a grid square) and zoom `1.0`.

### Requirement: Scroll wheel zoom

Scroll wheel input SHALL zoom the camera by factor `1.0 + scroll_amount * 0.1`. Zoom SHALL be clamped to the range `[0.1, 16666.0]`.

### Requirement: Middle mouse drag pan

Pressing middle mouse SHALL begin camera dragging. While dragging, mouse movement SHALL pan the camera by pixel delta divided by zoom to convert to world units.

### Requirement: Arrow key pan

Held arrow keys SHALL continuously pan the camera at `1.67 / camera_zoom` world units per second.

### Requirement: Auto-focus on parts

When loading a blueprint, the camera SHALL auto-focus by centering on the bounding box of all parts and setting zoom to `0.6 / max(width, height, 1.0)`.

## UI Layout

### Requirement: Editor UI panels

The editor SHALL render the following egui panels:

| Panel              | Position               | Width  |
|-------------------|------------------------|--------|
| Top Toolbar        | Top bar                | Full   |
| Stats Bar          | Below toolbar          | Full   |
| Parts Palette      | Left side panel        | 200px  |
| Staging Panel      | Right side panel       | 150px  |
| Part Info Panel    | Right (left of staging)| 200px  |
| Instructions Bar   | Bottom bar             | Full   |

### Requirement: Toolbar contents

The toolbar SHALL contain: "Vehicle Editor" heading, New/Save/Load buttons, symmetry mode toggle, Launch button (disabled when `can_launch()` is false), Exit to Flight button, and right-aligned part count.

### Requirement: Instructions bar

The instructions bar SHALL display: "Click part to select, Click build area to place, Right-click to deselect, Scroll to zoom, Drag to pan".

## Input

### Requirement: Keyboard bindings

The editor SHALL handle these keyboard inputs:

| Key              | Action                                  |
|-----------------|-----------------------------------------|
| Arrow keys       | Pan camera (hold for continuous)        |
| Escape           | Deselect palette part, else deselect placed part |
| Delete/Backspace | Delete selected placed part             |
| R                | Cycle symmetry mode                     |

### Requirement: Mouse input

The editor SHALL handle these mouse inputs:

| Input                  | Action                                      |
|-----------------------|--------------------------------------------|
| Left click on part     | Start dragging that part                    |
| Left click on empty    | Place selected palette part (if valid ghost) |
| Left release           | Finish drag (commit or revert)              |
| Right click (dragging) | Cancel drag, revert position                |
| Right click (palette)  | Deselect palette part                       |
| Right click (placed)   | Delete selected placed part                 |
| Middle mouse           | Toggle camera drag mode                     |
| Scroll wheel           | Zoom camera                                 |

### Requirement: egui input gating

All mouse and keyboard input SHALL be ignored when egui reports the pointer is over a UI area.

## Rendering

### Requirement: Camera-relative coordinates

All editor rendering SHALL output vertices in camera-relative coordinates (world position minus camera offset). The shader handles zoom and aspect ratio.

### Requirement: Part colors

Parts SHALL be rendered with the following colors:

| State    | Color                          |
|---------|---------------------------------|
| Normal   | `[0.4, 0.4, 0.45, 1.0]`       |
| Selected | `[0.5, 0.7, 1.0, 1.0]` (blue) |
| Hovered  | `[0.55, 0.55, 0.6, 1.0]`      |

### Requirement: Procedural part rendering

Engines SHALL render with nozzle bell and engine-specific details (chambers, nozzle rings, gimbal actuators, turbopumps). Pods SHALL render with a dark grey body and white circular window. Decouplers SHALL render with a horizontal ring and an adapter trapezoid connecting to the nearest aligned part above.

### Requirement: Render order

Editor rendering SHALL follow this order: grid vertices, placed part vertices (shapes first, then decoupler adapters), ghost preview vertices.

## Launch

### Requirement: Launch prerequisites

The Launch button SHALL be enabled only when `root_part` is set and `parts` is non-empty.

### Requirement: Launch flow

When launching, the system SHALL convert the editor state to a `VesselBlueprint`, create a `FlightVessel` and `Ship` on the launchpad, and switch to `GameMode::Flight`.

## Stats Bar

### Requirement: Stats bar calculations

The stats bar SHALL display:

| Stat      | Calculation                                         |
|----------|----------------------------------------------------|
| Dry mass  | Sum of `def.mass` for all parts (tonnes)            |
| Wet mass  | Dry mass + sum(resource_kg) / 1000                  |
| Thrust    | Sum of engine `thrust_vac` or `thrust_asl` (kN)     |
| TWR       | Thrust / (wet_mass * surface_gravity)               |
| Delta-v   | Total across all stages (Tsiolkovsky)               |
| Resources | Per-resource totals (O2, RP-1, CH4, LH2) from filled tanks |

### Requirement: TWR body selector

TWR SHALL default to Earth at sea level. The body SHALL be selectable via dropdown, with an ASL/Vac toggle button.

### Requirement: Mass display formatting

Mass SHALL display in tonnes if >= 1000 kg, otherwise in kg. Delta-v SHALL display in km/s if >= 1000 m/s, otherwise in m/s.
