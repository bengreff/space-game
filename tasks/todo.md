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

---

# Colony Foundation Data Model (Layer 0)

## Implementation
- [x] Step 1: `src/colony/mod.rs` + `src/colony/resources.rs` — ResourceType (26 variants), ResourceInventory, Company
- [x] Step 2: `src/colony/buildings.rs` — BuildingType (22 variants), FactoryRecipe (15 variants), BuildingInstance, Colony, ColonyManager
- [x] Step 3: `src/colony/tech.rs` — TechNodeData, TechLineData, TechTree, DiscoveryTracker, ScienceState
- [x] Step 4: `data/tech/tree.ron` — 39 tech nodes, 11 efficiency lines, default unlocked parts
- [x] Step 5: Extend CelestialBody — mineable_resources, atmospheric_resources, habitability_score for all 21 bodies
- [x] Step 6: Wire into Game struct — colony_manager, company, science, tech_tree fields
- [x] Step 7: Extend save system — new fields with #[serde(default)], version check `>` instead of `!=`
- [x] Step 8: Build verification + spec

## Files Changed
- `src/colony/mod.rs` — NEW: re-exports
- `src/colony/resources.rs` — NEW: ResourceType, ResourceInventory, Company
- `src/colony/buildings.rs` — NEW: BuildingType, FactoryRecipe, BuildingInstance, Colony, ColonyManager
- `src/colony/tech.rs` — NEW: TechNodeData, TechLineData, TechTree, DiscoveryTracker, ScienceState
- `data/tech/tree.ron` — NEW: tech tree data file
- `src/bodies.rs` — 3 new fields on CelestialBody, populated for all 21 bodies
- `src/game.rs` — 4 new fields, import colony module, init in new()/reset_for_new_game()
- `src/save.rs` — 5 new SaveGame fields with serde(default), version check relaxed
- `src/lib.rs` — `pub mod colony;`
- `openspec/specs/game/colony/foundation/spec.md` — NEW: foundation spec

## Verification
- [x] `cargo build` — compiles clean, no warnings
- [ ] Visual test: existing saves load cleanly (colony fields default)
- [ ] Visual test: new game starts with empty colony state

---

# Colony Core Loop (Layer 1)

## Implementation
- [x] Task 1: Cargo containers & `PartCategory::Cargo` — `data/parts/cargo.ron`, `CargoData`
- [x] Task 2: Gas Giant Flag — `is_gas_giant` on CelestialBody
- [x] Task 3: Notification Infrastructure — NotificationKind, Notification, toast rendering
- [x] Task 4: Colony Establishment via Cargo — establish_colony() extracts buildings/resources from cargo containers
- [x] Task 5: Colony Simulation Core — simulate_colony_tick() with 8 subsystems, batch ticking
- [x] Task 6: Construction Queue — queue_building(), can_queue_building(), robot throughput
- [x] Task 7: Maintenance System — degradation, repair, robot capacity
- [x] Task 8: Science Lab Simulation — lab_elapsed_years, exact extraction formula
- [x] Task 9: Colony Management UI — render_colony_panel() free function, colony selector/overview/buildings/construction/resources
- [x] Task 10: Main Loop Integration — update_colonies in all 3 frame renderers, notification/toast processing
- [x] Task 11: Save System & Spec Updates — notifications in SaveGame, lab_elapsed_years field, spec
- [x] Task 12: Camera Tracking Fix — skip panning when tracking body/vessel in tracking station
- [x] Task 13: Cargo Manifest on Blueprint/PlacedPart — cargo_resources, cargo_buildings fields
- [x] Task 14: FlightPart Cargo & from_blueprint() — cargo_buildings field, mass calculations include building mass
- [x] Task 15: Editor Cargo Configuration UI — capacity bar, resource/building lists, add/remove, DragValue amounts
- [x] Task 16: Cargo Transfer to Colony — "Transfer Cargo" button, extract_all_cargo() to colony

