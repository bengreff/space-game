# Flight HUD

Flight HUD panels: time warp, orbital info, autopilot, vessel stats, velocity/altitude, throttle, fuel, staging.

## Time Warp Panel

### Requirement: Time warp panel at top of screen

An egui top panel named "time_warp_panel" SHALL display time warp controls and orbital information.

#### Scenario: Time warp button labels
- Warp levels: `[1, 2, 3, 5, 10, 100, 1000, 10000, 100000, 1000000, 10000000, 100000000, 1000000000]` (13 levels)
- Labels: `>= 1,000,000,000` -> "{X}B", `>= 1,000,000` -> "{X}M", `>= 1,000` -> "{X}K", else "{X}x"
- Current warp level shown as selected via `egui::SelectableLabel`

#### Scenario: Warp blocking while thrusting
- **WHEN** throttle > 0.0 AND ship has positive thrust acceleration
- **THEN** warp levels where warp > `RAILS_WARP_THRESHOLD` (10.0) SHALL be disabled

#### Scenario: Warp blocking near SOI boundary
- **WHEN** `time_to_intercept / warp_rate < 0.5` seconds of real time
- **THEN** that warp level SHALL be disabled

#### Scenario: Warp blocking below landing altitude
- **WHEN** ship is below the current SOI body's landing altitude
- **THEN** warp levels where warp > `RAILS_WARP_THRESHOLD` (10.0) SHALL be disabled (greyed out)

#### Scenario: Current warp display
- After separator: "Current: {warp}x" where warp is cast to i64

## Orbital Information

### Requirement: Orbital information display

Below time warp buttons, orbital information SHALL be shown horizontally when the ship has orbit data.

#### Scenario: Orbital info fields
- "Ap: {altitude}" "({time_to_ap})" | "Pe: {altitude}" "({time_to_pe})" | "T: {period}" | "e: {eccentricity:.3}" | "{soi_body_name}"

#### Scenario: Distance formatting
- `>= 1e9` -> "X.XX Gm", `>= 1e6` -> "X.X Mm", `>= 1e3` -> "X.X km", else "X m"

#### Scenario: Time formatting
- `>= 86400 * 365` -> "X.Xy", `>= 86400` -> "X.Xd", `>= 3600` -> "X.Xh", `>= 60` -> "X.Xm", else "Xs"

## Bottom Panel

### Requirement: Bottom panel with autopilot and vessel stats

An egui bottom panel named "flight_info_panel" with background `rgba(20, 20, 30, 200)` SHALL show autopilot buttons, vessel stats, and velocity/altitude in a horizontal row.

### Requirement: Autopilot (SAS) buttons

The SAS system SHALL display buttons for: "PRO" (Prograde), "RET" (Retrograde), "R-" (RadialIn), "R+" (RadialOut), and "MAN" (ManeuverNode, only when a maneuver node is selected).

#### Scenario: Button colors
- Active fill = `rgb(80, 150, 80)`, inactive = `rgb(60, 60, 70)`
- Active text = white, inactive text = light gray
- Button text size = 11, minimum size = 35x20

#### Scenario: Toggle behavior
- Clicking active button toggles to Off; clicking inactive button activates that mode

### Requirement: Vessel stats display

The bottom panel SHALL show: mass ("M: {mass:.2}t"), thrust ("T: {thrust:.0}kN"), TWR ("TWR: {value:.2}" green if >= 1.0, red if < 1.0), delta-v ("Dv: {formatted}"), and G-force ("G: {value:.1}" color-coded: white < 3g, yellow < 6g, red >= 6g). G-force SHALL use atmospheric-adjusted thrust (interpolated between vacuum and sea-level values based on atmospheric pressure fraction), not vacuum thrust.

### Requirement: Atmospheric thrust and TWR adjustment

Thrust and TWR displayed in the bottom panel SHALL be adjusted for atmospheric pressure. The thrust SHALL be linearly interpolated between vacuum and sea-level values based on the atmospheric pressure fraction at the ship's current altitude: `thrust = thrust_vac * (1 - pressure) + thrust_asl * pressure`, where pressure is `atmosphere.pressure_at_altitude(alt) / 101325` clamped to [0, 1]. TWR SHALL use this atmospheric thrust value. Delta-v estimates SHALL always use vacuum ISP.

