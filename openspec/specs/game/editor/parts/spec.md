# Parts

Part selection, placement, dragging, deletion, and info display in the vehicle editor.

## Part Palette

### Requirement: Category tabs

The parts palette SHALL display selectable tabs for each `PartCategory`. The default selected category SHALL be `Pods`.

### Requirement: Size grouping

Within a selected category, parts SHALL be grouped by `PartSize` under collapsible headers (default open). Sizes with no parts in the current category SHALL be hidden.

### Requirement: Palette selection toggle

Clicking a part in the palette SHALL select it. Clicking an already-selected part SHALL deselect it. Selecting a palette part SHALL clear any placed-part selection.

## Ghost Preview

### Requirement: Ghost follows cursor

When a palette part is selected, a ghost preview SHALL appear at the cursor position, updated on every mouse move via `update_ghost()`.

### Requirement: Ghost grid snapping

The ghost SHALL snap to the grid using hitbox dimensions: parts with odd hitbox width snap to grid square centers (`floor(x / grid) * grid + grid/2`), parts with even hitbox width snap to grid lines (`floor(x / grid + 0.5) * grid`). X and Y axes snap independently.

### Requirement: Ghost validity coloring

The ghost SHALL render in green (`[0.3, 0.9, 0.3, 0.4]`) when placement is valid and red (`[0.9, 0.3, 0.3, 0.4]`) when invalid. For procedurally-rendered parts (engines, pods, decouplers), the part renders at alpha 0.5 with a validity overlay on top.

### Requirement: Ghost overlap check

The ghost SHALL be marked invalid when its hitbox AABB overlaps any existing part's hitbox AABB. Touching edges (exact boundary contact) SHALL NOT count as overlap.

## Placement

### Requirement: Left-click placement

Left-clicking on empty space with a valid ghost SHALL place the selected part at the ghost position. Placement requires: a palette part is selected, the ghost position exists, and the ghost is valid.

### Requirement: First part becomes root

The first part placed SHALL become the root part, regardless of its category.

### Requirement: Auto-stage engines and decouplers

When an engine or decoupler is placed, it SHALL be automatically added to stage 0. If no stages exist, a new stage SHALL be created.

### Requirement: Monotonic part IDs

Part IDs SHALL be monotonically increasing, starting from 1 and incrementing on each placement.

## Selection

### Requirement: Click to select placed part

Left-clicking on a placed part SHALL select it and clear any palette selection.

### Requirement: Hover highlight

Moving the cursor over a placed part SHALL highlight it. Hit testing SHALL use hitbox dimensions (not visual dimensions) for AABB point-in-rect testing.

## Part Info Panel

### Requirement: Info panel visibility

The part info panel SHALL appear when either a palette part or a placed part is selected.

### Requirement: Common part info

The info panel SHALL display: name, description, size, mass (tonnes and kg), cost, and grid dimensions for all parts.

### Requirement: Engine info

For engines, the info panel SHALL display: propellant type, thrust (vacuum and sea level), specific impulse (vacuum and sea level), gimbal range (or "Fixed" if 0), throttleable status, and single-engine vacuum TWR on Earth.

### Requirement: Tank info for palette selection

For tanks selected in the palette, the info panel SHALL display: dry mass, grid area, and propellant capacity for all three fuel types.

### Requirement: Tank info for placed parts

For placed tanks, the info panel SHALL additionally display: fuel type selector buttons, fill/empty toggle, progress bars for oxidizer and fuel amounts, and dry/propellant/total mass breakdown.

### Requirement: Pod info

For pods, the info panel SHALL display: crew capacity and reaction wheel torque.

### Requirement: Decoupler info for placed parts

For placed decouplers, the info panel SHALL display a "Fuel Crossfeed" checkbox toggling `crossfeed_enabled`.

### Requirement: Delete button for placed parts

The info panel for placed parts SHALL include a "Delete Part" button.

## Dragging

### Requirement: Drag grid snapping

During a drag, the part position SHALL snap to the grid using the same rules as ghost snapping. The position updates in real-time.

### Requirement: Drag overlap detection

During a drag, overlap SHALL be checked against all parts except the one being dragged. Invalid drag positions SHALL show a red overlay (`[0.9, 0.2, 0.2, 0.4]`).

### Requirement: Revert on invalid drop

When a drag finishes at an overlapping position, the part SHALL revert to its pre-drag position.

### Requirement: Right-click cancels drag

Right-clicking during a drag SHALL cancel the drag and revert the part to its pre-drag position.

## Deletion

### Requirement: Delete key

Pressing Delete or Backspace with a placed part selected SHALL delete that part.

### Requirement: Right-click delete

Right-clicking with a placed part selected (no palette selection, no drag active) SHALL delete that part.

### Requirement: Stage cleanup on delete

Deleting a part SHALL remove it from all stages. Empty stages SHALL be removed.

### Requirement: Root reassignment on delete

If the deleted part was the root, the root SHALL be reassigned to any remaining part. If no parts remain, root becomes `None`.

## Deselection

### Requirement: Escape deselection

Pressing Escape SHALL deselect the palette part if one is selected, otherwise deselect the placed part. Escape does NOT delete the placed part.

### Requirement: Right-click deselection

Right-clicking with a palette part selected (no drag active) SHALL deselect the palette part and clear the ghost.

## Symmetry

### Requirement: Symmetry mode cycling

The symmetry mode SHALL cycle through Off, x2, x3, x4, x6, x8 via the R key or the toolbar button. The current mode SHALL be displayed in the toolbar.

#### Scenario: Symmetry mode display
- **WHEN** the symmetry mode is `Radial4`
- **THEN** the toolbar SHALL display "x4"

### Requirement: Symmetry not yet functional

The symmetry mode is stored and displayed but SHALL NOT affect part placement (single part placed regardless of mode). This is a planned feature.
