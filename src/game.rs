use crate::bodies::SolarSystem;
use crate::editor::EditorState;
use crate::parts::{BlueprintRegistry, FlightVessel, PartDefinitions};
use crate::render::ManeuverNode;
use crate::ship::{Ship, ShipInput};

/// Launchpad constants
pub const LAUNCHPAD_BODY_INDEX: usize = 3; // Earth
pub const LAUNCHPAD_SURFACE_ANGLE: f64 = std::f64::consts::FRAC_PI_2;
pub const LAUNCHPAD_HEIGHT: f64 = 10.0; // meters
pub const LAUNCHPAD_TOP_WIDTH: f64 = 100.0; // meters
pub const LAUNCHPAD_BOTTOM_WIDTH: f64 = 120.0; // meters

/// The current game mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    MainMenu,
    Editor,
    Flight,
    TrackingStation,
}

/// Unique identifier for a vessel (active or inactive)
pub type VesselId = u64;

/// An inactive vessel tracked by the game (on-rails propagation)
#[derive(Clone)]
pub struct TrackedVessel {
    pub id: VesselId,
    pub name: String,
    pub ship: Ship,
    pub vessel: Option<FlightVessel>,
    pub maneuver_nodes: Vec<ManeuverNode>,
}

/// Flight-specific state
pub struct FlightState {
    // Active vessel (direct access — existing code unchanged)
    pub ship: Ship,
    pub ship_input: ShipInput,
    pub tracking_ship: bool,
    pub vessel: Option<FlightVessel>,

    // Active vessel identity
    pub active_vessel_id: VesselId,
    pub active_vessel_name: String,

    // Inactive vessels (on-rails only)
    pub inactive_vessels: Vec<TrackedVessel>,
    pub next_vessel_id: VesselId,
    pub debris_counter: u32,
}

impl FlightState {
    pub fn new(ship: Ship) -> Self {
        Self {
            ship,
            ship_input: ShipInput::default(),
            tracking_ship: true,
            vessel: None,
            active_vessel_id: 0,
            active_vessel_name: "Ship".to_string(),
            inactive_vessels: Vec::new(),
            next_vessel_id: 1,
            debris_counter: 0,
        }
    }

    /// Total number of vessels (active + inactive)
    pub fn vessel_count(&self) -> usize {
        1 + self.inactive_vessels.len()
    }

    /// Get all vessel IDs sorted (active first, then inactive by ID)
    pub fn all_vessel_ids(&self) -> Vec<VesselId> {
        let mut ids = vec![self.active_vessel_id];
        for v in &self.inactive_vessels {
            ids.push(v.id);
        }
        ids.sort();
        ids
    }

    /// Switch to a different vessel by ID.
    /// `current_maneuver_nodes` are saved with the current active vessel.
    /// Returns the new active vessel's maneuver nodes (to load into render_state).
    pub fn switch_to_vessel(
        &mut self,
        target_id: VesselId,
        current_maneuver_nodes: Vec<ManeuverNode>,
        solar_system: &SolarSystem,
    ) -> Result<Vec<ManeuverNode>, String> {
        // Find target in inactive vessels
        let target_pos = self.inactive_vessels.iter()
            .position(|v| v.id == target_id)
            .ok_or_else(|| format!("Vessel {} not found", target_id))?;

        let target = self.inactive_vessels.remove(target_pos);

        // Save current active vessel as inactive
        let mut saved_ship = self.ship.clone();
        saved_ship.enter_rails_mode(solar_system);

        self.inactive_vessels.push(TrackedVessel {
            id: self.active_vessel_id,
            name: self.active_vessel_name.clone(),
            ship: saved_ship,
            vessel: self.vessel.take(),
            maneuver_nodes: current_maneuver_nodes,
        });

        // Load target as active
        self.ship = target.ship;
        self.ship.exit_rails_mode(solar_system);
        self.vessel = target.vessel;
        self.active_vessel_id = target.id;
        self.active_vessel_name = target.name;
        self.ship_input = ShipInput::default();
        self.tracking_ship = true;

        Ok(target.maneuver_nodes)
    }

