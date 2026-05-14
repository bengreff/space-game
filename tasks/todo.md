# Code Review Fixes (April 2026)

Following the full-codebase code review. Phase A is complete. This document now plans the remaining phases.

## Phase A — production fixes (COMPLETE)

- [x] **#1 `editor/state.rs:797`** — Already safe: bounds check present at line 796; function has zero callers. No change needed.
- [x] **#2 `main.rs:509`** — Already safe: `ship/mod.rs` sub-steps physics at `MAX_PHYSICS_DT = 0.01` with `MAX_SUBSTEPS = 1000`; on-rails uses Keplerian. Existing `dt.clamp(0.0001, 0.1)` plus substepping adequately bounds per-frame work. Retracted.
- [x] **#3 `colony/simulation.rs:453-466`** — Deleted per-type degradation averaging. Each building's degradation is now independent; starved siblings no longer drag down healthy ones. Spec updated.
- [x] **#4 `parts/blueprint.rs:296-304`** — Sorted by `PlacedPartId` before assigning indices so blueprint RON output is byte-identical across runs. Spec updated (persistence/spec.md).
- [x] **#5 `colony/simulation.rs:108-140`** — Merged two-pass check-then-remove into a single atomic pass using `ResourceStorage::remove` (Fission) and `remove_all` (Fusion). Reactors competing for the same fuel pool now degrade gracefully in iteration order. Spec updated.
- [x] **#6 Mk IV Particle Accelerator circumference scaling** — Added `BuildingType::size_multiplier(body_radius_m)` and applied it at all four consumption sites. Spec updated (core_loop + foundation).

Phase A review: 4 real fixes landed, 2 retracted as already-safe, 3 specs updated, 54 tests passing.

---

## Phase A+ — physics bugs (COMPLETE)

Researched each item against actual code. Only #8 is a real bug.

- [x] **#8 `parts/vessel.rs:3048-3062` + `2553-2562`** — VERIFIED. When a decoupler fires, the y-partition computes each other part's hitbox top as `local_position[1] + hitbox_height/2.0`, but omits `hitbox_y_offset`. Engines (the only parts with a non-zero `hitbox_y_offset`, used to shift the flight hitbox vertically relative to the build hitbox) therefore get a miscomputed top, and can end up on the wrong side of a decoupler. Two call sites need the same fix:
  ```rust
  let other_top = part.local_position[1]
      + part.hitbox_y_offset
      + hitbox_half_height;
  ```
  ~8 LOC total. Risk: medium — affects staging behavior, so test with a standard multi-stage rocket that has engines directly above a decoupler. Update `openspec/specs/game/vessels/staging/spec.md` (or closest match) to document that the partition uses hitbox center + half-extent including offset.

- [x] **#7 `ship/soi.rs:138-154`** — FALSE ALARM. Binary search converges correctly: when `test_dist >= soi_radius` (outside), `lo` advances toward later times; when inside, `hi` retreats. Retracted.
- [x] **#9 `ship/orbit.rs:168-207`** — FALSE ALARM. `acos(-1.0/e)` with `e > 1` always has input in `(-1, 0)`; `!max_ta.is_finite()` check provides defense in depth. Retracted.
- [x] **#10 `ship/patched_conics.rs:96, 151`** — FALSE ALARM. Both sites are guarded by `if cos_nu_exit.abs() <= 1.0`, with a mathematically correct asymptote fallback when `p > soi_radius`. Retracted.

Phase A+ workflow:
1. Read `parts/vessel.rs` around both sites to confirm current state
2. Apply identical fix to both, factoring out a small `fn hitbox_top_y(part: &FlightPart, defs) -> f64` helper if it reduces duplication
3. `cargo build --quiet` + `cargo test`
4. Manually test: build a Kerolox rocket with [pod → decoupler → engine → tank → decoupler → engine → tank], launch, stage, verify the correct half falls away
5. Update spec

---

## Phase B — structural refactors (COMPLETE)

Four real refactors, in order of increasing risk. B4 (dead `Game::update`) was already resolved — no such function exists.

### B0 — trivial cleanup (COMPLETE)

- [x] **B0.1 Delete dead `update_bodies()` stub** in `src/render/scene.rs:~2706`. Unused wrapper, ~50 LOC. Pure deletion, no spec impact.

### B1 — RenderState request queue (COMPLETE)

Replace 13 `*_request: Option<T>` fields on `RenderState` with a single `Vec<RenderRequest>` queue. Main.rs drains the queue once instead of 13 separate `if let Some(..).take()` blocks (currently at `src/main.rs:2683-2920`).

