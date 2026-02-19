# Aerodynamic Heating

Convective aerodynamic heating, radiative cooling, thermal destruction, and heat visualization.

## Temperature Model

### Requirement: Ship temperature field

The `Ship` struct SHALL have a `temperature: f64` field (Kelvin) and a `heat_flux: f64` field (W/m^2). Initial/ambient temperature SHALL be `AMBIENT_TEMPERATURE = 300.0` K.

### Requirement: Temperature constants

The system SHALL define the following constants:
- `VESSEL_SPECIFIC_HEAT = 900.0` J/(kg*K) (aluminum)
- `STEFAN_BOLTZMANN = 5.670374419e-8` W/(m^2*K^4)
- `VESSEL_EMISSIVITY = 0.8`
- `VESSEL_DESTRUCTION_TEMP = 2000.0` K
- `HEAT_COEFFICIENT = 1.0e-4` (Sutton-Graves simplified coefficient)

## Heating

### Requirement: Convective heating (Sutton-Graves simplified)

Heat input SHALL be computed as `q_in = HEAT_COEFFICIENT * sqrt(density) * airspeed^3 * frontal_area`, where `frontal_area` is the same orientation-dependent cross-section used for drag.

### Requirement: Radiative cooling

Heat output SHALL be computed using Stefan-Boltzmann radiation: `q_out = EMISSIVITY * STEFAN_BOLTZMANN * T^4 * surface_area`, where `surface_area` is approximated as the vessel perimeter: `2 * (width*2 + height*2)`.

### Requirement: Temperature update per frame

Temperature SHALL be updated once per frame (after physics substeps) in `update_flying()`:
1. `dT = (q_in - q_out) / (mass_kg * specific_heat) * dt`
2. `temperature += dT`
3. Temperature SHALL be clamped to a minimum of `AMBIENT_TEMPERATURE`
4. `heat_flux = q_in / frontal_area` (stored for HUD display)

### Requirement: No-atmosphere cooling

When the SOI body has no atmosphere or the ship is above the atmosphere, `heat_flux` SHALL be 0.0 and temperature SHALL decay toward `AMBIENT_TEMPERATURE` using exponential decay: `temp += (AMBIENT - temp) * (1 - exp(-0.01 * dt))`.

### Requirement: On-rails cooling

While on-rails, temperature SHALL decay toward `AMBIENT_TEMPERATURE` using the same exponential decay formula. `heat_flux` SHALL be 0.0.

### Requirement: Temperature reset on launch

When launching from the editor, `ship.temperature` SHALL be reset to `AMBIENT_TEMPERATURE` and `ship.heat_flux` to 0.0.

## Thermal Destruction

### Requirement: Vessel destruction at critical temperature

After each physics update in `render_flight_frame()`, if `ship.temperature >= VESSEL_DESTRUCTION_TEMP` (2000K), the flight vessel SHALL be destroyed (`game.flight.vessel = None`) and temperature/heat_flux SHALL be reset to ambient.

## Heat Visualization

### Requirement: Heat fraction for rendering

`ShipRenderData` SHALL include `temperature: f64`, `heat_fraction: f32` (0.0 to 1.0), and `heat_flux: f64`. Heat fraction SHALL be computed as `((temperature - 300) / (2000 - 300)).clamp(0.0, 1.0)`.

### Requirement: Heat tinting on part vertices

When `heat_fraction > 0.01`, part vertex colors SHALL be tinted toward orange then white:
- R: lerp toward 1.0
- G: lerp toward 0.6 (orange phase, h < 0.5), then toward 1.0 (white phase, h >= 0.5)
- B: reduce toward 0 (h < 0.5), then increase toward 1.0 (h >= 0.5)
- Alpha unchanged

### Requirement: Heat tinting on ship triangle icon

The same heat tinting SHALL be applied to the ship's triangle indicator (both fallback triangle and map-view indicator) using the `apply_heat_tint()` function.

### Requirement: Heat bar in left HUD panel

When `temperature > 350K`, a vertical heat bar SHALL be shown in the left panel below the fuel bar:
- "HEAT" label at font size 10, temperature readout in Kelvin at font size 11
- Bar height 80px, width 20px, fill from bottom proportional to `heat_fraction`
- Colors: `< 0.33` -> yellow `rgb(220, 200, 80)`, `< 0.66` -> orange `rgb(220, 140, 40)`, `>= 0.66` -> red `rgb(220, 60, 60)`
- Background `rgb(40, 40, 50)`, border gray 1px

### Requirement: Temperature readout in bottom panel

When `heat_fraction > 0.01`, a compact temperature readout ("{temp}K") SHALL appear in the bottom panel after the altitude display:
- Font size 13 strong
- Color matches heat bar thresholds: yellow / orange / red
