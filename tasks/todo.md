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