Current fields (from `src/render/state.rs`):
1. `staging_reorder: Option<Vec<Vec<usize>>>`
2. `engine_toggle_request: Option<(usize, bool)>`
3. `crossfeed_toggle_request: Option<(usize, bool)>`
4. `decouple_request: Option<usize>`
5. `fairing_deploy_request: Option<usize>`
6. `solar_deploy_request: Option<(usize, bool)>`
7. `parachute_deploy_request: Option<usize>`
8. `parachute_cut_request: Option<usize>`
9. `transfer_node_request: Option<(f64, f64, f64, f64)>`
10. `establish_colony_request: Option<usize>`
11. `transfer_cargo_request: Option<usize>`
12. `open_colony_request: Option<usize>`
13. `debug_teleport_body: Option<usize>`

Explicitly NOT moved: `debug_teleport_leo: bool` — it's a toggled state flag, not a request.

Steps:
1. Add `pub enum RenderRequest { ... }` in `src/render/state.rs` with one variant per current field
2. Add `pub request_queue: Vec<RenderRequest>` to `RenderState`; add helper `pub fn push_request(&mut self, req: RenderRequest)`
3. Migrate producers (UI code that sets `*_request = Some(..)`) to `push_request(RenderRequest::Variant(..))` — one file at a time, building after each
4. Replace the 13 drain blocks in main.rs with a single `for req in render_state.request_queue.drain(..) { match req { ... } }`
5. Remove the old fields
6. `cargo build --quiet` after each file, `cargo test` at the end
7. No spec change needed (pure internal refactor)

LOC: ~+110 net, but the main.rs drain becomes one match instead of 13 if-lets. Risk: low (no behavior change). Risk hotspot: ordering — if two request types need to run in a specific order (e.g., stage decouple before stage reorder), preserve that ordering via the enum match arms, not queue insertion order.

### B2 — move non-flight `render_*_frame` functions out of main.rs (COMPLETE)

`main.rs` is 6318 LOC. Seven non-flight render functions total 3027 LOC and should move to existing render modules. Leaves main.rs at ~3300 LOC (still large, but dominated by the core `render_flight_frame` at 2473 LOC and top-level event-loop logic).

| Function | Main.rs lines | LOC | Target |
|---|---|---|---|
| `render_editor_frame` | 2973-4288 | 1316 | `src/render/editor_render.rs` (exists) |
| `render_title_screen_frame` | 4288-4547 | 260 | `src/render/menus.rs` |
| `render_main_menu_frame` | 4547-4698 | 152 | `src/render/menus.rs` |
| `render_tracking_station_frame` | 4698-5044 | 347 | `src/render/menus.rs` (or new `tracking.rs`) |
| `render_colony_frame` | 5044-5369 | 326 | `src/render/colony_ui.rs` |
| `render_colony_overview_frame` | 5369-5482 | 114 | `src/render/colony_overview_ui.rs` |
| `render_management_frame` | 5482-5589 | 108 | `src/render/management_ui.rs` |
| `render_tech_tree_frame` | 5589-6318 | 730 | `src/render/tech_tree_ui.rs` |

`render_flight_frame` stays in main.rs — it's the hot path, tightly coupled to input dispatch, and moving it yields marginal benefit.

Steps (one function per commit so each move is reviewable/revertable):
1. Identify all `main.rs` helpers the function depends on; either pass them as params or move them along
2. Create `pub fn render_*_frame(...)` in the target module with the moved body
3. Update main.rs dispatch to call the new module path
4. `cargo build --quiet` + `cargo test`
5. Commit
6. Next function

Risk notes:
- Some frames touch a lot of `RenderState` internals; moving them may force adding `pub(crate)` to previously private fields — accept this, it's better than leaving 1800 LOC in main
- Watch for dependency cycles (render module calling back to game.rs types is fine; game.rs calling render modules is not)
- `render_editor_frame` is the biggest (1316 LOC) and the riskiest; do it last so the easier wins are banked first

No spec impact (pure code organization).

### B3 — scene.rs decomposition (COMPLETE)

`src/render/scene.rs` is 2905 LOC in one file with 14 functions. Decompose into a `src/render/scene/` submodule tree:

```
src/render/scene/
├── mod.rs              (~300 LOC — re-exports, small shared helpers)
├── orbits.rs           (~150 LOC — update_bodies_with_orbits, ellipse/line gen)
├── bodies.rs           (~250 LOC — add_body_vertices, draw_ring_indicator, add_launchpad_vertices)
├── atmosphere.rs       (~150 LOC — add_atmosphere_vertices, falloff encoding)
├── accretion_disc.rs   (~130 LOC — add_accretion_disc_vertices)
├── procedural_stars.rs (~300 LOC — add_procedural_stars_impl, add_galactic_orbit_line)
├── galaxy_bg.rs        (~70 LOC — add_galaxy_texture_quad)
└── integration.rs      (~1500 LOC — update_bodies_orbits_ship_and_vessels, the big state sync)
```

