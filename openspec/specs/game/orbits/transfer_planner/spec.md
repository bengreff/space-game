# Transfer Planner

## Overview

The transfer planner provides Hohmann and Lambert transfer calculations for planning interplanetary maneuvers. It is accessed via a UI window in flight mode.

## Modes

### Hohmann Mode
- Computes circular coplanar Hohmann transfers to sibling bodies (same SOI parent as ship).
- Displays departure delta-v, arrival delta-v, transfer time, phase angle, and time to window.
- "Create Node" places a prograde maneuver node at the computed departure position.

### Lambert Mode (Porkchop Plot)
- Computes interplanetary Lambert transfers to bodies orbiting the same grandparent (e.g., Earth→Mars via Sun).
- Displays a 2D **porkchop plot** grid (60 columns x 50 rows = 3000 points):
  - **Horizontal axis**: departure time (linear), spanning one synodic period between the departure and target bodies from current sim time. The synodic period is `2π / |ω_dep - ω_tgt|`, the full cycle of transfer windows.
  - **Vertical axis**: transfer time of flight (log scale), from `hohmann_tof / 100` to `hohmann_tof * 2`.
- Each cell is colored by ejection delta-v using log-scale normalization with a multi-stop gradient.
- The grid is computed on a **background thread** when a target is selected; recomputed when the target changes or when more than 10% of the departure window has elapsed (keeping the x-axis columns in the near future). "Computing..." is shown during computation.
- Departure body positions and velocities are precomputed per column to avoid redundant Kepler solves.
- **Interaction**:
  - Hover over the plot to preview a transfer point (highlighted cell, info shown below).
  - Click to lock selection to a point.
  - Default selection is the lowest delta-v point (marked with a white circle).
- The selected point's (departure_time, tof) is fed into `compute_interplanetary()` for full transfer details.
- If the selected departure time is in the past relative to `sim_time`, the time-to-node is advanced forward by full synodic periods to the next window with equivalent phase angle geometry.
- "Create Node" creates a maneuver node from the selected transfer.

## Target Selection

- **Hohmann targets**: Bodies that share the same SOI parent as the ship (e.g., Moon when orbiting Earth).
- **Lambert targets**: Bodies that orbit the same grandparent as the ship's SOI body (e.g., Mars when orbiting Earth, both orbit Sun).

## Delta-V Computation

### Porkchop Grid Points
For each grid point:
1. Get departure body position at `dep_time` and target body position at `dep_time + tof`.
2. Solve Lambert problem (universal variable formulation with Stumpff functions).
3. Compute v-infinity: `v_inf = v_lambert - v_planet`.
4. Compute ejection delta-v via vis-viva: `v_ejection = sqrt(v_inf^2 + 2*mu/r_parking)`, `dv = v_ejection - v_circular`.

### Full Interplanetary Computation (Selected Point)
1. Initial Lambert solve for approximate v-infinity.
2. Compute hyperbolic escape time from parking orbit to SOI boundary.
3. Re-solve Lambert with corrected departure time (at SOI exit) for accurate v-infinity direction.
4. Compute ejection angle using hyperbolic turn angle at finite SOI distance.

## Color Scale
- `normalized = (dv - min_dv) / (max_dv - min_dv)`, clamped to [0, 1].
- `max_dv` is capped at `3 * min_dv` to prevent outliers from washing out the gradient.
- HSV: `hue = 120 * (1 - normalized)`, saturation = 0.8, value = 0.9.

## Files
- `src/render/types.rs`: `PorkchopPoint`, `PorkchopGrid` data structures.
- `src/ship/transfer.rs`: `compute_porkchop_grid()`, `solve_lambert_2d()`, `compute_interplanetary()`.
- `src/render/state.rs`: Porkchop plot UI rendering, state fields (`porkchop_grid`, `porkchop_selected`, `porkchop_hovered`, `porkchop_last_target`, `porkchop_receiver`, `porkchop_computing`).
- `src/main.rs`: Orchestrates grid computation and selected-point evaluation.
