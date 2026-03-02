# Save Game System

## Implementation
- [x] Phase 1: Add Serialize/Deserialize to core types (Ship, ShipState, ShipOrbit, AutopilotTarget, Orbit, FlightVessel, FlightPart, ManeuverNode, ManeuverDeltaV)
- [x] Phase 2: Create `src/save.rs` with SaveGame/SavedVessel structs and save/load/list/restore functions
- [x] Phase 3: Add TitleScreen game mode, TitleScreenUiState, save_name field, TitleScreenAction enum
- [x] Phase 4: Title screen rendering and UI (render_title_screen_frame, New Game/Load Game dialogs)
- [x] Phase 5: Update escape/quit behavior — all modes quit to title screen with save
- [x] Phase 6: Auto-save every 5 minutes when game is loaded
- [x] Phase 7: Full build verification and spec updates

## Files Changed
- `src/ship/mod.rs` — Serialize/Deserialize derives on Ship, ShipState, ShipOrbit, AutopilotTarget; #[serde(skip)] on cache fields
- `src/bodies.rs` — Serialize/Deserialize on Orbit
- `src/parts/vessel.rs` — Serialize/Deserialize on FlightVessel, FlightPart
- `src/parts/registry.rs` — `all_blueprints()` and `merge_blueprints()` methods
- `src/render/types.rs` — Serialize/Deserialize on ManeuverNode, ManeuverDeltaV; TitleScreenAction enum; Quit variant on MainMenuAction
- `src/render/mod.rs` — re-export TitleScreenAction
- `src/render/state.rs` — render_title_screen() method; "Quit" button text on all pause overlays
- `src/save.rs` — NEW: SaveGame, SavedVessel, SaveFileInfo; save/load/list/restore functions
- `src/lib.rs` — `pub mod save;`
- `src/game.rs` — TitleScreen variant, TitleScreenUiState, save_name field, enter_title_screen(), reset_for_new_game()
- `src/main.rs` — render_title_screen_frame(), save_and_quit_to_title(), auto-save timer, TitleScreen input handling

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: title screen appears on launch
- [ ] Visual test: New Game → name → Start → game menu
- [ ] Visual test: build/launch/fly → Escape → Quit → title screen
- [ ] Visual test: Load Game → save file → game menu with correct state
- [ ] Visual test: auto-save creates file after 5 minutes

---

# Sprite-Based Part Rendering

## Implementation
- [x] Step 1: Vertex + Shader Foundation — `Vertex::sprite()` constructor, shader group 2 bindings, sprite sampling path
- [x] Step 2: Sprite Atlas Loading — `src/render/sprites.rs` with shelf packer, PNG loading, GPU texture creation
- [x] Step 3: Pipeline Integration — sprite bind group layout in pipeline, set_bind_group(2) at all render passes
- [x] Step 4: Replace Part Rendering — sprite quads in `generate_part_vertices`, `generate_part_shape_vertices`, ghost rendering
- [x] Step 5: Sprite-Based Engine Plumes — animated plume sprites with 4-frame cycling at 10fps
- [x] Step 6: Wire up call sites — main.rs and state.rs pass sprite atlas to rendering functions
- [x] Fix: Shelf packer boundary check includes padding
- [x] Fix: Preserve sprite UVs through flight vertex transformation (both active vessel and tracking station)
- [x] Spec update: `openspec/specs/game/editor_rendering/part_rendering/spec.md`

## Files Changed
- `src/render/sprites.rs` — NEW: sprite atlas loader, shelf packer, part ID -> UV mapping
- `src/render/shader.wgsl` — group 2 bindings, sprite sampling path with discard
- `src/render/types.rs` — `Vertex::sprite()` constructor
- `src/render/mod.rs` — `pub mod sprites;`
- `src/render/state.rs` — atlas loading, pipeline layout, bind_group(2), UV-preserving vertex transform
- `src/editor/render.rs` — sprite quad helper, sprite-first rendering in all part generators
- `src/main.rs` — pass sprite atlas to editor render functions

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: editor parts render as sprites
- [ ] Visual test: flight parts render with rotation and heat tinting
- [ ] Visual test: plume sprites animate
- [ ] Visual test: interstellar engines fall back to procedural

