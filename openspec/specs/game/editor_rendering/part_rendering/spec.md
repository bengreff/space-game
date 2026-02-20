# Part Rendering

Vertex generation for generic (non-procedural) placed parts by shape.

## Colors

### Requirement: Part color scheme

Placed parts SHALL use the following colors:
- Default: `[0.4, 0.4, 0.45, 1.0]`
- Selected: `[0.5, 0.7, 1.0, 1.0]`
- Hovered: `[0.55, 0.55, 0.6, 1.0]`

Mirror partners SHALL share the same visual state as the primary selected/hovered part.

## Shape Rendering

### Requirement: Rectangle part rendering

Rectangle parts SHALL be rendered as two triangles forming a quad, using the part's visual width and height centered at the part position.

### Requirement: Triangle part rendering

Triangle parts SHALL be rendered as a single triangle with the base at the bottom and the apex at top center.

### Requirement: TriangleRight part rendering

TriangleRight parts SHALL be rendered as a single right triangle with vertices at bottom-left, bottom-right, and top-right (vertical edge on the right side, hypotenuse on the left). Used for right-side booster nose cones.

### Requirement: TriangleLeft part rendering

TriangleLeft parts SHALL be rendered as a single right triangle with vertices at bottom-left, bottom-right, and top-left (vertical edge on the left side, hypotenuse on the right). Used for left-side booster nose cones.

### Requirement: Trapezoid part rendering

Trapezoid parts SHALL be rendered as two triangles forming a quad with `width` at the bottom edge and `top_width` at the top edge.

## Overlays

### Requirement: Invalid drag overlay for generic parts

When a generic part is being dragged to an invalid position, a red overlay (`[0.9, 0.2, 0.2, 0.4]`) SHALL be rendered on top of the part using the same shape geometry.

## Rendering Pipeline

### Requirement: Part rendering pass order

Parts SHALL be rendered in two passes: first pass draws all part shapes (engines, pods, decouplers, generic shapes), second pass draws decoupler adapter trapezoids. This ensures adapters render on top of adjacent parts.

### Requirement: Camera-relative part output

All part vertices SHALL be output in camera-relative coordinates by subtracting the camera offset from the world position of each part.
