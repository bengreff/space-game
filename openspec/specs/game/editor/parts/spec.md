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

### Requirement: Mirror ghost preview

When symmetry mode is Mirror and a palette part is selected, the editor SHALL display two ghost previews: the primary ghost at the cursor position (grid-snapped) and a mirrored ghost reflected across the center line. The mirrored ghost X position SHALL be `center_line_x * 2 - primary_x`. The mirrored ghost Y position SHALL equal the primary ghost Y position.

#### Scenario: Mirror ghost positions

- **WHEN** symmetry mode is Mirror, center line is at X = 2.0, and the cursor snaps to `[3.5, 1.0]`
- **THEN** the primary ghost SHALL be at `[3.5, 1.0]` and the mirrored ghost SHALL be at `[0.5, 1.0]`

#### Scenario: Mirror ghost on center line

- **WHEN** symmetry mode is Mirror and the primary ghost snaps to the center line X coordinate
- **THEN** only one ghost SHALL be displayed (the part is on the center line, no mirror needed)

### Requirement: Mirror ghost validity

Both mirror ghosts SHALL share the same validity state. The ghost pair SHALL be valid only when NEITHER ghost overlaps any existing part. Both ghosts SHALL render in green when valid and red when invalid.

#### Scenario: One ghost overlaps

- **WHEN** symmetry mode is Mirror, the primary ghost has no overlap, but the mirrored ghost overlaps an existing part
- **THEN** both ghosts SHALL render in red (invalid) and placement SHALL be blocked

#### Scenario: Both ghosts clear

- **WHEN** symmetry mode is Mirror and neither ghost overlaps any existing part
- **THEN** both ghosts SHALL render in green (valid) and placement SHALL be allowed

## Placement

### Requirement: Left-click placement

Left-clicking on empty space with a valid ghost SHALL place the selected part at the ghost position. Placement requires: a palette part is selected, the ghost position exists, and the ghost is valid.

### Requirement: First part becomes root

The first part placed SHALL become the root part, regardless of its category.

### Requirement: Auto-stage engines and decouplers

When an engine or decoupler is placed, it SHALL be automatically added to stage 0. If no stages exist, a new stage SHALL be created.

### Requirement: Mirror part placement

When symmetry mode is Mirror and placement is valid, left-clicking SHALL place two parts simultaneously: the primary part at the ghost position and the mirrored part at the mirrored position. Both parts SHALL be assigned unique `PlacedPartId` values and linked via `mirror_partner` fields pointing to each other.

#### Scenario: Mirror placement creates linked pair

- **WHEN** the user clicks to place a part in Mirror mode at `[3.5, 1.0]` with center line at X = 2.0
- **THEN** two parts SHALL be placed: one at `[3.5, 1.0]` and one at `[0.5, 1.0]`, each with `mirror_partner` pointing to the other's ID

#### Scenario: Center line placement creates single part

- **WHEN** the user clicks to place a part in Mirror mode and the ghost is on the center line
- **THEN** only one part SHALL be placed with `mirror_partner = None`

#### Scenario: Mirror placement auto-staging

- **WHEN** two engines are placed via mirror placement
- **THEN** both engines SHALL be added to stage 0

### Requirement: Monotonic part IDs

Part IDs SHALL be monotonically increasing, starting from 1 and incrementing on each placement.

## Selection

### Requirement: Click to select placed part

Left-clicking on a placed part SHALL select it and clear any palette selection. If the clicked part has a `mirror_partner`, both parts SHALL render with the selected color.

#### Scenario: Select mirrored part highlights both

- **WHEN** the user clicks on a placed part that has a mirror partner
- **THEN** both the clicked part and its mirror partner SHALL render with the selected color `[0.5, 0.7, 1.0, 1.0]`

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

### Requirement: Linked part info panel

When a part with a `mirror_partner` is selected, changes made in the part info panel SHALL apply to both parts. This includes fuel type selection, fill/empty toggle, and crossfeed toggle.

#### Scenario: Fuel type change applies to both

