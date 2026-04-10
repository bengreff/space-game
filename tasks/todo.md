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

---

# Economy & Science (Layer 2)

## Implementation
- [x] Task 1: Material Breakdown & Cost Computation — `src/colony/economy.rs` with MaterialBreakdown, MaterialMasses, material_breakdown(), fuel_price_per_kg(), format_money(), science reward functions
- [x] Task 2: Cost Display in Editor — vessel cost in stats bar (green/red), company balance in toolbar
- [x] Task 3: Money Deduction on Launch — cost check in launch_from_editor(), deduction on success, error on insufficient funds
- [x] Task 4: Money Display in Flight HUD — company money + science available in top panel right-aligned
- [x] Task 5: Tech Gating in Editor Parts Palette — filter both Interstellar flat list and size-grouped parts by tech_tree.is_part_available()
- [x] Task 6: Discovery Milestone Detection — check_discovery_milestones() for suborbital/orbit/geostationary + per-body orbit/landing
- [x] Task 7: R&D Science Generation — update_rd_science() with diminishing returns, R&D budget DragValue in toolbar, Research button
- [x] Task 8: Tech Tree UI — `src/render/tech_tree_ui.rs` with Nodes and Efficiency Lines tabs
- [x] Task 9: Contract System — `src/colony/contracts.rs` with 9 contract types, contract board UI, completion detection

## Files Changed
- `src/colony/economy.rs` — NEW: material breakdown, cost computation, format_money, science rewards
- `src/colony/contracts.rs` — NEW: ContractType (9 variants), Contract, ContractManager
- `src/render/tech_tree_ui.rs` — NEW: tech tree browser window with Nodes + Efficiency Lines tabs
- `src/colony/mod.rs` — Added economy + contracts modules
- `src/editor/state.rs` — calculate_vessel_cost() method
- `src/editor/ui.rs` — EditorAction variants (SetRdBudget, OpenTechTree, OpenContracts), cost display, tech gating, R&D widget, Research/Contracts buttons
- `src/game.rs` — Cost check in launch, check_discovery_milestones(), check_contracts(), update_rd_science(), contracts field
- `src/render/state.rs` — company_money, science_available, show_tech_tree, show_contracts fields
- `src/render/flight.rs` — Money/science in flight HUD top panel
- `src/render/mod.rs` — pub mod tech_tree_ui
- `src/main.rs` — Wire everything: pass new params, handle new actions, call milestone/contract/R&D methods, render tech tree + contract board
- `src/save.rs` — contracts field with serde(default)
- `openspec/specs/game/colony/economy/spec.md` — NEW: full economy & science spec
- `openspec/specs/game/colony/foundation/spec.md` — Updated with economy.rs, contracts.rs
- `openspec/specs/game/editor_rendering/toolbar/spec.md` — Research, Contracts, R&D, balance, launch cost
- `openspec/specs/game/editor_rendering/stats_bar/spec.md` — Vessel cost display
- `openspec/specs/game/flight_rendering/hud/spec.md` — Money/science in top panel
- `openspec/specs/game/editor/parts/spec.md` — Tech gating requirement

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: editor shows vessel cost in stats bar, green when affordable
- [ ] Visual test: launch deducts money, blocks when insufficient
- [ ] Visual test: flight HUD shows money + science in top panel
- [ ] Visual test: only 9 default parts visible in new game (tech gating)
- [ ] Visual test: reach 100km altitude → suborbital science awarded
- [ ] Visual test: R&D budget generates science over time, costs money
- [ ] Visual test: tech tree window shows nodes by era, can unlock with science
- [ ] Visual test: contracts can be accepted, completed, and pay out
- [ ] Visual test: save/load preserves money, science, contracts

---

# Colony UI Completion & Full-Screen Tech Tree (Layer 3)

## Implementation
- [x] Task 1: Infrastructure — GameMode variants (ColonyOverview, Management, TechTree), tech_tree_return_mode, navigation methods, action enums, module declarations
- [x] Task 2: Colony Overview Screen — render_colony_overview_ui.rs, colony cards with stats, empty state
- [x] Task 3: Management Screen — render_management_ui.rs, finances, R&D, science, contracts, tech tree link
- [x] Task 4: Full-Screen Tech Tree — rewritten tech_tree_ui.rs with graph visualization, era columns, prerequisite arrows, node clicking, detail side panel, efficiency lines
- [x] Task 5: Building Batch Assignment — batch size selector (10/100/1000/All), u32 count parameter on mine/collector/factory assignment actions, loop-based handling in main.rs
- [x] Task 6: Building Tech Gating — tech_tree.is_building_available() check in ComboBox, "[Locked]" suffix for unavailable buildings
- [x] Task 7: Main Loop Integration & Specs — 3 new frame functions, main menu buttons, input handling, render_colony() tech_tree param, editor tech tree navigation, spec files

## Files Changed
- `src/game.rs` — 3 new GameMode variants, tech_tree_return_mode, enter/leave methods
- `src/render/types.rs` — ColonyOverviewAction, ManagementAction, TechTreeScreenAction enums; Colonies/Management on MainMenuAction
- `src/render/colony_overview_ui.rs` — NEW: colony overview screen
- `src/render/management_ui.rs` — NEW: management screen
- `src/render/tech_tree_ui.rs` — REWRITTEN: full-screen graph tree
- `src/render/colony_ui.rs` — Batch assignment selector, u32 count on assignment actions, tech_tree param, tech gating in ComboBox
- `src/render/menus.rs` — 3 new render methods (colony_overview, management, tech_tree_screen), tech_tree param on render_colony
- `src/render/mod.rs` — New modules and re-exports
- `src/render/state.rs` — Removed show_tech_tree
- `src/main.rs` — 3 new frame functions, GameMode dispatch, input handling, main menu buttons, batch count handling, editor tech tree navigation
- `openspec/specs/game/colony/overview/spec.md` — NEW
- `openspec/specs/game/colony/management/spec.md` — NEW
- `openspec/specs/game/colony/tech_tree/spec.md` — NEW
- `openspec/specs/game/colony/core_loop/spec.md` — Updated (batch assignment, tech gating, navigation)
- `openspec/specs/game/editor_rendering/toolbar/spec.md` — Updated (Research navigates to full-screen tech tree)

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: main menu shows "Colonies" and "Management" buttons
- [ ] Visual test: colony overview lists colonies with stats, "Open" enters colony
- [ ] Visual test: management shows funds, science, R&D, contracts, "Open Tech Tree"
- [ ] Visual test: tech tree renders as graph with era columns, prerequisite arrows
- [ ] Visual test: node detail panel shows info + unlock button
- [ ] Visual test: building +/− uses batch selector (10/100/1000/All)
- [ ] Visual test: colony build queue filters by tech availability
- [ ] Visual test: editor "Research" enters full-screen tech tree, returns to editor
- [ ] Visual test: time warp + date works on all new screens

