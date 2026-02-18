use crate::bodies::SolarSystem;
use crate::editor::EditorState;
use crate::parts::{BlueprintRegistry, FlightVessel, PartDefinitions};
use crate::ship::{Ship, ShipInput};

/// The current game mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Flight,
    Editor,
}

/// Flight-specific state
pub struct FlightState {
    pub ship: Ship,
    pub ship_input: ShipInput,
    pub warp_index: usize,
    pub tracking_ship: bool,
    pub vessel: Option<FlightVessel>,
}

impl FlightState {
    pub fn new(ship: Ship) -> Self {
        Self {
            ship,
            ship_input: ShipInput::default(),
            warp_index: 0,
            tracking_ship: true,
            vessel: None,
        }
    }

    /// Get current time warp multiplier
    pub fn time_warp(&self, warp_levels: &[f64]) -> f64 {
        warp_levels.get(self.warp_index).copied().unwrap_or(1.0)
    }
}

/// Central game state container
pub struct Game {
    pub mode: GameMode,
    pub solar_system: SolarSystem,
    pub flight: FlightState,
    pub editor: EditorState,
    pub part_definitions: PartDefinitions,
    pub blueprints: BlueprintRegistry,
}

impl Game {
    pub fn new() -> Self {
        // Load part definitions
        let part_definitions = PartDefinitions::load_from_directory("data/parts")
            .unwrap_or_else(|e| {
                log::error!("Failed to load parts: {}", e);
                PartDefinitions::new()
            });

        // Load blueprints
        let mut blueprints = BlueprintRegistry::new("data/blueprints");
        if let Err(e) = blueprints.load_all() {
            log::error!("Failed to load blueprints: {}", e);
        }

        // Initialize solar system
        let solar_system = SolarSystem::new();

        // Create initial ship
        let ship = Ship::spawn_on_earth(&solar_system);
        let flight = FlightState::new(ship);

        // Create editor state
        let editor = EditorState::new();

        Self {
            mode: GameMode::Editor,
            solar_system,
            flight,
            editor,
            part_definitions,
            blueprints,
        }
    }

    /// Switch to editor mode
    pub fn enter_editor(&mut self) {
        self.mode = GameMode::Editor;
        log::info!("Entered editor mode");
    }

    /// Switch to flight mode (without launching - resume existing flight)
    pub fn enter_flight(&mut self) {
        self.mode = GameMode::Flight;
        log::info!("Entered flight mode");
    }

    /// Launch a vessel from the editor
    pub fn launch_from_editor(&mut self) -> Result<(), String> {
        // Build blueprint from editor state
        let blueprint = self.editor.to_blueprint(&self.part_definitions)?;

        // Get spawn position (on Earth's surface for now)
        let earth_idx = 3; // Earth is index 3 in the solar system
        let earth = &self.solar_system.bodies[earth_idx];

        // Surface angle (spawn at top of the body, angle = π/2)
        let surface_angle = std::f64::consts::FRAC_PI_2;

        // Create flight vessel first to get bounding height
        let vessel = FlightVessel::from_blueprint(
            &blueprint,
            &self.part_definitions,
            [0.0, 0.0], // Temporary, will set below
            [0.0, 0.0], // Stationary relative to Earth surface
            earth_idx,
        )?;

        let surface_distance = earth.radius + vessel.bottom_extent();

        // Position on the surface
        let spawn_position = [
            surface_distance * surface_angle.cos(),
            surface_distance * surface_angle.sin(),
        ];

        // Recreate vessel at correct position
        let mut vessel = FlightVessel::from_blueprint(
            &blueprint,
            &self.part_definitions,
            spawn_position,
            [0.0, 0.0],
            earth_idx,
        )?;

        // Build weld connections
        let _weld_connections = vessel.find_weld_connections(&self.part_definitions);

        // Update ship state to match vessel
        self.flight.ship.rel_position = spawn_position;
        self.flight.ship.rel_velocity = [0.0, 0.0];
        self.flight.ship.rotation = surface_angle; // Point up from surface
        self.flight.ship.rotational_velocity = 0.0;
        self.flight.ship.soi_body = earth_idx;
        self.flight.ship.throttle = 0.0;
        self.flight.ship.on_rails = false;
        self.flight.ship.cached_orbit = None;
        self.flight.ship.state = crate::ship::ShipState::Landed {
            body_index: earth_idx,
            surface_angle,
        };

        // Sync vessel state
        vessel.rel_position = spawn_position;
        vessel.rotation = surface_angle;
        self.flight.vessel = Some(vessel);

        // Switch to flight mode
        self.mode = GameMode::Flight;
        self.flight.warp_index = 0;
        self.flight.tracking_ship = true;
        self.flight.ship_input = ShipInput::default();

        log::info!("Launched vessel: {}", blueprint.name);
        Ok(())
    }

    /// Load a blueprint into the editor
    pub fn load_blueprint(&mut self, name: &str) -> Result<(), String> {
        let blueprint = self.blueprints.get(name)
            .ok_or_else(|| format!("Blueprint not found: {}", name))?
            .clone();

        self.editor.load_blueprint(&blueprint, &self.part_definitions);
        self.mode = GameMode::Editor;
        Ok(())
    }

    /// Save the current editor state as a blueprint
    pub fn save_blueprint(&mut self, name: String) -> Result<(), String> {
        let mut blueprint = self.editor.to_blueprint(&self.part_definitions)?;
        blueprint.name = name;
        self.blueprints.save(blueprint)
    }

    /// Create a new vessel in the editor
    pub fn new_vessel(&mut self) {
        self.editor.clear();
        self.mode = GameMode::Editor;
    }

    /// Update the game simulation
    pub fn update(&mut self, dt: f64, warp_levels: &[f64]) {
        match self.mode {
            GameMode::Flight => {
                let time_warp = self.flight.time_warp(warp_levels);

                // Update solar system
                self.solar_system.update(dt * time_warp);

                // Update ship physics
                self.flight.ship.update(
                    dt * time_warp,
                    time_warp,
                    &self.flight.ship_input,
                    &self.solar_system,
                    None,
                );

                // Update flight vessel if present (fuel consumption, etc.)
                if let Some(ref mut vessel) = self.flight.vessel {
                    // Sync vessel state with ship
                    vessel.rel_position = self.flight.ship.rel_position;
                    vessel.rel_velocity = self.flight.ship.rel_velocity;
                    vessel.rotation = self.flight.ship.rotation;
                    vessel.throttle = self.flight.ship.throttle;
                }
            }
            GameMode::Editor => {
                // No physics updates in editor mode
                // Editor updates camera, UI state, etc.
            }
        }
    }

    /// Check if we're in editor mode
    pub fn is_editing(&self) -> bool {
        self.mode == GameMode::Editor
    }

    /// Check if we're in flight mode
    pub fn is_flying(&self) -> bool {
        self.mode == GameMode::Flight
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}
