use crate::parts::{
    FuelType, PartCategory, PartDefinitions,
    PlacedPart, PlacedPartId, SymmetryMode, VesselBlueprint,
    blueprint_to_parts, parts_to_blueprint,
};
use std::collections::HashMap;

/// Grid snap size in meters
const GRID_SIZE: f64 = 0.5;

/// Ship statistics calculated from placed parts
#[derive(Debug, Clone, Default)]
pub struct ShipStats {
    pub dry_mass: f64,           // Mass without resources (tonnes)
    pub wet_mass: f64,           // Mass with resources (tonnes)
    pub thrust_vac: f64,         // Total vacuum thrust (kN)
    pub thrust_asl: f64,         // Total sea-level thrust (kN)
    pub resources: HashMap<String, ResourceAmount>,  // Resource name -> amounts
}

/// Resource amount tracking
#[derive(Debug, Clone, Default)]
pub struct ResourceAmount {
    pub current: f64,
    pub max: f64,
}

impl ShipStats {
    /// Calculate TWR for a given surface gravity
    pub fn twr_vac(&self, surface_gravity: f64) -> f64 {
        if self.wet_mass <= 0.0 || surface_gravity <= 0.0 {
            return 0.0;
        }
        // thrust in kN, mass in tonnes, gravity in m/s²
        // TWR = thrust / (mass * g) = kN / (t * m/s²) = kN / kN = dimensionless
        self.thrust_vac / (self.wet_mass * surface_gravity)
    }

    /// Calculate TWR at sea level for a given surface gravity
    pub fn twr_asl(&self, surface_gravity: f64) -> f64 {
        if self.wet_mass <= 0.0 || surface_gravity <= 0.0 {
            return 0.0;
        }
        self.thrust_asl / (self.wet_mass * surface_gravity)
    }
}

/// TWR display settings
#[derive(Debug, Clone)]
pub struct TwrSettings {
    pub body_index: usize,       // Which body to calculate TWR for
    pub show_asl: bool,          // Show ASL TWR (vs vacuum)
}

impl Default for TwrSettings {
    fn default() -> Self {
        Self {
            body_index: 3,  // Earth by default
            show_asl: true, // Show ASL by default
        }
    }
}

/// Editor state
#[derive(Debug)]
pub struct EditorState {
    // Placed parts
    pub parts: HashMap<PlacedPartId, PlacedPart>,
    pub root_part: Option<PlacedPartId>,
    next_part_id: PlacedPartId,

    // Selection and placement
    pub selected_part_def: Option<String>,
    pub selected_placed_part: Option<PlacedPartId>,
    pub ghost_position: Option<[f64; 2]>,
    pub ghost_valid: bool,

    // Camera (zoom is pixels per meter)
    pub camera_offset: [f64; 2],
    pub camera_zoom: f32,  // Higher = more zoomed in

    // Camera movement keys held
    pub keys_held: CameraKeys,

    // Tools
    pub symmetry_mode: SymmetryMode,
    pub selected_category: PartCategory,

    // Staging
    pub stages: Vec<Vec<PlacedPartId>>,
    pub staging_selected_engine: Option<PlacedPartId>,

    // UI state
    pub vessel_name: String,
    pub show_save_dialog: bool,
    pub show_load_dialog: bool,
    pub hovered_part: Option<PlacedPartId>,
    pub part_to_delete: Option<PlacedPartId>,

    // Dragging state
    pub dragging_part: Option<PlacedPartId>,
    pub drag_start_pos: Option<[f64; 2]>,  // Original position before drag
    pub drag_valid: bool,                   // Whether current drag position is valid

    // Stats display settings
    pub twr_settings: TwrSettings,
}