---

# Fix R&D Simulation + Redesign Tech Tree Prerequisites & Layout

## Implementation
- [x] Task 1: Fix R&D Science in All Frame Functions — add `check_contracts()` + `update_rd_science()` to tracking station, colony, colony overview, management, and tech tree frame functions
- [x] Task 2: Add col/row fields to TechNodeData — `col: u32` and `row: u32` with `#[serde(default)]`
- [x] Task 3: Rewrite data/tech/tree.ron — 39 nodes with descriptive IDs, explicit col/row layout, technology area row bands, updated efficiency line node references
- [x] Task 4: Update Tech Tree Renderer — replace era-column layout with col/row positioning, dynamic canvas size, remove era column headers, show prereq name in efficiency lines
- [x] Task 5: Save Compatibility — migrate_tech_ids() maps old "N.N" IDs to new descriptive IDs in restore_to_game()
- [x] Task 6: Spec Updates — tech_tree spec (new layout, node data model, save compat), core_loop spec (R&D in all frames)

## Files Changed
- `src/main.rs` — `check_contracts()` + `update_rd_science()` in 5 frame functions
- `src/colony/tech.rs` — `col: u32`, `row: u32` fields on TechNodeData
- `data/tech/tree.ron` — Complete rewrite: new IDs, prereqs, col/row, efficiency line refs
- `src/render/tech_tree_ui.rs` — col/row positioning, dynamic canvas, no era headers
- `src/save.rs` — `migrate_tech_ids()` for old save compatibility
- `openspec/specs/game/colony/tech_tree/spec.md` — Updated
- `openspec/specs/game/colony/core_loop/spec.md` — Updated

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: R&D drains funds and generates science in Management/Colony/TrackingStation screens
- [ ] Visual test: tech tree displays as graph with logical technology areas
- [ ] Visual test: all prerequisite arrows go left-to-right
- [ ] Visual test: clicking nodes shows correct detail panel
- [ ] Visual test: old saves load correctly with migrated tech IDs

---

# Tech Tree Layout Polish + Part Name Fix

## Implementation
- [x] Fix backwards arrow on AM Power — moved to col 12 (prereqs at col 10, 11)
- [x] Compact layout — removed gap rows, late-game techs moved up (rows 0-14 instead of 0-22)
- [x] Deep space hab connected to compact_fission (fission prereq)
- [x] Fixed all part names in tree.ron to match actual RON part file names (76 name mismatches fixed)
- [x] Removed "Colony Module" (nonexistent part) from colony_engineering unlocks
- [x] Selection highlighting — selected node outline blue, connected nodes (prereqs + dependents) outline white
- [x] Updated default_unlocked_parts to use correct names (e.g. "Tiny Fuel Tank 1" not "Tank 1x1")
- [x] Fixed Daedalus engine names ("Daedalus Stage 1/2" not "Daedalus S1/S2")
- [x] Flattened layout to 6 rows (was 14) — nuclear thermal in line with medium hydrolox at row 2
- [x] Nuclear → Fusion → Antimatter runs flat on row 2 as main highway
- [x] Branches (NTR variants, shielding, power) on row 3 below highway
- [x] Crewed/colony chain on row 4, electric propulsion on row 5
- [x] Passive shielding now requires deep_space_hab
- [x] Both Daedalus Stage 1 and Stage 2 in fusion_full
- [x] Renamed tech-application nodes to technology names (Inertial Confinement Fusion, High-Energy Fusion, Magnetoplasmadynamics, Pulsed Nuclear Detonation, Antimatter Annihilation, Directed Gamma Radiation)

## Files Changed
- `data/tech/tree.ron` — Flat 6-row layout, renamed nodes, updated prereqs
- `src/render/tech_tree_ui.rs` — Selection highlight (blue selected, white connected)
- `openspec/specs/game/colony/tech_tree/spec.md` — Updated row bands, stroke docs

## Verification
- [x] `cargo build` — compiles clean
- [x] All 55 prerequisite edges verified left-to-right (col strictly increasing)
- [x] All 39 node positions unique (no collisions)
- [x] All 32 prerequisite IDs reference valid nodes
- [ ] Visual test: flat tree layout with 6 rows, no downward slope
- [ ] Visual test: selected node outlined in blue, connected nodes outlined in white
- [ ] Visual test: all parts appear in editor palette when tech is unlocked

---

# Double Horizontal Spacing + Efficiency Lines as Graph Nodes

## Implementation
- [x] Task 1: Double horizontal spacing — COL_SPACING 190 → 380
- [x] Task 2: Add col/row fields to TechLineData in tech.rs
- [x] Task 3: Add col/row values to all 11 efficiency lines in tree.ron (rows 6-7)
- [x] Task 4: Rewrite renderer — line nodes in graph, arrows, click detection, detail panel, remove old lines section
- [x] Task 5: Update tech_tree spec
- [x] Task 6: Update todo.md

## Files Changed
- `src/render/tech_tree_ui.rs` — COL_SPACING → 380, line nodes in graph, line detail panel, removed render_efficiency_lines()
- `src/colony/tech.rs` — Added col/row to TechLineData
- `data/tech/tree.ron` — Added col/row to all 11 efficiency lines
- `openspec/specs/game/colony/tech_tree/spec.md` — Updated layout, line nodes spec, removed old efficiency lines section

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: graph shows tech nodes AND line nodes with arrows between them
- [ ] Visual test: nodes are twice as far apart horizontally as before
- [ ] Visual test: clicking a line node shows detail panel with recipes, cost, tier info
- [ ] Visual test: upgrade button works from line node detail panel
- [ ] Visual test: line nodes use distinct teal color when in progress

---

# Efficiency Line Restructuring + UI Polish

## Implementation
- [x] COL_SPACING 380 → 285 (25% closer horizontally)
- [x] Sidebar width 280 → 420 (50% wider)
- [x] Added "Affects:" section to line detail panel (line_affects() helper)
- [x] Restructured efficiency line prerequisites and positions:
  - metallurgy: colony_engineering → extended_missions + mining T1 (col 9→6)
  - construction: colony_engineering → deep_space_hab + metallurgy T1 (col 7 unchanged)
  - chemical_processing: colony_engineering → extended_missions + mining T1 (col 8→6)
  - life_support: col 6→5, row 6→7
  - precision_mfg: extended_missions → deep_space_hab (col 9→7)
  - electronics_mfg: row 7→8
  - nuclear_engineering: row 7→8