## Files Changed
- `src/colony/simulation.rs` — simulate_colony_tick, process_maintenance, process_construction, process_science_labs, habitability_multiplier
- `src/colony/notification.rs` — NotificationKind, Notification
- `src/colony/buildings.rs` — queue_building, can_queue_building, food_days_remaining, lab_elapsed_years, storage_capacity, all(), from_display_name()
- `src/colony/mod.rs` — re-exports for simulation, notification
- `src/render/colony_ui.rs` — NEW: render_colony_panel(), ColonyPanelActions
- `src/render/flight.rs` — toast rendering, Colony button, Establish Colony button, Transfer Cargo button, colony panel call
- `src/render/menus.rs` — Colony button in tracking station, colony panel call
- `src/render/state.rs` — colony UI state fields, active_toasts, transfer_cargo_request, vessel_has_cargo, landed_body_has_colony
- `src/game.rs` — notifications field, establish_colony() (cargo-based), update_colonies(), reset
- `src/parts/definition.rs` — CargoData struct, PartCategory::Cargo
- `src/parts/blueprint.rs` — cargo_resources, cargo_buildings on BlueprintPart and PlacedPart
- `src/parts/vessel.rs` — cargo_buildings on FlightPart, has_colony_buildings(), extract_all_cargo(), has_cargo(), cargo_building_mass_tonnes()
- `src/editor/ui.rs` — cargo configuration section in part info panel
- `src/main.rs` — colony integration in all 3 frame renderers, request handling, camera tracking fix, cargo transfer handling
- `src/save.rs` — notifications field with serde(default)
- `src/bodies.rs` — is_gas_giant, landing_science_value()
- `data/parts/cargo.ron` — cargo container definitions (4 sizes)
- `openspec/specs/game/colony/core_loop/spec.md` — Layer 1 spec

## Verification
- [x] `cargo build` — compiles clean, no warnings
- [ ] Visual test: Cargo containers appear in editor under Cargo category
- [ ] Visual test: Cargo container info panel shows capacity bar, add resource/building dropdowns
- [ ] Visual test: Load Habitat + SmallSolarFarm in Medium cargo → shows 24,000/50,000 kg
- [ ] Visual test: Launch with cargo → cargo persists into flight
- [ ] Visual test: Land with colony buildings in cargo → "Establish Colony" button visible
- [ ] Visual test: Establish colony → cargo emptied, colony created with those buildings
- [ ] Visual test: Land at existing colony with cargo → "Transfer Cargo" → resources appear in colony
- [ ] Visual test: Camera tracks bodies in tracking station (no panning interrupts tracking)
- [ ] Visual test: Colony panel shows buildings, resources, construction queue
- [ ] Visual test: Save/load preserves cargo manifest and colony state

---

# Porkchop Plot for Lambert Transfer Planner

## Implementation
- [x] Step 1: Add `PorkchopPoint` and `PorkchopGrid` structs to `src/render/types.rs`
- [x] Step 2: Implement `compute_porkchop_grid()` in `src/ship/transfer.rs` (50x40 grid, log-scale TOF axis)
- [x] Step 3: Replace Lambert sliders with painted 2D porkchop plot in `src/render/state.rs` (HSV color, hover/click interaction)
- [x] Step 4: Wire up grid computation and selected-point evaluation in `src/main.rs`
- [x] Step 5: Build verification — compiles clean
- [x] Step 6: Spec update — `openspec/specs/game/orbits/transfer_planner/spec.md`

## Files Changed
- `src/render/types.rs` — `PorkchopPoint`, `PorkchopGrid` structs
- `src/render/mod.rs` — re-export `PorkchopPoint`, `PorkchopGrid`
- `src/ship/transfer.rs` — `compute_porkchop_grid()` function
- `src/render/state.rs` — porkchop plot UI (replaces sliders), `hsv_to_rgb()` helper, new state fields
- `src/main.rs` — grid computation on target change, selected-point evaluation via `compute_interplanetary()`

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: porkchop plot renders with green-to-red gradient when Lambert target selected
- [ ] Visual test: hover highlights cell and shows transfer info below
- [ ] Visual test: click locks selection
- [ ] Visual test: default selection is lowest-dv point (white circle marker)
- [ ] Visual test: "Create Node" creates correct maneuver node
- [ ] Visual test: changing target recomputes grid
