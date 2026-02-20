# Craft Collision

Collision detection between the active vessel and inactive vessels (debris, jettisoned stages).

## Requirements

### Requirement: Bounding circle collision during physics warp

During physics warp (time_warp <= RAILS_WARP_THRESHOLD, i.e. 1x through 10x), the system SHALL check for collisions between the active vessel and each inactive vessel in the same SOI. Collision is detected when the distance between vessel positions is less than the sum of their bounding extents (max of bounding_half_height and bounding_half_width).

### Requirement: Collision response

When a collision is detected, the active vessel's velocity is set to match the inactive vessel's stored velocity. This effectively stops the active vessel from passing through the debris. Only the first collision per frame is processed (break after first hit).

### Requirement: On-rails exclusion

Collision detection is NOT performed during on-rails warp (time_warp > RAILS_WARP_THRESHOLD, i.e. 100x and above). On-rails vessels use Keplerian propagation and collision detection would be unreliable at high time steps.

## Implementation
- Collision check runs in `render_flight_frame()` after inactive vessel propagation
- Uses `vessel.bounding_half_height()` and `vessel.bounding_half_width()` for extents
- Only checks vessels in the same SOI (`soi_body` match)