- [x] Verified: no position collisions, all arrows left-to-right
- [x] Spec and todo updated

## Arrow Gap Summary (before → after)
- metallurgy: 5 col → 1 col (node) + 1 col (line)
- construction: 3 col → 1 col (node) + 1 col (line)
- chemical_processing: 4 col → 1 col (node) + 1 col (line)
- precision_mfg: 4 col → 1 col (node) + 1 col (line)
- nuclear_engineering: 2 col → 2 col (unchanged, only >1 gap remaining)

## Verification
- [x] `cargo build` — compiles clean
- [x] All 51 positions unique (no collisions)
- [x] All arrows strictly left-to-right
- [ ] Visual test: short arrows between efficiency line nodes
- [ ] Visual test: mining → metallurgy → construction chain visible
- [ ] Visual test: "Affects" section shows in line detail panel

---

# Trade Route Mechanics (Layer 4)

## Implementation
- [x] Task 1: Trade Route Data Model — `src/colony/trade.rs` with TradeShipId, TradeRouteId, TradeShipState, RouteLeg, CargoManifest, AutomationMode, TradeRoute, TradeShip, FleetManager; integration into Game, SaveGame
- [x] Task 2: Transfer Delta-V Computation — `src/colony/transfer.rs` with AtmosphereClass, gravity_loss_factor, compute_leg_delta_v, blueprint_total_delta_v, compute_cargo_capacity, next_launch_window
- [x] Task 3: Notification Extensions — 4 new NotificationKind variants (ShipArrived, ShipDeparted, RoutePaused, ShipConstructionComplete); RoutePaused stops warp
- [x] Task 4: Fleet Transit Simulation — update_fleet(), process_arrival(), launch_ship(), build_ship() on FleetManager; called from Game::update_colonies()
- [x] Task 5: Route Automation — check_automation() with WindowBased, FrequencyBased, DvThreshold modes; pre-launch validation, priority sorting
- [x] Task 6: Trade Route UI — render_fleet_overview_panel() in colony overview, render_colony_trade_section() in per-colony screen, route creation wizard, TradeAction enum + handle_trade_action() dispatcher
- [x] Task 7: Specs & Todo Update — trade_routes spec, todo tracking

## Files Changed
- `src/colony/trade.rs` — NEW: FleetManager, TradeShip, TradeRoute, fleet simulation, automation
- `src/colony/transfer.rs` — NEW: transfer delta-v computation, atmosphere classification, cargo capacity
- `src/render/trade_ui.rs` — NEW: fleet overview panel, colony trade section, route creation wizard
- `src/colony/mod.rs` — Added trade + transfer modules
- `src/colony/notification.rs` — 4 new notification variants
- `src/game.rs` — fleet field, update_fleet() + check_automation() calls in update_colonies()
- `src/save.rs` — fleet field with serde(default)
- `src/render/types.rs` — TradeAction enum, Trade variant on ColonyOverviewAction
- `src/render/colony_ui.rs` — Trade(TradeAction) variant on ColonyScreenAction, FleetManager/earth_index params, trade section card
- `src/render/colony_overview_ui.rs` — FleetManager/earth_index params, fleet panel replaces "coming soon" placeholder
- `src/render/menus.rs` — fleet/earth_index params on render_colony_overview() and render_colony()
- `src/render/mod.rs` — pub mod trade_ui, TradeAction re-export
- `src/main.rs` — handle_trade_action(), fleet/earth_index passthrough to render calls, Trade match arms
- `openspec/specs/game/colony/trade_routes/spec.md` — NEW

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: colony overview shows Fleet section with routes and ships
- [ ] Visual test: per-colony screen has Trade section showing local routes/ships
- [ ] Visual test: route creation wizard: select destination, blueprint, slider updates dv/time
- [ ] Visual test: route creation wizard: set cargo, crew, automation, create route
- [ ] Visual test: launch ship on route, advance time, ship arrives and unloads cargo
- [ ] Visual test: automated route launches at window/frequency
- [ ] Visual test: route pauses with notification when fuel/cargo insufficient
- [ ] Visual test: save/load preserves routes, ships, fleet state
- [ ] Visual test: old saves load cleanly (empty fleet default)

---

# Trade Route UI Overhaul

## Implementation
- [x] Task 1: Colony Screen Pause Menu — Add "Colony Overview" button between Tracking Station and Main Menu; GoToColonyOverview action + handler
- [x] Task 2: Make Colony Screen Trade Section Read-Only — Remove route_creation/wizard params, remove New Trade Route button, remove Pause/Resume buttons, add "Manage routes in Colony Overview" text
- [x] Task 3: New Transfer Computation Functions — RouteCategory enum (SameSOI/Interplanetary/Interstellar), classify_route(), compute_synodic_period(), compute_interstellar_transfer() with relativistic math, estimate_flight_time(), public blueprint_dv_with_cargo()
- [x] Task 4: Rewrite RouteCreationState and Creation Panel — Single-panel creation/editing UI replacing old 7-step wizard; cache invalidation via hash key; all sections visible at once
- [x] Task 5: Wire Creation Panel into Colony Overview + Route Editing — New params on menus.rs/main.rs, EditRoute/OpenEditor TradeAction variants, Edit button on route cards, OpenEditor interception, EditRoute handler
- [x] Task 6: Update TradeRoute Struct for New Scheduling — route_category, interval_days, ships_per_window fields with serde defaults; updated check_automation()
- [x] Task 7: Specs & Verification — trade_routes spec updated, todo updated, cargo build clean

## Files Changed
- `src/colony/transfer.rs` — RouteCategory enum, classify_route(), compute_synodic_period(), compute_interstellar_transfer(), estimate_flight_time(), blueprint_dv_with_cargo() (renamed + public)
- `src/colony/trade.rs` — route_category, interval_days, ships_per_window on TradeRoute; updated check_automation()
- `src/render/trade_ui.rs` — Rewritten RouteCreationState + render_route_creation_panel() (single-panel); read-only render_colony_trade_section(); Edit button on fleet overview route cards
- `src/render/colony_overview_ui.rs` — New params (route_creation, blueprints, part_defs, solar_system, sim_time); Create Trade Route button; wizard modal
- `src/render/colony_ui.rs` — GoToColonyOverview action + pause menu button; removed wizard/creation params from render_colony_screen()
- `src/render/types.rs` — TradeAction::EditRoute, TradeAction::OpenEditor variants
- `src/render/menus.rs` — New params on render_colony_overview(); removed extra params from render_colony(); pass route_creation/blueprints/part_defs/solar_system/sim_time
- `src/render/mod.rs` — pub use RouteCreationState
- `src/main.rs` — GoToColonyOverview handler; new params on render_colony_overview call; OpenEditor interception; EditRoute handler in handle_trade_action()
- `openspec/specs/game/colony/trade_routes/spec.md` — Updated: route categories, scheduling modes, single-panel creation, edit capability, read-only colony trade section

