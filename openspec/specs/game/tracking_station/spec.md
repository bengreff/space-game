# Tracking Station

## Overview
The tracking station provides a solar system observatory view. It renders all celestial bodies and their orbits with time warp controls, and displays all tracked vessels with their orbits and a sidebar for vessel management. There is no active vessel — all vessels are equal.

## Behavior

### Display
- Solar system bodies and orbits render (same wgpu pipeline as flight)
- Time warp panel at top of screen (with date display)
- "Tracking Station" label at bottom of screen (14pt, gray)
- Hovered body labels (white text above body, same as flight)
- Vessels rendered with full parts at each vessel's position (falls back to colored triangle icon if no part data)
- Vessel orbit lines in grey (`[0.6, 0.6, 0.6, 0.4]`). Only elliptical orbits (e < 1) are rendered using 256-segment parametric ellipse line approximation.

### Vessels Sidebar
- Left side panel titled "Vessels" (180px wide, non-resizable)
- Panel only shown when at least one vessel exists
- Scrollable list of all vessels
- Each entry shows:
  - Color indicator circle (8px, vessel color)
  - Vessel name (white)
  - SOI body name in gray below the name
  - "Fly" button on the right side
  - "X" delete button (red) to remove the vessel
- Clicking a vessel name focuses the camera on that vessel and continuously tracks it as it moves (stops body tracking). Panning the camera breaks vessel tracking.
- Clicking "Fly" activates that vessel (pulls from `inactive_vessels` via `activate_vessel()`) and enters flight mode, resetting time warp to 1x
- Clicking "X" deletes the vessel permanently. If the camera was tracking the deleted vessel, it refocuses on Earth.

### Camera Controls
- Left-click drag: pan camera (clears body and vessel tracking)
- Scroll wheel: zoom in/out
- Double-click on body: focus camera on that body (tracked via `render_state.focus_on_body()`)

### Initial Camera
- On entry from main menu, camera focuses on Earth (LAUNCHPAD_BODY_INDEX) and zooms so Earth fills ~half the screen

### State
- `GameMode::TrackingStation` mode
- Solar system simulation runs with time warp
- All vessels are in `inactive_vessels` (no active vessel) and propagate on rails; deleted if orbit periapsis below surface AND vessel is in atmosphere/below landing altitude
- Camera tracking follows focused body (via `render_state.update_tracking()`) or focused vessel (via `render_state.tracked_vessel`)

### Navigation
- Escape key pauses (shows pause overlay with "Main Menu" button)
- Pause overlay "Main Menu" returns to `GameMode::MainMenu`

## Implementation
- `GameMode::TrackingStation` variant in `src/game.rs`
- `render_tracking_station()` method on `RenderState` in `src/render/state.rs`
  - Accepts `vessels: &[TrackingVesselData]` and `active_vessel_id: u64`
  - Returns `(usize, PauseAction, TrackingStationAction)`
- `render_tracking_station_frame()` orchestrator in `src/main.rs`
  - Builds `Vec<TrackingVesselData>` via `build_tracking_vessel_data()`
  - Handles `TrackingStationAction::FlyVessel(id)`, `FocusVessel(id)`, and `DeleteVessel(id)`
- Input handlers: `handle_tracking_station_mouse_input()`, `handle_tracking_station_cursor_moved()`
- `TrackingVesselData` and `TrackingStationAction` types in `src/render/types.rs`