/// Tracks which camera movement keys are held
#[derive(Debug, Default)]
pub struct CameraKeys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            parts: HashMap::new(),
            root_part: None,
            next_part_id: 1,
            selected_part_def: None,
            selected_placed_part: None,
            ghost_position: None,
            ghost_valid: false,
            camera_offset: [GRID_SIZE / 2.0, GRID_SIZE / 2.0],  // Center on middle of a square
            camera_zoom: 1.0,  // Start zoomed out to see workspace
            keys_held: CameraKeys::default(),
            symmetry_mode: SymmetryMode::Off,
            selected_category: PartCategory::Pods,
            stages: Vec::new(),
            staging_selected_engine: None,
            vessel_name: "Untitled Vessel".to_string(),
            show_save_dialog: false,
            show_load_dialog: false,
            hovered_part: None,
            part_to_delete: None,
            dragging_part: None,
            drag_start_pos: None,
            drag_valid: true,
            twr_settings: TwrSettings::default(),
        }
    }

    /// Update camera position based on held keys (call each frame)
    pub fn update_camera(&mut self, dt: f32) {
        let speed = 1.67 / self.camera_zoom as f64;  // World units per second (3x slower)
        let delta = speed * dt as f64;

        if self.keys_held.up {
            self.camera_offset[1] += delta;
        }
        if self.keys_held.down {
            self.camera_offset[1] -= delta;
        }
        if self.keys_held.left {
            self.camera_offset[0] -= delta;
        }
        if self.keys_held.right {
            self.camera_offset[0] += delta;
        }
    }

    /// Clear the editor to start a new vessel
    pub fn clear(&mut self) {
        self.parts.clear();
        self.root_part = None;
        self.next_part_id = 1;
        self.selected_part_def = None;
        self.selected_placed_part = None;
        self.ghost_position = None;
        self.ghost_valid = false;
        self.stages.clear();
        self.staging_selected_engine = None;
        self.vessel_name = "Untitled Vessel".to_string();
    }

    /// Load a blueprint into the editor
    pub fn load_blueprint(&mut self, blueprint: &VesselBlueprint, part_defs: &PartDefinitions) {
        self.clear();
        let (parts, root_id, stages) = blueprint_to_parts(blueprint);
        self.parts = parts;
        self.root_part = Some(root_id);
        self.stages = stages;
        self.vessel_name = blueprint.name.clone();

        // Update next_part_id to be higher than any existing ID
        if let Some(max_id) = self.parts.keys().max() {
            self.next_part_id = max_id + 1;
        }

        // Auto-populate stage 1 with all engines if stages are empty
        if self.stages.is_empty() {
            let engine_ids: Vec<PlacedPartId> = self.parts.iter()
                .filter(|(_, part)| part_defs.get(&part.definition_id).map(|d| d.engine.is_some()).unwrap_or(false))
                .map(|(&id, _)| id)
                .collect();
            if !engine_ids.is_empty() {
                self.stages.push(engine_ids);
            }
        }

        // Center and zoom camera to fit the loaded craft
        self.focus_on_parts(part_defs);
    }

    /// Convert editor state to a blueprint
    pub fn to_blueprint(&self, _part_defs: &PartDefinitions) -> Result<VesselBlueprint, String> {
        let root_id = self.root_part.ok_or("No root part (command pod) placed")?;

        if self.parts.is_empty() {
            return Err("No parts placed".to_string());
        }

        Ok(parts_to_blueprint(
            &self.parts,
            root_id,
            self.vessel_name.clone(),
            &self.stages,
        ))
    }

    /// Update ghost position based on mouse world coordinates
    pub fn update_ghost(&mut self, world_x: f64, world_y: f64, part_defs: &PartDefinitions) {
        let Some(ref def_id) = self.selected_part_def else {
            self.ghost_position = None;
            self.ghost_valid = false;
            return;
        };

        let Some(def) = part_defs.get(def_id) else {
            self.ghost_position = None;
            self.ghost_valid = false;
            return;
        };

        // Snap based on HITBOX dimensions (for proper alignment with other parts)
        // Odd dimensions snap to square center, even dimensions snap to grid line
        let snapped_x = if def.hitbox_grid_width() % 2 == 1 {
            // Odd width: center on middle of square
            (world_x / GRID_SIZE).floor() * GRID_SIZE + GRID_SIZE / 2.0
        } else {
            // Even width: center on grid line
            (world_x / GRID_SIZE + 0.5).floor() * GRID_SIZE
        };

        let snapped_y = if def.hitbox_grid_height() % 2 == 1 {
            // Odd height: center on middle of square
            (world_y / GRID_SIZE).floor() * GRID_SIZE + GRID_SIZE / 2.0
        } else {
            // Even height: center on grid line
            (world_y / GRID_SIZE + 0.5).floor() * GRID_SIZE
        };

        self.ghost_position = Some([snapped_x, snapped_y]);

        // Check if placement would overlap any existing part using HITBOX dimensions
        let new_bounds = Self::calc_bounds([snapped_x, snapped_y], def.hitbox_width(), def.hitbox_height());

        let mut overlaps = false;
        for (_, part) in &self.parts {
            if let Some(existing_def) = part_defs.get(&part.definition_id) {
                let existing_bounds = Self::calc_bounds(part.position, existing_def.hitbox_width(), existing_def.hitbox_height());
                if Self::bounds_overlap(&new_bounds, &existing_bounds) {
                    overlaps = true;
                    break;
                }
            }
        }

        // Valid if no overlap with existing parts
        self.ghost_valid = !overlaps;
    }

    /// Calculate exact bounds for a part (no padding)
    fn calc_bounds(pos: [f64; 2], width: f64, height: f64) -> [f64; 4] {
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        [pos[0] - half_w, pos[1] - half_h, pos[0] + half_w, pos[1] + half_h]
    }

    /// Check if two bounds overlap (AABB collision)
    /// Uses < not <= so touching edges don't count as overlap
    fn bounds_overlap(a: &[f64; 4], b: &[f64; 4]) -> bool {
        // a and b are [min_x, min_y, max_x, max_y]
        a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
    }

    /// Place a part at the current ghost position
    pub fn place_part(&mut self, part_defs: &PartDefinitions) -> bool {
        let Some(ref def_id) = self.selected_part_def else {
            return false;
        };

        let Some(position) = self.ghost_position else {
            return false;
        };

        if !self.ghost_valid {
            return false;
        }

        let id = self.next_part_id;
        self.next_part_id += 1;

        let def = part_defs.get(def_id);
        let is_engine = def.map(|d| d.engine.is_some()).unwrap_or(false);
        let is_decoupler = def.map(|d| d.decoupler.is_some()).unwrap_or(false);

        let part = PlacedPart::new(id, def_id.clone(), position);

        // First part becomes root
        if self.root_part.is_none() {
            self.root_part = Some(id);
        }

        self.parts.insert(id, part);

        // Auto-add engines to stage 1
        if is_engine {
            if self.stages.is_empty() {
                self.stages.push(Vec::new());
            }
            self.stages[0].push(id);
        }

        // Auto-add decouplers to stage 1
        if is_decoupler {
            if self.stages.is_empty() {
                self.stages.push(Vec::new());
            }
            self.stages[0].push(id);
        }

        true
    }

    /// Delete a part
    pub fn delete_part(&mut self, part_id: PlacedPartId) {
        self.parts.remove(&part_id);

        // Update root if deleted
        if self.root_part == Some(part_id) {
            // Set new root to any remaining part
            self.root_part = self.parts.keys().next().copied();
        }

        // Remove from stages and clean up empty stages
        for stage in &mut self.stages {
            stage.retain(|&sid| sid != part_id);
        }
        self.stages.retain(|s| !s.is_empty());

        // Clear staging selection if it was this part
        if self.staging_selected_engine == Some(part_id) {
            self.staging_selected_engine = None;
        }

        // Clear selection if deleted part was selected
        if self.selected_placed_part == Some(part_id) {
            self.selected_placed_part = None;
        }
    }

    /// Move an engine to a different stage
    pub fn move_engine_to_stage(&mut self, engine_id: PlacedPartId, target_stage_idx: usize) {
        for stage in &mut self.stages {
            stage.retain(|&id| id != engine_id);
        }
        if target_stage_idx < self.stages.len() {
            self.stages[target_stage_idx].push(engine_id);
        }
        self.staging_selected_engine = None;
    }

    /// Process any pending deletions (call after UI)
    pub fn process_pending_delete(&mut self) {
        if let Some(part_id) = self.part_to_delete.take() {
            self.delete_part(part_id);
        }
    }

    /// Select a part in the palette
    pub fn select_part_def(&mut self, def_id: &str) {
        self.selected_part_def = Some(def_id.to_string());
        self.selected_placed_part = None;
    }

    /// Deselect the current part
    pub fn deselect(&mut self) {
        self.selected_part_def = None;
        self.selected_placed_part = None;
        self.ghost_position = None;
        self.ghost_valid = false;
    }

    /// Select a placed part
    pub fn select_placed_part(&mut self, part_id: PlacedPartId) {
        self.selected_placed_part = Some(part_id);
        self.selected_part_def = None;
    }

    /// Start dragging a placed part
    pub fn start_drag(&mut self, part_id: PlacedPartId) {
        if let Some(part) = self.parts.get(&part_id) {
            self.dragging_part = Some(part_id);
            self.drag_start_pos = Some(part.position);
            self.drag_valid = true;
            self.selected_placed_part = Some(part_id);
            self.selected_part_def = None;
        }
    }

    /// Update the position of the part being dragged
    pub fn update_drag(&mut self, world_x: f64, world_y: f64, part_defs: &PartDefinitions) {
        let Some(part_id) = self.dragging_part else {
            return;
        };

        let Some(part) = self.parts.get(&part_id) else {
            self.dragging_part = None;
            return;
        };

        let Some(def) = part_defs.get(&part.definition_id) else {
            return;
        };

        // Snap based on HITBOX dimensions
        let snapped_x = if def.hitbox_grid_width() % 2 == 1 {
            (world_x / GRID_SIZE).floor() * GRID_SIZE + GRID_SIZE / 2.0
        } else {
            (world_x / GRID_SIZE + 0.5).floor() * GRID_SIZE
        };

        let snapped_y = if def.hitbox_grid_height() % 2 == 1 {
            (world_y / GRID_SIZE).floor() * GRID_SIZE + GRID_SIZE / 2.0
        } else {
            (world_y / GRID_SIZE + 0.5).floor() * GRID_SIZE
        };

        // Check if new position would overlap any other part
        let new_bounds = Self::calc_bounds([snapped_x, snapped_y], def.hitbox_width(), def.hitbox_height());

        let mut overlaps = false;
        for (&other_id, other_part) in &self.parts {
            if other_id == part_id {
                continue; // Skip the part being dragged
            }
            if let Some(other_def) = part_defs.get(&other_part.definition_id) {
                let other_bounds = Self::calc_bounds(other_part.position, other_def.hitbox_width(), other_def.hitbox_height());
                if Self::bounds_overlap(&new_bounds, &other_bounds) {
                    overlaps = true;
                    break;
                }
            }
        }

        self.drag_valid = !overlaps;

        // Update part position (we'll revert if invalid on release)
        if let Some(part) = self.parts.get_mut(&part_id) {
            part.position = [snapped_x, snapped_y];
        }
    }

    /// Finish dragging - revert if position is invalid
    pub fn finish_drag(&mut self, part_defs: &PartDefinitions) {
        let Some(part_id) = self.dragging_part.take() else {
            return;
        };

        let Some(start_pos) = self.drag_start_pos.take() else {
            return;
        };

        // Check final validity
        let Some(part) = self.parts.get(&part_id) else {
            return;
        };

        let Some(def) = part_defs.get(&part.definition_id) else {
            return;
        };

        let current_pos = part.position;
        let bounds = Self::calc_bounds(current_pos, def.hitbox_width(), def.hitbox_height());

        let mut overlaps = false;
        for (&other_id, other_part) in &self.parts {
            if other_id == part_id {
                continue;
            }
            if let Some(other_def) = part_defs.get(&other_part.definition_id) {
                let other_bounds = Self::calc_bounds(other_part.position, other_def.hitbox_width(), other_def.hitbox_height());
                if Self::bounds_overlap(&bounds, &other_bounds) {
                    overlaps = true;
                    break;
                }
            }
        }

        // Revert to original position if invalid
        if overlaps {
            if let Some(part) = self.parts.get_mut(&part_id) {
                part.position = start_pos;
            }
        }

        self.drag_valid = true;
    }

    /// Cancel dragging and revert to original position
    pub fn cancel_drag(&mut self) {
        if let (Some(part_id), Some(start_pos)) = (self.dragging_part.take(), self.drag_start_pos.take()) {
            if let Some(part) = self.parts.get_mut(&part_id) {
                part.position = start_pos;
            }
        }
        self.drag_valid = true;
    }

    /// Check if currently dragging a part
    pub fn is_dragging(&self) -> bool {
        self.dragging_part.is_some()
    }

    /// Pan the editor camera
    pub fn pan_camera(&mut self, dx: f64, dy: f64) {
        self.camera_offset[0] += dx / self.camera_zoom as f64;
        self.camera_offset[1] += dy / self.camera_zoom as f64;
    }

    /// Zoom the editor camera
    pub fn zoom_camera(&mut self, factor: f32) {
        self.camera_zoom *= factor;
        self.camera_zoom = self.camera_zoom.clamp(0.1, 16666.0);  // Zoom range
    }

    /// Center and zoom the camera to fit all placed parts
    pub fn focus_on_parts(&mut self, part_defs: &PartDefinitions) {
        if self.parts.is_empty() {
            return;
        }

        // Find bounding box of all parts
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for part in self.parts.values() {
            let half_w;
            let half_h;
            if let Some(def) = part_defs.get(&part.definition_id) {
                half_w = def.width() / 2.0;
                half_h = def.height() / 2.0;
            } else {
                half_w = 0.25;
                half_h = 0.25;
            }
            min_x = min_x.min(part.position[0] - half_w);
            max_x = max_x.max(part.position[0] + half_w);
            min_y = min_y.min(part.position[1] - half_h);
            max_y = max_y.max(part.position[1] + half_h);
        }

        // Center camera on the bounding box center
        self.camera_offset = [
            (min_x + max_x) / 2.0,
            (min_y + max_y) / 2.0,
        ];

        // Zoom to fit with padding (craft should take ~60% of the smaller screen dimension)
        let width = max_x - min_x;
        let height = max_y - min_y;
        let extent = width.max(height).max(1.0);
        // The editor camera maps: visible_half_extent ≈ 1/zoom (in meters)
        // We want extent to be ~60% of the view, so visible extent ≈ extent / 0.6
        self.camera_zoom = (0.6 / extent) as f32;
        self.camera_zoom = self.camera_zoom.clamp(0.1, 16666.0);
    }

    /// Check if the editor has any parts
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Check if the vessel is ready to launch
    pub fn can_launch(&self) -> bool {
        self.root_part.is_some() && !self.parts.is_empty()
    }

    /// Get total part count
    pub fn part_count(&self) -> usize {
        self.parts.len()
    }

    /// Calculate ship statistics from placed parts
    pub fn calculate_stats(&self, part_defs: &PartDefinitions) -> ShipStats {
        let mut stats = ShipStats::default();

        for part in self.parts.values() {
            let Some(def) = part_defs.get(&part.definition_id) else {
                continue;
            };

            // Add dry mass
            stats.dry_mass += def.mass;

            // Add engine thrust
            if let Some(ref engine) = def.engine {
                stats.thrust_vac += engine.thrust_vac;
                stats.thrust_asl += engine.thrust_asl;
            }

            // Add tank resources if filled
            if let Some(ref tank) = def.tank {
                if part.tank_filled && part.fuel_type != FuelType::Empty {
                    let (ox_kg, fuel_kg) = tank.propellant_capacity(part.fuel_type);

                    // Add oxygen
                    let ox_entry = stats.resources.entry("oxygen".to_string()).or_default();
                    ox_entry.current += ox_kg;
                    ox_entry.max += ox_kg;

                    // Add fuel
                    if let Some(fuel_name) = part.fuel_type.fuel_resource_name() {
                        let fuel_entry = stats.resources.entry(fuel_name.to_string()).or_default();
                        fuel_entry.current += fuel_kg;
                        fuel_entry.max += fuel_kg;
                    }
                }
            }
        }

        // Calculate wet mass (dry mass + resource mass in tonnes)
        // Resources are in kg, convert to tonnes
        let resource_mass: f64 = stats.resources.values().map(|r| r.current / 1000.0).sum();
        stats.wet_mass = stats.dry_mass + resource_mass;

        stats
    }

    /// Calculate per-stage delta-v (vacuum) using the Tsiolkovsky rocket equation.
    /// Simulates staging sequentially: decouplers fire, engines activate, all fuel burns.
    pub fn calculate_stage_delta_v(&self, part_defs: &PartDefinitions) -> Vec<f64> {
        let g0 = 9.80665;
        let mut stage_dvs = Vec::new();

        // Track state across stages
        let mut decoupled: std::collections::HashSet<PlacedPartId> = std::collections::HashSet::new();
        let mut engines_enabled: std::collections::HashSet<PlacedPartId> = std::collections::HashSet::new();

        // Track remaining fuel per part (in tonnes)
        let mut fuel_remaining: HashMap<PlacedPartId, f64> = HashMap::new();
        for (&part_id, part) in &self.parts {
            let Some(def) = part_defs.get(&part.definition_id) else { continue };
            if let Some(ref tank) = def.tank {
                if part.tank_filled && part.fuel_type != FuelType::Empty {
                    let (ox_kg, fuel_kg) = tank.propellant_capacity(part.fuel_type);
                    fuel_remaining.insert(part_id, (ox_kg + fuel_kg) / 1000.0);
                }
            }
        }

        for stage in &self.stages {
            // 1. Fire decouplers in this stage
            for &part_id in stage {
                if decoupled.contains(&part_id) { continue; }
                let Some(part) = self.parts.get(&part_id) else { continue };
                let Some(def) = part_defs.get(&part.definition_id) else { continue };
                if def.decoupler.is_some() {
                    let decoupler_bottom = part.position[1] - def.hitbox_height() / 2.0;
                    decoupled.insert(part_id);
                    for (&other_id, other_part) in &self.parts {
                        if decoupled.contains(&other_id) { continue; }
                        let Some(other_def) = part_defs.get(&other_part.definition_id) else { continue };
                        let other_top = other_part.position[1] + other_def.hitbox_height() / 2.0;
                        if other_top <= decoupler_bottom + 0.01 {
                            decoupled.insert(other_id);
                        }
                    }
                }
            }

            // 2. Enable engines in this stage
            for &part_id in stage {
                if decoupled.contains(&part_id) { continue; }
                let Some(_part) = self.parts.get(&part_id) else { continue };
                let Some(def) = part_defs.get(&_part.definition_id) else { continue };
                if def.engine.is_some() {
                    engines_enabled.insert(part_id);
                }
            }

            // 3. Calculate wet mass and fuel mass of remaining parts
            let mut wet_mass = 0.0;
            let mut fuel_mass = 0.0;
            for (&part_id, part) in &self.parts {
                if decoupled.contains(&part_id) { continue; }
                let Some(def) = part_defs.get(&part.definition_id) else { continue };
                wet_mass += def.mass;
                let part_fuel = fuel_remaining.get(&part_id).copied().unwrap_or(0.0);
                wet_mass += part_fuel;
                fuel_mass += part_fuel;
            }

            // 4. Calculate thrust-weighted average Isp of active engines
            let mut total_thrust = 0.0;
            let mut weighted_isp = 0.0;
            for &engine_id in &engines_enabled {
                if decoupled.contains(&engine_id) { continue; }
                let Some(part) = self.parts.get(&engine_id) else { continue };
                let Some(def) = part_defs.get(&part.definition_id) else { continue };
                if let Some(ref engine) = def.engine {
                    total_thrust += engine.thrust_vac;
                    weighted_isp += engine.thrust_vac * engine.isp_vac;
                }
            }
            let isp = if total_thrust > 0.0 { weighted_isp / total_thrust } else { 0.0 };

            // 5. Δv = Isp * g0 * ln(wet / dry)
            let dry_mass = wet_mass - fuel_mass;
            let dv = if isp > 0.0 && dry_mass > 0.0 && wet_mass > dry_mass {
                isp * g0 * (wet_mass / dry_mass).ln()
            } else {
                0.0
            };
            stage_dvs.push(dv);

            // 6. Consume all fuel in remaining parts
            for (&part_id, fuel) in fuel_remaining.iter_mut() {
                if !decoupled.contains(&part_id) {
                    *fuel = 0.0;
                }
            }
        }

        stage_dvs
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