## Verification
- [x] `cargo build` — compiles clean, zero warnings
- [ ] Visual test: colony screen pause menu has "Colony Overview" button
- [ ] Visual test: colony screen trade section is read-only with management hint
- [ ] Visual test: colony overview has "Create Trade Route" button
- [ ] Visual test: creation panel shows all sections (name, blueprint, source, dest, transfer analysis, cargo, costs, scheduling)
- [ ] Visual test: source filters to Earth + colonies with Launchpad
- [ ] Visual test: transfer analysis shows min dv, adjustable budget, ship dv, travel time
- [ ] Visual test: interplanetary routes show transfer window frequency + ships-per-window scheduler
- [ ] Visual test: same-SOI routes show interval scheduler
- [ ] Visual test: Edit button on route opens pre-filled creation panel
- [ ] Visual test: old saves load cleanly (new scheduling fields default)

---

# Trade Route Creation Panel Fixes

## Implementation
- [x] Task 1: Fix cargo mass not affecting ship delta-v — added `extra_dry_mass_tonnes` to FlightVessel, included in `calculate_stage_delta_v()` wet_mass, used in `blueprint_dv_with_cargo()`
- [x] Task 2: Fix ParentToChild/ChildToParent transfer delta-v — `compute_hohmann_simple()` returns (dep_dv, arr_dv, transfer_time), ParentToChild uses synthetic 200km parking orbit + full Hohmann
- [x] Task 3: Add `blueprint_cargo_container_capacity()` — sums CargoData.capacity_kg across blueprint parts
- [x] Task 4: Filter cargo manifest + enforce capacity — exclude ship fuels from dropdown, effective capacity = min(dv_capacity, container_capacity), "No cargo containers" warning
- [x] Task 5: Crew limits from blueprint — detect probe core (can_control + crew=0), min crew = 0 with probe / 1 without, max = total pod capacity
- [x] Task 6: Departure Inventory — shows fuel, cargo, food (crew x 0.5 kg/day x flight_days), crew count
- [x] Task 7: Destination Inventory — ship name, remaining fuel estimate, cargo delivered intact, food consumed, crew arriving
- [x] Task 8: Spec + todo update

## Files Changed
- `src/parts/vessel.rs` — `extra_dry_mass_tonnes` field on FlightVessel, added to wet_mass in `calculate_stage_delta_v()`
- `src/colony/transfer.rs` — `compute_hohmann_simple()` returns 3-tuple, ParentToChild uses parking orbit Hohmann, `blueprint_cargo_container_capacity()`, fixed `blueprint_dv_with_cargo()`
- `src/render/trade_ui.rs` — Cargo filtering (ship fuels excluded), container capacity, crew limits, departure/destination inventory sections
- `openspec/specs/game/colony/trade_routes/spec.md` — Updated

## Verification
- [x] `cargo build` — compiles clean, zero warnings
- [ ] Visual test: Earth→Moon transfer_dv ~3,900 m/s (was 509)
- [ ] Visual test: Adding cargo reduces ship delta-v
- [ ] Visual test: Fuel resources excluded from cargo dropdown
- [ ] Visual test: Cargo limited by container capacity
- [ ] Visual test: Crew capped by blueprint pod capacity
- [ ] Visual test: Departure inventory shows fuel + cargo + food + crew
- [ ] Visual test: Destination inventory shows ship + remaining fuel + cargo + crew

---

# Trade Route Resource Flow & UI Fixes

## Implementation
- [x] Task 1: Fix cargo Add button when no containers — require `effective_capacity > 0.0` in `can_add`
- [x] Task 2: Add "Min" button on dv budget — sets `dv_budget = cached_min_dv`
- [x] Task 3: Rework fleet overview ship display — in-transit shows progress (X/Y days), stationed shows "Ready at" / "Arrived at"
- [x] Task 4: Add `alert_reason` to TradeRoute — automation sets alert instead of pausing, clears on success, shown inline in fleet overview
- [x] Task 5: Add food to `launch_ship()` departure costs — crew × 0.5 kg/day × flight_days, bought from Earth or subtracted from colony
- [x] Task 6: Spec + todo update

## Files Changed
- `src/render/trade_ui.rs` — Fixed cargo Add button, added Min dv button, reworked fleet ship display with progress and alert
- `src/colony/trade.rs` — Added `alert_reason` to TradeRoute, changed `check_automation` to alert instead of pause, added food subtraction to `launch_ship()`
- `openspec/specs/game/colony/trade_routes/spec.md` — Updated spec

## Verification
- [x] `cargo build` — compiles clean, zero warnings
- [ ] Visual test: Cargo Add button disabled when no cargo containers on blueprint
- [ ] Visual test: "Min" button next to dv budget sets it to Hohmann minimum
- [ ] Visual test: Fleet overview shows in-transit ships with progress (X / Y days)
- [ ] Visual test: Fleet overview shows "Ready at [Source]" for stationed ships at source
- [ ] Visual test: Fleet overview shows "Arrived at [Dest]" for stationed ships at destination
- [ ] Visual test: Route alert shows inline when resources insufficient (route not paused)
- [ ] Visual test: Alert clears when ship successfully launches
- [ ] Visual test: Food subtracted from colony on launch
- [ ] Visual test: Food bought from Earth on Earth-source launch

---

# Trade Route Resource Flow & UI Fixes (Round 2)

## Implementation
- [x] Fix automation never triggering — `check_automation` filter excluded all new routes (`automation == Manual` was filtered out, but all new routes use Manual + route_category)
- [x] Fix ships stuck at destination — `process_arrival` now always starts return trip (even without return cargo) so ships cycle back to source for next launch
- [x] Add total launch cost summary — Earth-source routes show total cost (fuel + cargo + food) in Departure Inventory
- [x] Fix fleet overview — only show ship status line when in transit; no "Arrived at" / "Ready at" noise
- [x] Show alert_reason on manual launch failure — `handle_trade_action` sets alert_reason on route when manual Launch button fails
- [x] Show return direction — in-transit ships show "Returning to [Source]" vs "In transit to [Dest]"