### Requirement: Velocity and altitude display

Right side of bottom panel SHALL show: "VEL" label with value at font size 13 strong, and "ALT" label with value at font size 13 strong. When `heat_fraction > 0.01`, a temperature readout ("{temp}K") SHALL appear after altitude, colored by severity: yellow (`< 0.33`), orange (`< 0.66`), red (`>= 0.66`).

#### Scenario: Velocity formatting
- `>= 1000` -> "{X.XX} km/s", else "{X.X} m/s"

#### Scenario: Altitude formatting
- `>= 1e9` -> "{X.XX} Gm", `>= 1e6` -> "{X.XX} Mm", `>= 1e3` -> "{X.XX} km", else "{X.X} m"

## Throttle Bar

### Requirement: Throttle bar on left side panel

An egui left panel named "throttle_panel" with width 50 and background `rgba(20, 20, 30, 200)`.

#### Scenario: Throttle bar layout
- "THR" label at font size 10, percentage at font size 12 strong
- Vertical bar: height 150px, width 20px, fill from bottom up

#### Scenario: Throttle bar colors
- Background = `rgb(40, 40, 50)`, border = gray 1.0px
- Fill: `< 0.5` -> green `rgb(100, 200, 100)`, `< 0.8` -> yellow `rgb(200, 200, 100)`, `>= 0.8` -> red `rgb(200, 100, 100)`

### Requirement: Fuel bar below throttle

When vessel has fuel data, show fuel bar with 10px gap below throttle.

#### Scenario: Fuel bar colors
- `> 0.3` -> blue `rgb(80, 160, 220)`, `> 0.1` -> amber `rgb(220, 180, 80)`, `<= 0.1` -> red `rgb(220, 80, 80)`
- Bar height 80px, width 20px

### Requirement: Heat bar below fuel bar

When ship temperature exceeds 350K, a vertical heat bar SHALL be shown:
- "HEAT" label at font size 10, temperature readout in Kelvin at font size 11
- Bar height 80px, width 20px, fill from bottom proportional to `heat_fraction`
- Colors: `< 0.33` -> yellow `rgb(220, 200, 80)`, `< 0.66` -> orange `rgb(220, 140, 40)`, `>= 0.66` -> red `rgb(220, 60, 60)`
- Background `rgb(40, 40, 50)`, border gray 1px

### Requirement: Stage indicator below fuel/heat bars

When vessel has stages, show "STG" label and "{current}/{total}" at font size 12 strong.

## Staging Panel

### Requirement: Staging panel on right side

When vessel has stages, an egui right panel named "flight_staging_panel" with default width 150 SHALL show the staging panel.

#### Scenario: Header
- "Staging" heading, total delta-v if > 0, "Active: {current}/{total}" in gray

#### Scenario: Stage display order
- Reverse order (highest index at top), 1-based labels
- Activated stages use dark gray label, unactivated use white

#### Scenario: Per-stage delta-v
- Green `rgb(120, 200, 120)` at font size 10
- `>= 1000` -> "{X.X}km/s", else "{X.0}m/s"

#### Scenario: Drag-and-drop
- Stage headers are drag sources for reordering
- Parts within stages are drag sources for moving between stages
- "+" buttons between stages insert empty stages
- "X" button deletes stage

## Flight Part Info

### Requirement: Flight part info popup

When a flight part is selected, an egui Window with the part's name SHALL show part details.

#### Scenario: Engine info
- Thrust (vac/ASL), ISP (vac/ASL), propellant type
- Status: "Active" (green), "No Fuel" (amber), "Disabled" (red)
- Activate/Deactivate toggle button

#### Scenario: Tank info
- Fuel type, oxidizer progress bar (`rgb(80, 140, 200)`), fuel progress bar (`rgb(200, 160, 60)`)

#### Scenario: Pod info
- Crew capacity

#### Scenario: Decoupler info
- Enable/Disable Crossfeed toggle, "Decouple" button
