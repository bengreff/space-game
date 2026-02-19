# Staging

Stage management, ordering, drag-and-drop, and delta-v calculation in the vehicle editor.

## Data Structure

### Requirement: Stage representation

Stages SHALL be represented as `Vec<Vec<PlacedPartId>>` where the outer Vec is the ordered list of stages and the inner Vec contains the part IDs in each stage. Stage index 0 fires first in flight.

### Requirement: Auto-staging on placement

When an engine or decoupler is placed, it SHALL be automatically added to stage 0. If no stages exist, a new stage SHALL be created first. No other part categories are auto-staged.

### Requirement: Auto-staging on blueprint load

When loading a blueprint with empty stages, all engine parts SHALL be collected into a single stage 0. Decouplers are NOT auto-staged during this fallback.

## Staging Panel UI

### Requirement: Reverse display order

Stages SHALL be displayed in reverse index order: highest stage index at top, lowest at bottom. Stage labels SHALL be 1-indexed in the UI (e.g., "Stage 1" for index 0).

### Requirement: Part names in stages

Each part in a stage SHALL display its `PartDefinition.name`. If the definition is not found, it SHALL display "Part {id}".

### Requirement: Mirrored pair display

When a part and its `mirror_partner` are both in the same stage, they SHALL be displayed as a single entry with " x2" appended to the part name. Drag-reordering a mirrored entry in staging SHALL move both parts together.

#### Scenario: Mirrored pair shows as single entry

- **WHEN** engine A and its mirror partner engine B are both in stage 0
- **THEN** the staging panel SHALL show one entry: "Engine Name x2"

#### Scenario: Only one partner in stage

- **WHEN** engine A is in stage 0 but its mirror partner engine B is in stage 1
- **THEN** each stage SHALL show its respective engine as a normal entry (no x2 badge)

### Requirement: Empty stage display

Empty stages SHALL display "(empty)" in weak/gray style.

### Requirement: Per-stage delta-v display

Each stage with delta-v > 0 SHALL display its delta-v value to the right of the stage label, formatted in green (`rgb(120, 200, 120)`) at font size 10.

### Requirement: Total delta-v display

The total delta-v across all stages SHALL be displayed at the top of the staging panel as "Total dv: {value}" in bold at font size 12. It SHALL also appear in the stats bar.

## Drag and Drop

### Requirement: Part drag between stages

Dragging a part from one stage and dropping it onto another stage SHALL remove the part from all stages and append it to the target stage.

### Requirement: Stage reordering

Dragging a stage and dropping it onto another stage's position SHALL move the stage to that position. The insertion index SHALL be adjusted if the source was before the target.

### Requirement: Insert stage gaps

Between each pair of stages (and at top/bottom), a "+" button SHALL be rendered inside a drop zone. Clicking "+" SHALL insert a new empty stage at that position. Dropping a stage onto a gap SHALL move it to that position.

### Requirement: Stage deletion

Each stage SHALL have an "X" button. Clicking it SHALL remove the stage; parts in the deleted stage become unstaged.

### Requirement: Single action per frame

Only one staging action SHALL be processed per frame, in priority order: stage move via gap, stage insert, stage delete, part/stage drop onto stage zone.

## Delta-V Calculation

### Requirement: Tsiolkovsky rocket equation

Per-stage delta-v SHALL be calculated using the Tsiolkovsky equation: `dv = Isp * g0 * ln(wet_mass / dry_mass)`, where `g0 = 9.80665 m/s^2`.

### Requirement: Sequential stage simulation

Delta-v calculation SHALL simulate stages sequentially in order. For each stage:

1. **Fire decouplers**: Mark decoupler and all parts with top edge at or below the decoupler's bottom edge (0.01m tolerance) as decoupled.
2. **Enable engines**: Mark non-decoupled engines in this stage as active.
3. **Compute wet mass**: Sum mass + remaining fuel for all non-decoupled parts.
4. **Compute Isp**: Thrust-weighted average of `isp_vac` across all active non-decoupled engines.
5. **Apply Tsiolkovsky**: Calculate delta-v from wet/dry mass ratio.
6. **Consume fuel**: Set all non-decoupled tank fuel to 0.

#### Scenario: Zero delta-v cases
- **WHEN** Isp <= 0, dry mass <= 0, or wet mass <= dry mass
- **THEN** the stage delta-v SHALL be 0.0

### Requirement: Fuel tracking initialization

Fuel remaining per tank SHALL be initialized from `propellant_capacity(fuel_type)` (oxidizer + fuel mass), converted from kg to tonnes. Only tanks with `tank_filled == true` and `fuel_type != Empty` SHALL contribute fuel.