## Files Changed
- `src/colony/trade.rs` — Fixed `check_automation` filter, fixed `process_arrival` to always return, added food to `launch_ship`
- `src/render/trade_ui.rs` — Fleet overview only shows in-transit ships, total launch cost for Earth source, return direction label
- `src/main.rs` — Manual launch sets/clears alert_reason on route
- `openspec/specs/game/colony/trade_routes/spec.md` — Updated

## Verification
- [x] `cargo build` — compiles clean, zero warnings
- [ ] Visual test: automated launches trigger on schedule (SameSOI interval, interplanetary window)
- [ ] Visual test: ships return to source after delivering cargo (even without return cargo)
- [ ] Visual test: fleet overview only shows ship line when in transit
- [ ] Visual test: in-transit shows "Returning to Earth" on return leg
- [ ] Visual test: Earth-source routes show total launch cost in Departure Inventory
- [ ] Visual test: manual Launch button failure shows alert on route card
- [ ] Visual test: alert clears after successful launch

---

# Trade Route Automation & Cost Fixes (Round 3)

## Implementation
- [x] Add rocket cost to launch — `launch_ship()` charges rocket cost (sum of part costs) for Earth-source launches, not just fuel/cargo/food
- [x] Add rocket cost to UI — Departure Inventory shows itemized cost breakdown (Rocket + Fuel + Cargo + Food) and total
- [x] Fix check_automation scheduling — all new routes (automation=Manual) now use route_category-based scheduling; SameSOI uses interval_days
- [x] Fix process_arrival to always return — ships now return to source even without return cargo, enabling the launch→deliver→return→relaunch cycle
- [x] Fix notification spam — only send RoutePaused notification on FIRST failure, not every tick
- [x] Show alert in colony trade section — alerts now visible in per-colony trade view too
- [x] Fix cargo purchase — Earth-source cargo now checks total cost upfront instead of silently skipping items
- [x] Manual launch failure shows alert — alert_reason set on route for both manual and automated launch failures

## Files Changed
- `src/colony/trade.rs` — Rocket cost in launch_ship, upfront cargo cost check, check_automation scheduling fix, notification throttling, process_arrival always returns
- `src/render/trade_ui.rs` — Itemized launch cost breakdown, alerts in colony trade section, in-transit-only ship display
- `src/main.rs` — Manual launch sets alert_reason
- `openspec/specs/game/colony/trade_routes/spec.md` — Updated

## Verification
- [x] `cargo build` — compiles clean, zero warnings
- [ ] Visual test: Earth route Departure Inventory shows Rocket + Fuel + Cargo + Food cost breakdown
- [ ] Visual test: Total launch cost includes rocket cost
- [ ] Visual test: Automated launches trigger on schedule (interval-based for SameSOI)
- [ ] Visual test: Ships return to source after delivery (even without return cargo)
- [ ] Visual test: Launch failure shows alert on route card + notification toast
- [ ] Visual test: Alert doesn't spam (one notification per failure state change)
- [ ] Visual test: Insufficient funds error shows both need and have amounts

---

# Contract System Revamp

## Implementation
- [x] Phase 1: Rewrite `src/colony/contracts.rs` — Destination, ContractKind, Contract, ContractPayload, GovernmentMilestone, ContractManager with pool-based generation, accept/cancel/complete, payload/tourism checks, milestone checks, size scaling, destination gating, deterministic PRNG
- [x] Phase 1: Add notification variants — ContractCompleted, MilestoneAchieved in notification.rs; crewed discovery fields (first_crewed_orbit/lunar/mars) in DiscoveryTracker
- [x] Phase 2: Add cargo_payloads — Vec<ContractPayload> on BlueprintPart, PlacedPart, FlightPart; wired through parts_to_blueprint/blueprint_to_parts/from_blueprint; all_payloads() on FlightVessel; payload mass in cargo_extra_mass_tonnes()
- [x] Phase 3: Game logic — Rewritten check_contracts() with payload/tourism checks + notifications; new check_government_milestones() with crewed discovery tracking
- [x] Phase 4: UI — ManagementAction::AcceptContract(u64) + CancelContract(u64); rewritten management screen contracts section with pool/active/milestones; payload placement UI in editor cargo section; editor contract board uses shared render_contracts_section()
- [x] Phase 5: Integration — Tourism completion on vessel recovery; pool refill at init/load/completion; check_government_milestones() wired next to check_contracts() in all frame functions; save compatibility via serde defaults

## Files Changed
- `src/colony/contracts.rs` — REWRITTEN: pool-based contract system
- `src/colony/notification.rs` — 2 new variants (ContractCompleted, MilestoneAchieved)
- `src/colony/tech.rs` — 3 new fields on DiscoveryTracker (first_crewed_orbit/lunar/mars)
- `src/parts/blueprint.rs` — cargo_payloads on BlueprintPart and PlacedPart
- `src/parts/vessel.rs` — cargo_payloads on FlightPart, all_payloads(), cargo_extra_mass_tonnes()
- `src/game.rs` — Rewritten check_contracts(), new check_government_milestones()
- `src/render/types.rs` — AcceptContract(u64), CancelContract(u64)
- `src/render/management_ui.rs` — REWRITTEN: pool-based UI with render_contracts_section()
- `src/render/mod.rs` — pub mod management_ui
- `src/editor/ui.rs` — Payload UI in cargo section, contracts param on render_editor_ui()
- `src/main.rs` — Tourism recovery hook, pool refill wiring, milestone checks, updated contract board
- `src/save.rs` — Pool refill on load
- `openspec/specs/game/colony/contracts/spec.md` — NEW
- `openspec/specs/game/colony/economy/spec.md` — Updated contract section

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: contract board shows ~5 available contracts, only suborbital initially
- [ ] Visual test: accept payload contract, payload appears in editor cargo UI
- [ ] Visual test: place payload in cargo container, capacity decreases by payload mass
- [ ] Visual test: launch, reach destination, notification pops, money increases
- [ ] Visual test: new contract appears in pool after completion
- [ ] Visual test: accept tourism contract, fly to destination, recover vessel, payout
- [ ] Visual test: government milestones fire on first achievements
- [ ] Visual test: size scaling: after 5+ completions, larger contracts appear
- [ ] Visual test: old saves load without crash (fresh contract state)

---

# Ship Hangar UI & Wiring

## Implementation
- [x] Add `BuildingType::Hangar` to `BUILDABLE_BUILDINGS` array in colony_ui.rs
- [x] Add `Hangar` to `colony_engineering` tech node in tree.ron
- [x] Add `ScrapShip(usize, StoredShipId)` variant to `ColonyScreenAction`
- [x] Add `render_hangar_card()` — capacity bar, stored ships grid with Scrap buttons
- [x] Wire hangar card into colony screen (card 8, between trade and debug)
- [x] Add `auto_build_ships` field to `RouteCreationState` + checkbox in scheduling section
- [x] Wire `auto_build_ships` value into route creation (was hardcoded false)
- [x] Call `migrate_stationed_ships()` on game load in save.rs
- [x] Handle `ScrapShip` action in main.rs — scrap ship, emit notification
- [x] Show ship names in construction queue via `effective_target()` check