    /// Save the active vessel into inactive_vessels (for leaving flight mode).
    /// The active vessel is put on rails first.
    pub fn shelve_active_vessel(
        &mut self,
        current_maneuver_nodes: Vec<ManeuverNode>,
        solar_system: &SolarSystem,
    ) {
        let mut saved_ship = self.ship.clone();
        saved_ship.enter_rails_mode(solar_system);

        self.inactive_vessels.push(TrackedVessel {
            id: self.active_vessel_id,
            name: self.active_vessel_name.clone(),
            ship: saved_ship,
            vessel: self.vessel.clone(),
            maneuver_nodes: current_maneuver_nodes,
        });
    }

    /// Load a vessel from inactive_vessels as the active vessel (for entering flight mode).
    /// Returns the vessel's maneuver nodes (to load into render_state).
    pub fn activate_vessel(
        &mut self,
        target_id: VesselId,
        solar_system: &SolarSystem,
    ) -> Result<Vec<ManeuverNode>, String> {
        let target_pos = self.inactive_vessels.iter()
            .position(|v| v.id == target_id)
            .ok_or_else(|| format!("Vessel {} not found", target_id))?;

        let target = self.inactive_vessels.remove(target_pos);

        self.ship = target.ship;
        self.ship.exit_rails_mode(solar_system);
        self.vessel = target.vessel;
        self.active_vessel_id = target.id;
        self.active_vessel_name = target.name;
        self.ship_input = ShipInput::default();
        self.tracking_ship = true;

        Ok(target.maneuver_nodes)
    }

    /// Create a debris vessel from extracted decoupled parts.
    /// `com_offset` is in vessel-local coordinates (before rotation).
    /// `ejection_force_kn` applies a separation impulse pushing debris away from the upper stage.
    pub fn create_debris_vessel(
        &mut self,
        debris_vessel: FlightVessel,
        com_offset: [f64; 2],
        ejection_force_kn: f64,
        solar_system: &SolarSystem,
    ) {
        // Local-to-world rotation for part positions (heading - PI/2, matches rendering)
        let local_rot = self.ship.rotation - std::f64::consts::FRAC_PI_2;
        let world_offset_x = com_offset[0] * local_rot.cos() - com_offset[1] * local_rot.sin();
        let world_offset_y = com_offset[0] * local_rot.sin() + com_offset[1] * local_rot.cos();

        let mut debris_ship = self.ship.clone();
        debris_ship.rel_position[0] += world_offset_x;
        debris_ship.rel_position[1] += world_offset_y;
        debris_ship.throttle = 0.0;
        debris_ship.on_rails = false;
        debris_ship.cached_orbit = None;
        debris_ship.cached_trajectory = None;
        debris_ship.color = [1.0, 1.0, 1.0, 1.0]; // Same as active vessels

        // Apply separation impulse: F*dt/m, using a 0.1s impulse duration
        // Direction: opposite to vessel heading (push debris downward)
        if ejection_force_kn > 0.0 && debris_vessel.total_mass > 0.0 {
            let impulse_duration = 0.1; // seconds
            let force_newtons = ejection_force_kn * 1000.0;
            let mass_kg = debris_vessel.total_mass * 1000.0;
            let dv = force_newtons * impulse_duration / mass_kg;
            // Heading is [cos(rotation), sin(rotation)]; push debris opposite
            let sep_dir = self.ship.rotation + std::f64::consts::PI;
            debris_ship.rel_velocity[0] += dv * sep_dir.cos();
            debris_ship.rel_velocity[1] += dv * sep_dir.sin();
        }

        // Put debris on rails immediately
        debris_ship.enter_rails_mode(solar_system);

        self.debris_counter += 1;
        let name = format!("Debris {}", self.debris_counter);
        let id = self.next_vessel_id;
        self.next_vessel_id += 1;

        self.inactive_vessels.push(TrackedVessel {
            id,
            name: name.clone(),
            ship: debris_ship,
            vessel: Some(debris_vessel),
            maneuver_nodes: Vec::new(),
        });

        log::info!("Created debris vessel: {} (id={})", name, id);
    }

