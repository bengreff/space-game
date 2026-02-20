# Main Menu

## Overview
The main menu is the game's entry point. It displays an animated solar system background with a centered menu overlay offering navigation to the Editor and Tracking Station.

## Behavior

### Display
- Solar system bodies and orbits render as the background (same wgpu pipeline as flight)
- Camera always focuses on the Sun at a fixed zoom (0.002) to show the full solar system
- Camera zoom and position are locked (scroll zoom has no effect)
- Time warp panel at top of screen (functional — solar system advances in real-time, with date display)
- Centered overlay with semi-transparent dark background:
  - "Sunscatter" title heading (36pt, white)
  - "Editor" button (20pt) — opens the ship editor
  - "Tracking Station" button (20pt) — opens the tracking station

### State
- Game starts in `GameMode::MainMenu`
- Solar system simulation runs (bodies orbit, time advances)
- Time warp controls are active (can speed up background animation)
- Active vessel propagates on rails (if on_rails)
- Inactive vessels propagate on rails; deleted if entering atmosphere or below landing altitude

### Navigation
- Clicking "Editor" transitions to `GameMode::Editor` via `game.enter_editor()`
- Clicking "Tracking Station" transitions to `GameMode::TrackingStation` via `game.enter_tracking_station()`, focuses camera on Earth at half-screen zoom
- Escape key pauses (shows pause overlay with "Exit Game" button)

### Pause Overlay (Main Menu)
- When paused from the main menu, the overlay shows "Paused" heading and "Exit Game" button
- "Exit Game" terminates the process

## Implementation
- `GameMode::MainMenu` variant in `src/game.rs`
- `render_main_menu()` method on `RenderState` in `src/render/state.rs`
- `render_main_menu_frame()` orchestrator in `src/main.rs`
- Returns `MainMenuAction` enum (`None`, `Editor`, `TrackingStation`)