The monster is `update_bodies_orbits_ship_and_vessels` at ~1700 LOC. Don't try to split it — leave it in `integration.rs`. Splitting that function internally is a separate follow-up effort if needed.

Steps:
1. Do B0.1 first (delete `update_bodies` stub)
2. Create `src/render/scene/mod.rs`, move shared helpers + re-exports
3. Extract each submodule one at a time, smallest first:
   a. `galaxy_bg.rs` (smallest, easiest)
   b. `accretion_disc.rs`
   c. `atmosphere.rs`
   d. `orbits.rs`
   e. `bodies.rs`
   f. `procedural_stars.rs`
   g. `integration.rs` (last, biggest, riskiest)
4. Each extraction: `cargo build --quiet` + `cargo test` before moving on
5. Final commit: delete the old `src/render/scene.rs`
6. No spec impact (pure organization)

Risk: medium. Hazards: circular module deps, lots of `impl RenderState` blocks (keep them unified via `use` re-exports in mod.rs), easy to break builds mid-refactor. Mitigation: incremental commits, one submodule at a time.

### B5 — per-frame clone elimination in `flight.rs` (COMPLETE)

`src/render/flight.rs` has 17 `.clone()` calls; 7–9 are per-frame hot-path clones used to work around egui closure lifetimes:

- Line 31: `bodies_copy` (full solar system snapshot)
- Line 41: `ship_orbit` (OrbitRenderData)
- Line 73: `flight_parts_cache` (Vec<ShipPartRenderData>)
- Lines 74-76: `ap_markers`, `pe_markers`, `pending_orbit_click`
- Line 78: `maneuver_nodes` (Vec<ManeuverNode>)
- Lines 84-86: `vessel_stages`, `vessel_stage_delta_vs`, `vessel_stage_burn_times`

Steps:
1. Profile first (or at least confirm these are hot) — `cargo build --release` and use `puffin` / `tracy` if integrated, otherwise inspect with `cargo flamegraph` on a 60-second flight session
2. For each clone, determine if egui actually requires ownership. Many egui APIs accept references; the `.clone()`s may be copy-paste cargo cult from when egui had stricter requirements.
3. Refactor to pass `&[T]` / `&T` where possible. If egui genuinely needs `'static` ownership in a closure, investigate `Arc<[T]>` as a middle ground.
4. Benchmark before/after to confirm improvement
5. `cargo test` to confirm no regressions

Risk: medium. Hazard: egui lifetime errors cascading through the file. If a clone turns out to be load-bearing, leave it — don't fight the borrow checker for nothing. The goal is to eliminate the easy 60% and document why the remainder must clone.

No spec impact.

### Skipped from original deferred list

- **B4 dead `Game::update`** — Already resolved: no such function exists in `src/game.rs`. The game loop calls individual update methods directly from main.rs. Nothing to delete.
- **B6 Galaxy catalog Rc** — Only 1 clone in the entire galaxy module, and sectors are regenerated on demand from PRNG (not cached in a way that causes repeated cloning). Not a real bottleneck. Skip.

---

## Colony Gap Items — Deferred

These features were identified in a gap analysis of `docs/colonies.md` vs implementation. Deferred for future implementation.

- [ ] **Return cargo / round-trip trade routes** — currently 1-way only (correct for now)
- [ ] **Multi-hop routes with refueling waypoints**
- [x] **Mass Driver system** — Mk I-IV electromagnetic mass drivers, mirror segment production, Dyson swarm
- [ ] **Orbital stations / gas giant scooping colonies**
- [ ] **Ship Part Manufacturing factory recipe**
- [ ] **Map icons for in-transit trade ships**
- [ ] **Launch window scheduling** — immediate vs optimal window choice UI

---

## Phase C — minor/low-priority items

Do these opportunistically, in any order. None block further work.

- [ ] **C1 RenderState `debug_teleport_leo: bool`** — Currently a toggled flag (not in the request queue from B1). Either (a) leave it as a flag for consistency with other debug toggles, or (b) fold it into `RenderRequest::TeleportToLeo` as part of B1. Recommendation: leave as flag. ~0 LOC or ~5 LOC depending on choice.
- [ ] **C2 Split the 1700-LOC `update_bodies_orbits_ship_and_vessels`** — Post-B3 follow-up. Split into per-cache sync functions (`sync_ship_cache`, `sync_vessel_cache`, `sync_maneuver_cache`, etc.). Only attempt after B3 is stable. Medium-large effort.
- [ ] **C3 Per-module rustdoc** — Each `src/*/mod.rs` should have a top-of-file `//!` doc comment describing its role. Currently sparse. Small, low-value but helpful for new contributors.
- [x] **C4 Integration test for reactor atomicity** — Phase A #5 fixed the atomicity bug but has no test. Add a test: colony with 3 Fission reactors and 1 kg Uranium; tick; verify exactly 2 reactors fueled, 0.0 kg remaining. ~20 LOC in an existing test file under `tests/`.
- [x] **C5 Integration test for blueprint determinism** — Phase A #4 made blueprint output deterministic but has no test. Add a test: place 10 parts, call `parts_to_blueprint` twice, assert the serialized RON is byte-identical. ~30 LOC.
- [x] **C6 Integration test for independent degradation** — Phase A #3 removed the averaging bug but has no test. Add a test: 2 Habitats, starve only one, tick 30 days, assert degradations differ. ~30 LOC.

