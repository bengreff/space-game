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
