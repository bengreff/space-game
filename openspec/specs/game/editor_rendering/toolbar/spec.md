# Toolbar

The top toolbar with vessel actions, symmetry mode, launch button, and part count.

### Requirement: Toolbar layout

The toolbar SHALL be a top panel containing: "Vehicle Editor" heading, separator, New/Save/Load buttons, separator, symmetry mode control, separator, Launch button, separator, Exit to Flight button, and right-aligned part count.

### Requirement: Launch button enabled state

The Launch button SHALL be enabled only when `can_launch()` returns true (root part exists and parts are non-empty). The launch button text SHALL be "Launch" (with rocket emoji prefix).

### Requirement: Launch control validation

Launching SHALL fail with an error if the vessel has no controllable part (no pod or probe core with `can_control: true`). The error message "Vessel has no controllable part (add a pod or probe core)" SHALL be displayed as a red alert banner at the top of the editor for 3 seconds.

### Requirement: Exit to Flight button

The toolbar SHALL include an "Exit to Flight" button that returns to flight mode without launching a new vessel.

### Requirement: Part count display

The toolbar SHALL display "Parts: {count}" right-aligned, showing the total number of placed parts.
