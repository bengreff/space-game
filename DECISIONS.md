# Design Decisions

## Part Placement System

**Decision:** Grid-based placement with hitbox welding (NOT attachment nodes like KSP)

### Three Hitbox Types:

Each part has three separate areas (sometimes overlapping):

1. **Build/Flight Hitbox** - Determines:
   - Whether parts can be placed (no overlap allowed)
   - How parts are centered on the grid
   - Collision detection in flight
   - Can be different from visual size (e.g., small engine visual is 2x2 but hitbox is 3x2)

2. **Welding Hitbox** - Extends 5% past the build/flight hitbox
   - If welding hitboxes intersect while building, parts are welded together in flight
   - Allows parts to connect when placed adjacent

3. **Visual Sprite** - What is actually drawn
   - Can be smaller than hitbox (e.g., trapezoid engine inside rectangular hitbox)

### How placement works:
1. Each part's build/flight hitbox occupies a certain number of grid squares
2. Parts cannot be placed if their build/flight hitboxes would overlap
3. Parts are centered based on hitbox dimensions, not visual dimensions
4. No explicit attachment points - parts connect automatically based on welding hitbox proximity

### Benefits:
- Simpler mental model for players
- More flexible part arrangements
- No need to match attachment point sizes
- Parts naturally connect when placed adjacent to each other

### Grid Alignment Rules (based on HITBOX dimensions):
- **Odd width/height** (1, 3, 5...): Center on the middle of a grid square
- **Even width/height** (2, 4, 6...): Center on a grid line (between squares)

This ensures parts can always attach flush to each other regardless of dimensions.

## Part Sizes

Part sizes denote their width in grid squares:
- **Tiny:** 1 wide
- **Small:** 3 wide
- **Medium:** 5 wide
- **Large:** 9 wide

## Current Parts

### Pods
- **Small Command Pod:** Triangle, 3 wide at base

### Fuel Tanks
- **Small Fuel Tank 1:** Square, 3x3

### Engines
- **Small Engine:** Visual: Trapezoid 2x2 (1 wide at top, 2 wide at base), Hitbox: 3x2
