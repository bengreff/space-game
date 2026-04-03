# Lessons Learned

## Spec-Code Coupling (2026-02-19)

**Mistake**: Implemented multiple features and bug fixes across a session, then tried to batch-update all spec files at the end. This resulted in code changes being committed without corresponding spec updates, and some changes being missed entirely.

**Rule**: Update the spec file as the LAST step of each individual task, before moving to the next task. The task is: implement code -> update spec -> done. Never defer spec updates.

## Coordinate Frame Consistency (2026-03-20)

**Mistake**: In `check_soi_transition_precise` (physics-mode SOI entry), computed ship position in absolute coordinates (`solar_system.body_position(soi_body) + rel_pos`) but compared against child body position in parent-relative coordinates (`get_body_position_at_time` returns orbit position relative to parent). The ~1 AU offset made child SOI entry impossible in physics mode.

**Rule**: When computing distances between objects, verify both positions are in the same reference frame. `rel_position` is relative to SOI body; `get_body_position_at_time` returns position relative to parent. When the SOI body IS the parent, both frames match — use `rel_pos - child_pos` directly without converting to absolute.

## Retrograde Mean Anomaly Propagation (2026-03-20)

**Mistake**: Used `departure_ma = (ship_ma + omega * t) * direction` which multiplies the entire accumulated mean anomaly by -1 for retrograde, instead of propagating with negative omega: `departure_ma = ship_ma + omega_signed * t`.

**Rule**: For retrograde orbit propagation, negate the rate (omega), not the result. Mean anomaly decreases over time for retrograde orbits.

## Hohmann Transfer on Eccentric Orbits (2026-03-20)

**Mistake**: Tried to fix Hohmann transfers on eccentric orbits by constraining departure to apsides. Two problems: (1) for near-circular orbits, `argument_of_periapsis` is arbitrary so the node was at a meaningless fixed angle; (2) for eccentric orbits, snapping to an apsis doesn't account for the actual transfer geometry.

**Rule**: When classical Hohmann assumptions break down (non-circular orbits), use the Lambert solver instead of patching formulas. Lambert naturally handles arbitrary eccentricities by solving for the exact velocity at the actual burn point. Use phase angle timing for the departure window, then Lambert for the exact delta-v.

## egui Borrow Checker Pattern (2026-03-21)

**Mistake**: Tried to call `self.render_colony_panel()` inside `self.egui_ctx.run()` closure. This fails with E0500 because the closure borrows `self` (via egui_ctx), and calling another `&mut self` method is a conflicting borrow.

**Rule**: For egui panels that need mutable access to RenderState fields, use free functions that take individual `&mut` parameters (e.g., `&mut bool`, `&mut Option<usize>`) instead of `&mut self` methods. Copy data out before the egui closure, pass borrowed fields to free functions inside the closure, then write results back after.

## Dual Kepler Solver Mismatch (2026-04-02)

**Mistake**: When fixing catalog star positioning, identified that `kepler_position()` uses a first-order approximation for e < 0.1 and wrote a matching inverse. But missed that `bodies.rs Orbit::position_at()` uses a DIFFERENT Kepler solver (exact Newton-Raphson for ALL eccentricities). The Sun's on-screen dot is rendered by bodies.rs, while catalog stars use galaxy::kepler_position. The ~106 ly offset between the two solvers at the Sun's orbital parameters (e=0.07, a=21,000 ly) completely dominated the catalog star distances (4-100 ly).

**Rule**: When two systems must agree on a position, verify they use the SAME math pipeline. Search for ALL callers of the relevant position computation, not just the one being fixed. In this codebase: bodies.rs has its own Kepler solver separate from galaxy/mod.rs.

## Check for Existing Layer 0 Code (2026-03-21)

**Mistake**: When implementing Layer 1, assumed all code needed to be written from scratch. Layer 0 had already implemented many stubs (establish_colony, update_colonies, simulation.rs, notification.rs, colony part, gas giant flag). This caused duplicate fields, methods, and imports.

**Rule**: Before implementing any task from a plan, search for existing implementations first. Run `grep` for function names, field names, and file names. Layer 0 data model work often includes stubs for Layer 1 functionality.
