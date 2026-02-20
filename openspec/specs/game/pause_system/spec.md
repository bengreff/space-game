# Pause System

## Overview
A universal pause system triggered by the Escape key in any game mode. Pausing freezes simulation and displays a modal overlay on top of the current mode's rendering.

## Behavior

### Pause Toggle
- Escape key toggles `game.paused` in ALL modes (MainMenu, Editor, Flight, TrackingStation)
- **Editor exception**: In Editor mode, if a part is selected (palette or placed), Escape deselects it first. Only when nothing is selected does Escape toggle pause.
- Handled before mode-specific keyboard input (mode handlers don't see Escape)
- When paused, time warp resets to index 0 (1x)
- When paused, mode-specific keyboard input is blocked (only Escape to unpause works)

### Simulation Freeze
- `game.update()` returns immediately when `paused == true`
- Solar system stops advancing (bodies freeze in place)
- Ship physics stop (no integration, no fuel consumption)
- Note: render frames still process (camera remains responsive for egui interaction)

### Pause Overlay
- Semi-transparent dark background (rgba 0,0,0,180) with rounded corners
- "Paused" heading (32pt, white)
- One action button depending on mode:
  - **Main Menu mode**: "Exit Game" button (terminates process)
  - **All other modes**: "Main Menu" button (transitions to `GameMode::MainMenu`)
- Overlay drawn at `egui::Order::Foreground` to appear above all other UI
- Clicking the button triggers the action AND unpauses (via `enter_main_menu()` which sets `paused = false`)

### State
- `paused: bool` field on `Game` struct
- `toggle_pause()` method: flips `paused`, resets `warp_index` to 0 when pausing
- Mode transition methods (`enter_editor()`, `enter_flight()`, `enter_main_menu()`, `enter_tracking_station()`) all set `paused = false`

### Integration with Render Methods
- `render()` (flight): accepts `paused: bool` parameter, draws overlay inside egui closure, returns `PauseAction`
- `render_tracking_station()`: accepts `paused: bool`, draws overlay, returns `PauseAction`
- `render_editor()`: pause overlay drawn in the egui callback from `main.rs`
- `render_main_menu()`: pause overlay drawn in the egui callback from `main.rs`

## Types
- `PauseAction` enum: `None`, `Resume`, `MainMenu` (defined in `src/render/types.rs`)

## Implementation
- `Game::paused` and `Game::toggle_pause()` in `src/game.rs`
- Escape handling in keyboard input dispatch in `src/main.rs`
- Overlay rendering distributed across render methods in `src/render/state.rs` and `src/main.rs`