    /// Remove inactive vessels that are landed on the launchpad.
    /// Called before launching a new vessel to clear the pad.
    pub fn recover_vessels_on_launchpad(&mut self, solar_system: &crate::bodies::SolarSystem) {
        let earth_radius = solar_system.bodies[LAUNCHPAD_BODY_INDEX].radius;
        let half_angle = (LAUNCHPAD_BOTTOM_WIDTH * 0.5) / earth_radius;

        self.inactive_vessels.retain(|v| {
            let dominated_by_earth = v.ship.soi_body == LAUNCHPAD_BODY_INDEX;
            let is_landed = matches!(v.ship.state, crate::ship::ShipState::Landed { body_index, .. } if body_index == LAUNCHPAD_BODY_INDEX);
            if dominated_by_earth && is_landed {
                let angle = v.ship.rel_position[1].atan2(v.ship.rel_position[0]);
                let angle_diff = angle - LAUNCHPAD_SURFACE_ANGLE;
                let angle_diff = angle_diff - (angle_diff / std::f64::consts::TAU).round() * std::f64::consts::TAU;
                if angle_diff.abs() < half_angle {
                    log::info!("Auto-recovered vessel on launchpad: {} (id={})", v.name, v.id);
                    return false; // Remove
                }
            }
            true // Keep
        });
    }
}

/// Returns true if the given body supports vessel recovery (i.e. has infrastructure).
/// Currently only Earth. Future colonies will add more indices.
pub fn is_recoverable_body(body_index: usize) -> bool {
    body_index == LAUNCHPAD_BODY_INDEX
}