- **WHEN** the user changes the fuel type of a selected tank that has a mirror partner
- **THEN** both the selected tank and its mirror partner SHALL update to the new fuel type

### Requirement: Delete button for placed parts

The info panel for placed parts SHALL include a "Delete Part" button. If the part has a `mirror_partner`, the button label SHALL read "Delete Parts (x2)" and clicking it SHALL delete both parts.

#### Scenario: Delete button for mirrored pair

- **WHEN** a mirrored part is selected and the user clicks the delete button
- **THEN** both the selected part and its mirror partner SHALL be deleted

## Dragging

### Requirement: Drag grid snapping

During a drag, the part position SHALL snap to the grid using the same rules as ghost snapping. If the part has a `mirror_partner`, the partner SHALL move to the mirrored position relative to the center line. Both positions update in real-time.

#### Scenario: Drag mirrored pair

- **WHEN** a part with a mirror partner is dragged to a new position
- **THEN** the partner SHALL move to the position reflected across the center line

### Requirement: Drag overlap detection

During a drag, overlap SHALL be checked against all parts except the one being dragged. When dragging a mirrored pair, overlap SHALL be checked for both parts, excluding both from the overlap check. Invalid drag positions SHALL show a red overlay on both parts.

#### Scenario: Mirrored drag overlap

- **WHEN** a mirrored pair is being dragged and the partner's mirrored position overlaps another part
- **THEN** both parts SHALL show a red overlay indicating invalid position

### Requirement: Revert on invalid drop

When a drag finishes at an overlapping position, the part SHALL revert to its pre-drag position. When dragging a mirrored pair, if either position is invalid, both parts SHALL revert to their pre-drag positions.

#### Scenario: Mirrored drag revert

- **WHEN** a mirrored pair drag finishes with the partner overlapping another part
- **THEN** both parts SHALL revert to their pre-drag positions

### Requirement: Right-click cancels drag

Right-clicking during a drag SHALL cancel the drag and revert the part to its pre-drag position.

## Deletion

### Requirement: Delete key

Pressing Delete or Backspace with a placed part selected SHALL delete that part. If the part has a `mirror_partner`, both parts SHALL be deleted.

#### Scenario: Delete mirrored pair via keyboard

- **WHEN** the user presses Delete with a mirrored part selected
- **THEN** both the selected part and its mirror partner SHALL be removed

### Requirement: Right-click delete

Right-clicking with a placed part selected (no palette selection, no drag active) SHALL delete that part. If the part has a `mirror_partner`, both parts SHALL be deleted.

#### Scenario: Right-click delete mirrored pair

- **WHEN** the user right-clicks with a mirrored part selected
- **THEN** both the selected part and its mirror partner SHALL be removed

### Requirement: Stage cleanup on delete

Deleting a part SHALL remove it from all stages. When deleting a mirrored pair, both parts SHALL be removed from all stages. Empty stages SHALL be removed.

#### Scenario: Stage cleanup for mirrored pair

- **WHEN** a mirrored pair is deleted and both were in stage 0
- **THEN** both parts SHALL be removed from stage 0 and stage 0 SHALL be removed if empty

### Requirement: Root reassignment on delete

If the deleted part was the root, the root SHALL be reassigned to any remaining part. If no parts remain, root becomes `None`.

## Mirror Partner Data Model

### Requirement: Mirror partner field

`PlacedPart` SHALL have a `mirror_partner: Option<PlacedPartId>` field. When two parts are mirror-placed, each SHALL reference the other. Parts placed without mirroring or on the center line SHALL have `mirror_partner = None`.

#### Scenario: Bidirectional link

- **WHEN** part A (ID=5) and part B (ID=6) are mirror-placed
- **THEN** part A's `mirror_partner` SHALL be `Some(6)` and part B's `mirror_partner` SHALL be `Some(5)`

## Deselection

### Requirement: Escape deselection

Pressing Escape SHALL deselect the palette part if one is selected, otherwise deselect the placed part. Escape does NOT delete the placed part.

### Requirement: Right-click deselection

Right-clicking with a palette part selected (no drag active) SHALL deselect the palette part and clear the ghost.
