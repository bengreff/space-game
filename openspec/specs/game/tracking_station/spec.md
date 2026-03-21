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

### Galaxy View
- When the camera screen span exceeds 0.1 light-years (`GALAXY_VIEW_THRESHOLD_M = 0.1 * 9.461e15` meters), the tracking station enters galaxy view mode.
- In galaxy view, only the galactic center root body (Sagittarius A*) and its direct children (stars, i.e. the Sun) are visible. All planets, moons, and their orbits are hidden (radius set to 0, no indicator rings, not clickable or hoverable).
- If the camera is tracking a planet or moon when entering galaxy view, tracking automatically redirects to the nearest star ancestor (walking up the parent chain until finding a body whose parent is the root).
- Star orbits (e.g., Sun's orbit around Sgr A*) are only shown in galaxy view when the star is the currently tracked body. Panning the camera (which clears tracking) hides the star orbit.
- At normal zoom levels, star orbits around the galactic center are never rendered — only planetary and moon orbits are shown.
- A galaxy background image (NASA/Spitzer Milky Way face-on) is rendered as a single textured quad spanning 100,000 ly centered on Sgr A*. The image is loaded from `data/textures/milky_way.jpg` as a layer in the body texture array. It is rendered after the accretion disc and before orbit lines — behind bodies and orbits. The galaxy image is static and purely visual. Outside galaxy view, it is not rendered.

### Vessels Sidebar
- Left side panel titled "Vessels" (180px wide, non-resizable)
- Panel only shown when at least one vessel exists
- Scrollable list of all vessels
- Each entry shows:
  - Color indicator circle (8px, vessel color)
  - Vessel name (white for controllable, grey `(140, 140, 140)` for debris)
  - SOI body name in gray below the name
  - "Fly" button on the right side (hidden for debris vessels)
  - "X" delete button (red) to remove the vessel
- Clicking a vessel name focuses the camera on that vessel and continuously tracks it as it moves (stops body tracking). Panning the camera breaks vessel tracking.
- Clicking "Fly" activates that vessel (pulls from `inactive_vessels` via `activate_vessel()`) and enters flight mode, resetting time warp to 1x. Debris vessels cannot be flown.
- Clicking "X" deletes the vessel permanently. If the camera was tracking the deleted vessel, it refocuses on Earth.

### Body Info Panel
- Right side panel titled with body name (220px wide, non-resizable)
- Panel only shown when a body is being tracked (`tracked_body.is_some()`)
- Displays:
  - Body name as heading (18pt, white)
  - Description in italic gray (12pt) if non-empty
  - Separator
  - "Physical Properties" subheading: radius (auto-scaled units), surface gravity (m/s^2), mass (scientific notation)
  - Atmosphere section: surface pressure (Pa/kPa/atm) and visible height, or "No atmosphere" in gray
  - "Orbit" section (hidden for root body): semi-major axis, eccentricity, orbital period
  - "Colony Prospects" section (hidden if body has no resources and habitability 0):
    - Habitability score (X/100)
    - "Mineable Resources" list (if any) — resource display names, indented
    - "Atmospheric Resources" list (if any) — resource display names, indented
- Orbital period computed as `T = 2pi * sqrt(a^3 / mu)` where `mu = G * parent_mass`
- `BodyInfoData` struct passed from `main.rs`, built from `SolarSystem.bodies` and `CelestialBody` colony fields

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
  - Accepts `body_info: &[BodyInfoData]` for the body info right panel
  - Returns `(usize, PauseAction, TrackingStationAction)`
- `render_tracking_station_frame()` orchestrator in `src/main.rs`
  - Builds `Vec<TrackingVesselData>` via `build_tracking_vessel_data()`
  - Handles `TrackingStationAction::FlyVessel(id)`, `FocusVessel(id)`, and `DeleteVessel(id)`
- Input handlers: `handle_tracking_station_mouse_input()`, `handle_tracking_station_cursor_moved()`
- `TrackingVesselData`, `TrackingStationAction`, and `BodyInfoData` types in `src/render/types.rs`