---

# Part Rotation in Editor

## Implementation
- [x] Step 1: Add rotated dimension helpers to `PartDefinition` (`definition.rs`)
- [x] Step 2: Add `ghost_rotation` to `EditorState`, update placement/drag/weld logic (`state.rs`)
- [x] Step 3: Change R key from symmetry toggle to rotation handler (`main.rs`)
- [x] Step 4: Apply rotation in editor rendering — `rotate_vertices_around()` helper, vertex rotation in placed parts and ghost (`render.rs`)
- [x] Step 5: Add rotation to flight rendering — `ShipPartRenderData.rotation`, per-part rotation in vertex transforms (`types.rs`, `state.rs`, `main.rs`)
- [x] Step 6: Update spec (`openspec/specs/game/editor/parts/spec.md`)

## Files Changed
- `src/parts/definition.rs` — `rotated_hitbox_grid_width/height()`, `rotated_hitbox_width/height()`, `rotated_weld_hitbox_width/height()`
- `src/editor/state.rs` — `ghost_rotation` field, rotated dimensions in placement/drag/weld overlap
- `src/main.rs` — R key rotation handler, `is_part_rotation_swapped()`, rotation in `ShipPartRenderData`
- `src/editor/render.rs` — `rotate_vertices_around()`, rotation in `generate_part_vertices`/`generate_single_ghost_vertices`/`part_at_screen_pos`
- `src/render/types.rs` — `rotation: f64` on `ShipPartRenderData`
- `src/render/state.rs` — per-part rotation in active and background vessel rendering loops
- `openspec/specs/game/editor/parts/spec.md` — Part Rotation requirements section

## Verification
- [x] `cargo build` — compiles clean, no warnings
- [ ] Visual test: R key rotates ghost part 90° clockwise
- [ ] Visual test: rotated parts place and snap correctly
- [ ] Visual test: placed parts rotate in-place with R key
- [ ] Visual test: mirror placement works with rotated parts
- [ ] Visual test: rotated parts render correctly in flight
- [ ] Visual test: R no longer toggles symmetry mode

---

# Solar Panel Deploy/Retract System

## Implementation
- [x] Step 1: Data model — `deployed` on PlacedPart/BlueprintPart, `deploy_fraction`/`deploy_target`/`mirror_partner` on FlightPart
- [x] Step 2: Deploy animation — `update_solar_deploy()` at 0.5/s speed (2s full deploy)
- [x] Step 3: Power gating — multiply solar output by `deploy_fraction`
- [x] Step 4: Render data — `deploy_fraction`/`is_solar_panel` on ShipPartRenderData, click hitbox adjustment
- [x] Step 5: Sprite rendering — `generate_solar_panel_partial()` helper, integration in `generate_part_shape_vertices` and `generate_part_vertices`
- [x] Step 6: Editor UI — Extend/Retract button in solar panel info section
- [x] Step 7: Flight UI — `solar_deploy_request` field, Extend/Retract button in flight popup
- [x] Step 8: Solar output display — multiply by `deploy_fraction`
- [x] Step 9: Spec update — power spec and editor parts spec

## Files Changed
- `src/parts/blueprint.rs` — `deployed` on PlacedPart and BlueprintPart, serialize/deserialize
- `src/parts/vessel.rs` — `deploy_fraction`/`deploy_target`/`mirror_partner` on FlightPart, `update_solar_deploy()`, power gating
- `src/render/types.rs` — `deploy_fraction`/`is_solar_panel` on ShipPartRenderData
- `src/editor/render.rs` — `generate_solar_panel_partial()`, deploy_fraction param on `generate_part_shape_vertices`
- `src/editor/ui.rs` — Extend/Retract button
- `src/render/state.rs` — `solar_deploy_request` field, Extend/Retract button in flight popup
- `src/main.rs` — process `solar_deploy_request`, call `update_solar_deploy()`, populate render data, click hitbox

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: editor Extend/Retract button toggles panel appearance
- [ ] Visual test: flight panels start retracted, animate on Extend
- [ ] Visual test: power shows 0W when retracted, ramps with deploy
- [ ] Visual test: mirror panels sync deploy state
- [ ] Visual test: click hitbox matches retracted panel size
