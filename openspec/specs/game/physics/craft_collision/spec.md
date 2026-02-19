# Craft Collision

Collision detection between vessels (ships and debris). Not yet implemented.

## Future Requirements

### Requirement: Vessel-to-vessel collision detection

The system SHALL detect collisions between multiple vessels in flight. When two vessels' bounding volumes overlap, per-part hitbox intersection checks SHALL determine the collision.

### Requirement: Collision response

On collision between vessels, the system SHALL apply appropriate physics response (elastic/inelastic collision, part destruction, debris generation). Specific collision response behavior is TBD.