---

## Execution order (if budget permits)

1. **Phase A+ #8** (1 real physics bug) — smallest, highest certainty
2. **B0.1** delete dead `update_bodies` stub — 10 min warmup
3. **Phase C tests C4/C5/C6** — lock in Phase A behavior before further refactoring
4. **B1** request queue refactor — low risk, frees main.rs of 13 drain blocks
5. **B2** move `render_*_frame` functions — 7 commits, low risk, biggest LOC win in main.rs
6. **B3** scene.rs decomposition — medium risk, biggest organizational win
7. **B5** flight.rs clone elimination — medium risk, perf-focused, do after structural work
8. **C2** split the big sync function — optional follow-up to B3
9. **C1, C3** — opportunistic polish

Each Phase B item is a natural commit boundary. Do not batch phases; commit after each numbered item.

---

## Workflow per fix

1. Read the file(s) to confirm exact current state — never edit code you haven't read in the current session
2. Make the smallest possible edit that addresses the item
3. `cargo build --quiet` after each fix, not batched
4. `cargo test` after structural refactors (Phase B)
5. Update corresponding spec under `openspec/specs/game/` in the same commit if behavior changed
6. Commit with a clear message referencing the item (e.g., "Fix decoupler y-partition to account for hitbox_y_offset (A+ #8)")
7. Mark task complete, move to next

## Review (Phase A)

**Phase A: complete.** 6 of 6 items addressed; all tests green (`cargo test`: 54 passed, 0 failed); full `cargo build --quiet` clean.

- **Real fixes landed: 4** (#3 degradation averaging, #4 blueprint determinism, #5 reactor atomicity, #6 Mk IV scaling)
- **Retracted as already-safe: 2** (#1 bounds check already present, #2 substepping already bounds per-frame work)
- **Spec updates: 3** — `colony/core_loop/spec.md` (reactor atomicity, independent degradation), `colony/foundation/spec.md` (size_multiplier), `editor/persistence/spec.md` (deterministic part ordering)
- **Files touched:** `src/colony/simulation.rs`, `src/colony/buildings.rs`, `src/parts/blueprint.rs`, `src/render/colony_ui.rs`, `src/render/menus.rs`, `src/main.rs`
- **New public API:** `BuildingType::size_multiplier(body_radius_m) -> f64`
- **Signature changes (breaking):** `Colony::can_queue_building` and `Colony::queue_building` now take `body_radius_m: f64`; `render_colony` and `render_colony_screen` take `body_radii: &[f64]`

## Review (Phase A+)

**Phase A+: complete.** 1 real bug fixed, 3 retracted as false alarms; all tests green.

- **Real fix: 1** (#8 decoupler hitbox_y_offset — both call sites in vessel.rs)
- **Retracted: 3** (#7 SOI binary search correct, #9 acos(-1/e) safe, #10 cos_nu_exit guarded)

## Review (Phase B)

**Phase B: complete.** 5 structural refactors landed; build clean, all 144 tests pass.

- **B0.1** — Deleted dead `update_bodies()` stub from scene.rs
- **B1** — Replaced 13 `*_request: Option<T>` fields with `RenderRequest` enum + `Vec<RenderRequest>` queue; main.rs drains via single match
- **B2** — Moved 8 non-flight `render_*_frame` functions (1585 LOC) into `src/frames.rs` binary-crate submodule; main.rs reduced from 6302 → 4717 lines
- **B3** — Decomposed `src/render/scene.rs` into `scene/` directory with 3 submodules: `bodies.rs` (308 LOC), `effects.rs` (223 LOC), `galaxy.rs` (330 LOC); mod.rs retains the orchestrator (1866 LOC)
- **B5** — Eliminated 12 per-frame `.clone()` calls in `flight.rs` by cloning `egui::Context` (cheap Arc bump) into a local, releasing the self borrow so the closure can reference `self` fields directly

## Review (Phase C — partial)

- **C4, C5, C6** — Integration tests added for reactor atomicity, blueprint determinism, and independent degradation
- **C1, C2, C3** — Remaining; low priority, opportunistic