/// Central game state container
pub struct Game {
    pub mode: GameMode,
    pub paused: bool,
    pub warp_index: usize,
    /// Elapsed simulation seconds since game epoch (Jan 1, 2030 00:00 UTC)
    pub simulation_time: f64,
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
            mode: GameMode::MainMenu,
            paused: false,
            warp_index: 0,
            simulation_time: 0.0,
            solar_system,
            flight,
            editor,
            part_definitions,
            blueprints,
        }
    }

    /// Switch to main menu
    pub fn enter_main_menu(&mut self) {
        self.mode = GameMode::MainMenu;
        self.paused = false;
        log::info!("Entered main menu");
    }

    /// Switch to editor mode
    pub fn enter_editor(&mut self) {
        self.mode = GameMode::Editor;
        self.paused = false;
        log::info!("Entered editor mode");
    }

    /// Switch to flight mode (without launching - resume existing flight)
    pub fn enter_flight(&mut self) {
        self.mode = GameMode::Flight;
        self.paused = false;
        log::info!("Entered flight mode");
    }

    /// Switch to tracking station
    pub fn enter_tracking_station(&mut self) {
        self.mode = GameMode::TrackingStation;
        self.paused = false;
        log::info!("Entered tracking station");
    }

    /// Get current time warp multiplier
    pub fn time_warp(&self, warp_levels: &[f64]) -> f64 {
        warp_levels.get(self.warp_index).copied().unwrap_or(1.0)
    }

    /// Launch a vessel from the editor
    pub fn launch_from_editor(&mut self) -> Result<(), String> {
        // Build blueprint from editor state
        let blueprint = self.editor.to_blueprint(&self.part_definitions)?;

        // Get spawn position (on launchpad)
        let earth_idx = LAUNCHPAD_BODY_INDEX;
        let earth = &self.solar_system.bodies[earth_idx];

        // Surface angle (spawn at launchpad location)
        let surface_angle = LAUNCHPAD_SURFACE_ANGLE;

        // Create flight vessel first to get bounding height
        let vessel = FlightVessel::from_blueprint(
            &blueprint,
            &self.part_definitions,
            [0.0, 0.0], // Temporary, will set below
            [0.0, 0.0], // Stationary relative to Earth surface
            earth_idx,
        )?;

        let surface_distance = earth.radius + LAUNCHPAD_HEIGHT + vessel.bottom_extent();

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
        self.flight.ship.temperature = crate::ship::AMBIENT_TEMPERATURE;
        self.flight.ship.heat_flux = 0.0;
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
        self.warp_index = 0;
        self.flight.tracking_ship = true;
        self.flight.ship_input = ShipInput::default();

        // Assign vessel identity
        self.flight.active_vessel_id = self.flight.next_vessel_id;
        self.flight.active_vessel_name = blueprint.name.clone();
        self.flight.next_vessel_id += 1;

        if let Some(ref v) = self.flight.vessel {
            log::info!("Launched vessel: {} ({} parts, {} stages, current_stage={})",
                blueprint.name, v.parts.len(), v.stages.len(), v.current_stage);
            for (i, stage) in v.stages.iter().enumerate() {
                let part_names: Vec<String> = stage.iter().map(|&idx| {
                    if idx < v.parts.len() {
                        format!("{}({})", idx, &v.parts[idx].definition_id)
                    } else {
                        format!("{}(INVALID)", idx)
                    }
                }).collect();
                log::info!("  Stage {}: {:?}", i, part_names);
            }
            let decoupled_count = v.parts.iter().filter(|p| p.decoupled).count();
            let destroyed_count = v.parts.iter().filter(|p| p.destroyed).count();
            if decoupled_count > 0 || destroyed_count > 0 {
                log::warn!("  WARNING: {} decoupled, {} destroyed parts at launch!", decoupled_count, destroyed_count);
            }
        }
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
        if self.paused {
            return;
        }

        let time_warp = self.time_warp(warp_levels);

        // Solar system advances in all modes
        self.solar_system.update(dt * time_warp);

        // Ship physics only in flight mode
        if self.mode == GameMode::Flight {
            self.flight.ship.update(
                dt * time_warp,
                time_warp,
                &self.flight.ship_input,
                &self.solar_system,
                None,
                false,
                self.flight.vessel.is_some(),
            );

            if let Some(ref mut vessel) = self.flight.vessel {
                vessel.rel_position = self.flight.ship.rel_position;
                vessel.rel_velocity = self.flight.ship.rel_velocity;
                vessel.rotation = self.flight.ship.rotation;
                vessel.throttle = self.flight.ship.throttle;
            }
        }
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        if self.paused {
            self.warp_index = 0;
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

/// Format simulation_time (seconds since Jan 1, 2030 00:00 UTC) as a date string.
pub fn format_date(simulation_time: f64) -> String {
    // Jan 1, 2030 = day 0
    const EPOCH_YEAR: i32 = 2030;

    let total_seconds = simulation_time as i64;
    let days_total = (total_seconds / 86400) as i32;
    let remaining_secs = total_seconds % 86400;
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;

    fn is_leap(y: i32) -> bool {
        (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
    }

    fn days_in_year(y: i32) -> i32 {
        if is_leap(y) { 366 } else { 365 }
    }

    fn days_in_month(y: i32, m: i32) -> i32 {
        match m {
            1 => 31, 2 => if is_leap(y) { 29 } else { 28 },
            3 => 31, 4 => 30, 5 => 31, 6 => 30,
            7 => 31, 8 => 31, 9 => 30, 10 => 31, 11 => 30, 12 => 31,
            _ => 30,
        }
    }

    let mut year = EPOCH_YEAR;
    let mut remaining_days = days_total;

    if remaining_days >= 0 {
        while remaining_days >= days_in_year(year) {
            remaining_days -= days_in_year(year);
            year += 1;
        }
    } else {
        while remaining_days < 0 {
            year -= 1;
            remaining_days += days_in_year(year);
        }
    }

    let mut month = 1;
    while month < 12 && remaining_days >= days_in_month(year, month) {
        remaining_days -= days_in_month(year, month);
        month += 1;
    }
    let day = remaining_days + 1; // 1-indexed

    const MONTH_NAMES: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    format!(
        "{} {}, {} {:02}:{:02}",
        MONTH_NAMES[(month - 1) as usize], day, year, hours, minutes
    )
}