## Files Changed
- `src/render/colony_ui.rs` — Hangar in BUILDABLE_BUILDINGS, ScrapShip action, render_hangar_card(), ship names in construction queue
- `src/render/trade_ui.rs` — auto_build_ships field + checkbox + route wiring
- `src/save.rs` — migrate_stationed_ships() call on load
- `src/main.rs` — ScrapShip action handler with notification
- `data/tech/tree.ron` — Hangar added to colony_engineering unlocks

## Verification
- [x] `cargo build` — compiles clean, zero warnings
- [ ] Visual test: Hangar appears in construction dropdown (tech-gated)
- [ ] Visual test: Hangar card shows capacity bar and stored ships
- [ ] Visual test: Scrap button removes ship and recovers resources
- [ ] Visual test: Ship construction items show ship name in queue
- [ ] Visual test: auto_build_ships checkbox appears for colony-source routes
- [ ] Visual test: Old saves load cleanly (migrate_stationed_ships runs)

---

# Bug Fixes: Save Menu, Contracts, Trade Routes

## Changes
- [x] **Save menu centering**: Save file buttons now fill available width using `add_sized()` instead of wrapping in `ui.horizontal` inside `vertical_centered`, which caused left-alignment
- [x] **Contract replenishment on cancel**: Both editor and management cancel handlers now call `refill_pool()` so new contracts appear immediately
- [x] **Trade route alert retry**: Removed the early-continue on `alert_reason` in `check_automation()` — routes now retry every cycle instead of getting permanently stuck. Alert still fires notification only on first failure (via `had_alert` guard)
- [x] **Remove ManualLaunch**: Removed Launch button from fleet overview UI, removed `ManualLaunch` variant from `TradeAction`, removed handler from `handle_trade_action()`. Launches are now fully automatic
- [x] **Quiet notifications**: Removed `ShipDeparted` and `ShipArrived` notification pushes from `launch_ship()` and `process_arrival()`. Only error notifications (RoutePaused) remain

## Files Changed
- `src/main.rs` — save button sizing, contract refill on cancel (2 sites), remove ManualLaunch handler
- `src/render/trade_ui.rs` — remove Launch button from fleet overview
- `src/render/types.rs` — remove ManualLaunch variant from TradeAction
- `src/colony/trade.rs` — remove alert_reason skip, remove ShipDeparted/ShipArrived notifications, cleanup unused params

## Verification
- [x] `cargo build` — compiles clean, zero warnings
- [ ] Visual test: save names centered in load dialog
- [ ] Visual test: cancel contract → new contract appears in pool
- [ ] Visual test: trade route with insufficient funds → alert appears → add funds → next cycle launches successfully
- [ ] Visual test: no Launch button in fleet overview
- [ ] Visual test: no departure/arrival toast notifications during time warp

---

# Keplerian Circular Orbits for Procedural Stars

## Implementation
- [x] Step 1: Replace `vel: [f64; 2]` with `galactic_r`, `theta_0`, `omega` on ProceduralStar (galaxy/mod.rs)
- [x] Step 2: Compute orbital elements in `generate_sector()` (galaxy/generation.rs)
- [x] Step 3: Backward rotation for sector lookup + per-star Kepler propagation in `build_procedural_star_data()` (main.rs)
- [x] Step 4: Build verification + spec creation

## Files Changed
- `src/galaxy/mod.rs` — ProceduralStar: replaced `vel` with `galactic_r`, `theta_0`, `omega`
- `src/galaxy/generation.rs` — Compute `galactic_r`, `theta_0`, `omega` from position and circular velocity
- `src/main.rs` — `build_procedural_star_data()`: backward rotation for sector lookup, per-star Kepler propagation
- `openspec/specs/game/flight_rendering/galaxy/spec.md` — NEW: procedural star field spec

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: galaxy view → zoom in on star field → increase time warp → stars co-rotate with Sun
- [ ] Visual test: focus on body at different galactic radius → stars show differential rotation
- [ ] Visual test: stars remain consistent when zooming in/out
- [ ] Visual test: no stars popping in/out at sector boundaries

---

# Elliptical Orbits for Procedural Stars

## Implementation
- [x] Step 1: Replace 3 circular-orbit fields with 5 elliptical fields on ProceduralStar, add `solve_kepler_nr()` and `kepler_position()` helpers (galaxy/mod.rs)
- [x] Step 2: Generate eccentricity from Rayleigh distribution, derive consistent orbital elements preserving t=0 position (galaxy/generation.rs)
- [x] Step 3: Replace circular propagation with Kepler equation solving, increase sector margin for radial drift (main.rs)
- [x] Step 4: Build verification + spec update

## Files Changed
- `src/galaxy/mod.rs` — ProceduralStar: replaced `galactic_r`/`theta_0`/`omega` with `semi_major_axis`/`mean_motion`/`mean_anomaly_0`/`eccentricity`/`arg_periapsis`; added `solve_kepler_nr()`, `kepler_position()`
- `src/galaxy/generation.rs` — Rayleigh eccentricity generation with radius-dependent σ, element derivation preserving t=0 position
- `src/main.rs` — `build_procedural_star_data()`: elliptical propagation via `kepler_position()`, margin = max(1 sector, 20% radius)
- `openspec/specs/game/flight_rendering/galaxy/spec.md` — Updated: elliptical elements, eccentricity generation, Kepler helpers

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: galaxy view — stars co-rotate with Sun, show differential rotation
- [ ] Visual test: with time warp — stars show slight radial oscillation, not perfectly circular paths
- [ ] Visual test: at t=0 — star positions identical to before (pos unchanged)
- [ ] Visual test: no stars popping in/out at sector boundaries (margin handles drift)

---

# Evolved Stellar Types in Procedural Star Generation

## Implementation
- [x] Add secondary evolution roll after spectral type selection in `generation.rs`
- [x] Spec update — evolved types section in galaxy spec

## Files Changed
- `src/galaxy/generation.rs` — Secondary evolution roll: ~6% white dwarfs, ~2% red giants, ~0.01% supergiants, ~0.1% neutron stars
- `openspec/specs/game/flight_rendering/galaxy/spec.md` — Evolved stellar types requirement

