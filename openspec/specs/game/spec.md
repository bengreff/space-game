# Game

Game-wide requirements shared across all modes (editor and flight).

## Game Modes

### Requirement: Two game modes

The system SHALL support two game modes: **Editor** and **Flight**, represented by the `GameMode` enum.

#### Scenario: Game starts in editor
- **WHEN** the application launches
- **THEN** the game mode SHALL be `Editor`

#### Scenario: Mode switching
- **WHEN** the player launches a vessel from the editor
- **THEN** the game mode SHALL switch to `Flight`

## Coordinate System

### Requirement: Double-precision positions

All physics positions SHALL use `f64` (double precision) to prevent precision loss at large distances. Rendering vertices SHALL use `f32`, with conversion happening at render time.

### Requirement: Grid square size

Each grid square SHALL measure 0.5m x 0.5m, defined by `GRID_SQUARE_SIZE = 0.5`. All part dimensions in grid squares are converted to meters by multiplying by this constant.

## Part Size System

### Requirement: Five part sizes

The system SHALL support five part sizes with the following grid widths:

| Size   | Grid Width | Meters |
|--------|-----------|--------|
| Tiny   | 1         | 0.5m   |
| Small  | 3         | 1.5m   |
| Medium | 5         | 2.5m   |
| Large  | 9         | 4.5m   |
| XL     | 13        | 6.5m   |

## Part Categories

### Requirement: Six part categories

The system SHALL support six part categories in this order: Pods ("Command"), Propulsion ("Engines"), FuelTanks ("Fuel Tanks"), Structural ("Structural"), Aerodynamic ("Aerodynamic"), Utility ("Utility").

### Requirement: Root part eligibility

Only parts with category `Pods` SHALL be eligible as root parts, as determined by `can_be_root()`.

## Propellant System

### Requirement: Three propellant types

The system SHALL support three propellant types, each pairing an oxidizer (LOX) with a fuel:

| Propellant | Display    | Fuel Type | O2 (kg/sq) | Fuel (kg/sq) |
|-----------|-----------|-----------|------------|-------------|
| Kerolox   | LOX/RP-1  | Rp1       | 470.0      | 185.0       |
| Methalox  | LOX/CH4   | Methane   | 270.0      | 75.0        |
| Hydrolox  | LOX/LH2   | Hydrogen  | 155.0      | 25.0        |

### Requirement: Propellant capacity scaling

Tank propellant capacity SHALL scale linearly with `grid_area`. A tank with grid area N stores `N * oxygen_per_sq` kg of oxidizer and `N * fuel_per_sq` kg of fuel.

### Requirement: Tank dry mass

Tank dry mass SHALL be 35.0 kg per grid square of area.

## Part Definitions

### Requirement: RON-based part definitions

Part definitions SHALL be loaded from RON files in the `data/parts/` directory. Each file contains a `PartDefinitionFile` with a `parts` Vec of `PartDefinition` structs. Parts are keyed by their `id` string in the `PartDefinitions` registry.

### Requirement: Part definition fields

Each `PartDefinition` SHALL include: `id`, `name`, `description`, `category`, `mass` (dry, in tonnes), `cost`, `size`, `shape`, `grid_width` (visual, decimal), `grid_height` (visual, decimal), optional `top_width` (for trapezoids), optional `hitbox_width`/`hitbox_height` overrides, and optional component data (`engine`, `tank`, `pod`, `decoupler`, `fairing`, `rcs`).

### Requirement: Part shapes

The system SHALL support three part shapes: `Rectangle`, `Triangle` (base at bottom), and `Trapezoid` (with `top_width` and bottom width).

### Requirement: Wet mass calculation

A part's wet mass SHALL equal `dry_mass + sum(resource_values_kg) / 1000.0`, where resources are stored in kg and mass in tonnes.

## Hitbox System

### Requirement: Three hitbox types

Each part SHALL have three distinct bounds:

| Type           | Purpose              | Dimensions                                      |
|---------------|---------------------|------------------------------------------------|
| Visual sprite  | What is drawn        | `grid_width * 0.5m` x `grid_height * 0.5m` (decimal) |
| Build hitbox   | Collision detection  | `hitbox_grid_width` x `hitbox_grid_height` (integer grid squares, defaults to `ceil(visual)`) |
| Weld hitbox    | Connection detection | Build hitbox * 1.05 (5% padding via `WELD_HITBOX_PADDING`) |

## Rendering Pipeline

### Requirement: wgpu colored-triangle rendering

All rendering SHALL use wgpu with a custom vertex shader (`shader.wgsl`) and 4x MSAA. The vertex format is `Vertex { position: [f32; 2], color: [f32; 4] }` -- everything is colored triangles with no textures.

## UI Framework

### Requirement: egui immediate-mode UI

All UI SHALL be rendered using egui 0.27 in immediate mode. The egui integration SHALL consume input events before game input handlers, preventing input passthrough when the pointer is over UI.

## Component Data

### Requirement: Engine data

Engine definitions SHALL include: `thrust_vac` (kN), `thrust_asl` (kN), `isp_vac` (s), `isp_asl` (s), `throttleable` (bool), `gimbal_range` (degrees, 0 = fixed), and `propellant` type.

### Requirement: Pod data

Pod definitions SHALL include: `crew_capacity` and `torque` (reaction wheel torque).

### Requirement: Decoupler data

Decoupler definitions SHALL include: `ejection_force` (kN).

### Requirement: Fairing data

Fairing definitions SHALL include: `ejection_force` (kN, used when jettisoning the fairing shell).

### Requirement: Tank data

Tank definitions SHALL include: `grid_area` for capacity calculation.