## Verification
- [ ] `cargo build` — compiles clean
- [ ] Visual test: galaxy view shows mostly dim red M-dwarfs + occasional bright red/orange dots (red giants)
- [ ] Visual test: rare very bright dots (supergiants) visible
- [ ] Visual test: white dwarfs appear as dim blue-white scatter
- [ ] Visual test: star field looks richer/more varied than before

---

# Fix Star Rendering: Performance Near Sgr A* + Nearby Star Visibility

## Implementation
- [x] Cap sector star count — `MAX_STARS_PER_SECTOR = 2000` in `sector_star_count()` (density.rs)
- [x] Lower min star radius — 0.1 ly threshold (was 1 ly), clamp query radius to 50 ly minimum
- [x] Remove density bail — no more bail-to-empty behavior
- [x] Distance-ordered sectors — sort sectors by distance from camera, iterate closest first
- [x] MAX_STARS cap — stop at 50k stars instead of bailing to empty
- [x] Spec update — updated rendering requirements in galaxy spec
- [x] Build verification — compiles clean

## Files Changed
- `src/galaxy/density.rs` — `.min(MAX_STARS_PER_SECTOR)` cap on star count
- `src/main.rs` — Rewritten `build_procedural_star_data()`: min radius 0.1 ly, star_radius clamped to 50 ly, no density bail, distance-ordered sector sort, MAX_STARS cap
- `openspec/specs/game/flight_rendering/galaxy/spec.md` — Updated rendering requirements

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: near Sgr A* — no lag, stars render (closest 50k shown)
- [ ] Visual test: near Sun, zoomed in — nearby stars visible at intermediate zoom
- [ ] Visual test: near Sun, zoomed out — same as before (all nearby stars shown)
- [ ] Visual test: stars render closest-first (no distant stars before near ones)

---

# Star Catalog: 67 Named Star Systems

## Implementation
- [x] Task 1: Create `src/galaxy/catalog.rs` — CatalogSystem/CatalogStar/CatalogBody/CatalogAtmosphere structs, build_catalog_stars() builder, lookup_system() lookup, spectral_temperature/spectral_to_star_type helpers
- [x] Task 2: Create `src/galaxy/catalog/catalog_data.rs` — Static CATALOG array with all 67 systems (318 bodies, ~4700 lines) transcribed from docs/nearby_stars.md
- [x] Task 3: Integrate catalog into galaxy mod.rs — catalog_index field on ProceduralStar, catalog_by_sector on GalaxyState, injection in get_sector() with 2 ly dedup
- [x] Task 4: Update rendering pipeline — catalog_name/catalog_index on StarRenderData, format_name() returns real name for catalog stars
- [x] Task 5: Add planetary system info to info panel — CatalogPlanetInfo struct, catalog fields on BodyInfoData, system info + planetary system sections in menus.rs
- [x] Task 6: Build, test, and update specs — cargo build clean, galaxy spec updated with catalog sections

## Files Created
- `src/galaxy/catalog.rs` — Data structures, builder, lookup (~204 lines)
- `src/galaxy/catalog/catalog_data.rs` — All 67 systems with planets/moons (~4700 lines)

## Files Modified
- `src/galaxy/mod.rs` — pub mod catalog, catalog_index on ProceduralStar, catalog_by_sector on GalaxyState, get_sector() injection
- `src/galaxy/generation.rs` — catalog_index: 0 on procedural stars
- `src/render/scene.rs` — catalog_name/catalog_index on StarRenderData, format_name()
- `src/render/types.rs` — CatalogPlanetInfo struct, catalog fields on BodyInfoData
- `src/render/mod.rs` — CatalogPlanetInfo re-export
- `src/render/menus.rs` — System info + planetary system sections in info panel
- `src/main.rs` — catalog name lookup in build_procedural_star_data/lookup_focused_star, catalog data in focused star info builder
- `openspec/specs/game/flight_rendering/galaxy/spec.md` — Catalog stars section

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: galaxy view → zoom to Sol → named stars appear at correct positions
- [ ] Visual test: hover catalog star → real name shown (e.g. "Alpha Centauri")
- [ ] Visual test: focus catalog star → info panel shows system info + planetary system
- [ ] Visual test: Zone 5 stars appear near Sgr A*
- [ ] Visual test: procedural stars still work normally

---

# Fix: Catalog Star Positions, Planet Indicators, and Z-Ordering

## Implementation
- [x] Fix 1: Correct catalog star positions — compute Sun's t=0 galactic position from orbital elements, place each non-Zone-5 star at `sun_pos + distance_ly * direction`, recompute M₀ from corrected position
- [x] Fix 2: Catalog planet indicator rings — add `game_time` param to `add_procedural_stars_impl`, draw planet indicator rings at orbital positions before star dots (z-order), color by habitability/life
- [x] Fix 3: Solar system star-on-top z-ordering — two-pass indicator rendering in `add_body_vertices`, defer root body indicator to render last (on top), extract `draw_ring_indicator` helper
- [x] Build verification — compiles clean

## Files Changed
- `src/galaxy/catalog.rs` — `build_catalog_stars()`: compute Sun's t=0 position via `kepler_position()`, place non-Zone-5 stars at correct distance, recompute M₀
- `src/render/scene.rs` — `add_procedural_stars_impl()`: `game_time` param, planet indicator rings for catalog stars; `add_body_vertices()`: deferred root body indicator; `draw_ring_indicator()` helper; `update_bodies_orbits_ship_and_vessels()`: `game_time` param
- `src/main.rs` — Pass `game.time()` to all 6 `update_bodies_orbits_ship_and_vessels()` call sites
- `openspec/specs/game/flight_rendering/galaxy/spec.md` — Catalog positioning + planet indicators
- `openspec/specs/game/flight_rendering/bodies/spec.md` — Root body indicator z-ordering

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: Alpha Centauri appears ~4.37 ly from Sun, Sirius ~8.6 ly
- [ ] Visual test: catalog stars show planet indicator rings at orbital positions
- [ ] Visual test: planet indicators spread out when zooming into a catalog star system
- [ ] Visual test: star indicator ring always renders on top of planet indicators
- [ ] Visual test: clicking near catalog star focuses the star (not a planet)
- [ ] Visual test: solar system Sun indicator on top when all bodies are indicator-sized

---

# Multi-Star Systems, Exoplanet Info, Orbit Segments

## Implementation
- [x] Step 1: Add `segments` field to `OrbitRenderData` — 256 for solar system, 5120 for catalog orbits
- [x] Step 2: Add `StarOrbitData` struct and `binary_orbits` field to `CatalogSystem`, populate 19 multi-star systems
- [x] Step 3: Add `host_star_index()` helper to determine which star a planet orbits from its designation
- [x] Step 4: Rewrite `inject_catalog_planets` for multi-star positioning — companion stars placed via mass-ratio barycenter offsets, planets routed to correct host star
- [x] Step 5: Build `BodyInfoData` for catalog planets during injection, store in `catalog_body_info` HashMap
- [x] Step 6: Update tracking station panel to check `catalog_body_info` for exoplanet info display
- [x] Step 7: Verify star hover labels (already working via `hovered_star_label` in flight.rs and menus.rs)
- [x] Step 8: Update specs

## Files Changed
- `src/galaxy/catalog.rs` — `StarOrbitData` struct, `binary_orbits` field on `CatalogSystem`, `host_star_index()` helper
- `src/galaxy/catalog/catalog_data.rs` — `binary_orbits` data for all 67 systems (19 populated, 48 empty)
- `src/render/types.rs` — `segments: u32` field on `OrbitRenderData`
- `src/render/scene.rs` — Use `orbit.segments` instead of hardcoded 256 in all 4 orbit rendering paths
- `src/render/state.rs` — `catalog_body_info: HashMap<usize, BodyInfoData>` field
- `src/render/menus.rs` — Check `catalog_body_info` when `body_info.get(idx)` returns None for tracked catalog planets
- `src/main.rs` — Rewritten `inject_catalog_planets` with multi-star positioning, companion star bodies, planet host routing, BodyInfoData construction; `spectral_temperature` helper; `segments: 256/5120` on all OrbitRenderData constructions
- `openspec/specs/game/flight_rendering/bodies/spec.md` — Updated orbit segment spec
- `openspec/specs/game/flight_rendering/galaxy/spec.md` — Multi-star rendering, exoplanet info panel, catalog data structures

## Verification
- [x] `cargo build` — compiles clean, no warnings
- [ ] Visual test: Alpha Centauri shows 3 stars orbiting a barycenter
- [ ] Visual test: A's planets orbit A, B's planets orbit B, Proxima's planets orbit Proxima
- [ ] Visual test: orbit lines are smooth at high zoom (5120 segments)
- [ ] Visual test: clicking an exoplanet shows info panel with name, description, radius, gravity, atmosphere
- [ ] Visual test: single-star systems still work correctly

---

# Fix Multi-Star System Visual and Gameplay Bugs

## Implementation
- [x] Task 1: Fix procedural star dot overlap — skip rendering dot for focused multi-star systems in `add_procedural_stars_impl()`, still record screen position for hit testing
- [x] Task 2: Fix re-centering to use primary star (star[0]) — simplified re-centering logic in `inject_catalog_planets()`
- [x] Task 3: Add group-level orbit lines — collect `GroupOrbit` data during hierarchy loop, emit orbit-only entries for multi-member group barycenters
- [x] Task 4: Fix Castor data — added Ca-Cb pair (0.018 AU), fixed A-B separation (100 AU, not 1100), added AB-C pair (1100 AU)
- [x] Task 5: Fix Epsilon Indi data — added Ba-Bb pair (2.5 AU), fixed brown dwarf masses (0.070/0.047 M☉) and radii (0.08/0.07 R☉)
- [x] Task 6: Audit other multi-star systems — Alpha Centauri, EZ Aquarii, Gliese 667C, Fomalhaut, 40 Eridani, Regulus all verified correct
- [x] Task 7: Update spec — procedural dot suppression, star[0] re-centering, group orbit lines, new scenarios

## Files Changed
- `src/render/scene.rs` — `focused_star` param on `add_procedural_stars_impl()`, skip rendering for focused multi-star systems
- `src/main.rs` — `GroupOrbit` struct, group orbit collection in hierarchy loop, simplified re-centering on star[0], group orbit line emission
- `src/galaxy/catalog/catalog_data.rs` — Castor binary_orbits (3→5 entries), Epsilon Indi binary_orbits (1→2 entries), brown dwarf mass/radius fixes
- `openspec/specs/game/flight_rendering/galaxy/spec.md` — Updated multi-star rendering: dot suppression, star[0] re-centering, group orbit scenarios

## Verification
- [x] `cargo build` — compiles clean
- [ ] Visual test: Regulus — no duplicate dot, WD visible, B/C visible with orbits, wide group orbit lines
- [ ] Visual test: Castor — all 6 stars visible with correct hierarchy
- [ ] Visual test: 40 Eridani — star A at system dot position
- [ ] Visual test: Epsilon Indi — Ba and Bb visible with inner orbit
- [ ] Visual test: Alpha Centauri — A near system dot, B nearby, Proxima distant
- [ ] Visual test: simple binaries (Sirius, Procyon) still work correctly

---

# Integration Test Suite

## Implementation
- [x] Visibility changes: `pub mod formatting` in render/mod.rs, `pub fn` on all formatting functions, `pub fn` on `calculate_orbit_from_state` and `mean_to_true_anomaly` in ship/orbit.rs
- [x] `tests/common/mod.rs` — Shared helpers: constants, `make_orbit`, `assert_close`, `assert_relative`, `make_solar_system`
- [x] `tests/formatting.rs` — 15 tests: format_duration, format_distance, format_mass, format_power_si, format_pressure, blackbody_color, apply_heat_tint
- [x] `tests/orbital_mechanics.rs` — 13 tests: Kepler solver roundtrips (circular, moderate, high, hyperbolic), state vector ↔ elements roundtrips, known-value checks (Earth orbit, galactic mass, SOI, atmosphere)
- [x] `tests/transfer.rs` — 7 tests: Lambert solver (near-Hohmann, degenerate, edge cases), normalize_angle, Hohmann Earth→Moon
- [x] `tests/relativistic.rs` — 9 tests: Lorentz gamma at various speeds, gravitational time dilation, relativistic cruise velocity
- [x] `tests/galaxy.rs` — 5 tests: deterministic generation, Sun exclusion zone, density model sanity (solar neighborhood, edge, center)

## Files Created
- `tests/common/mod.rs`
- `tests/formatting.rs`
- `tests/orbital_mechanics.rs`
- `tests/transfer.rs`
- `tests/relativistic.rs`
- `tests/galaxy.rs`

## Files Modified
- `src/render/mod.rs` — `mod formatting` → `pub mod formatting`
- `src/render/formatting.rs` — all `pub(crate) fn` → `pub fn`
- `src/ship/orbit.rs` — `calculate_orbit_from_state` and `mean_to_true_anomaly`: `pub(crate)` → `pub`

## Verification
- [x] `cargo test` — 53 tests pass (4 existing + 49 new), zero failures
- [x] `cargo build` — compiles clean, no regressions
- [x] `cargo test --test formatting` — single module works independently
