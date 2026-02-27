use crate::parts::{
    FairingShape, FuelType, PartCategory, PartDefinitions,
    PlacedPart, PlacedPartId, SymmetryMode, VesselBlueprint,
    blueprint_to_parts, parts_to_blueprint, GRID_SQUARE_SIZE,
};
use std::collections::{HashMap, HashSet, VecDeque};

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
    pub electricity_capacity: f64,  // Total battery Wh
    pub power_generation: f64,      // Total watts (solar + RTG)
    pub power_consumption: f64,     // Total watts (pods)
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
            body_index: 4,  // Earth by default
            show_asl: true, // Show ASL by default
        }
    }
}

/// State for the fairing shell building mode
#[derive(Debug, Clone)]
pub struct FairingBuildState {
    pub part_id: PlacedPartId,       // The fairing base being built
    pub base_top_y: f64,             // World Y of the base top edge
    pub base_center_x: f64,          // World X of the base center
    pub base_half_width: f64,        // Half-width of the fairing base (in grid squares)
    pub vertices: Vec<(f64, f64)>,   // Completed vertices (half_width_grid, y_offset_grid)
    pub ghost_point: Option<[f64; 2]>, // Current mouse-snapped point (world coords)
    pub ghost_valid: bool,           // Whether current ghost point is valid
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
    pub ghost_rotation: f64,  // Rotation for ghost preview (radians, 90° increments)
    pub mirror_ghost_position: Option<[f64; 2]>,
    pub mirror_ghost_def_id: Option<String>,

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
    pub drag_partner_start_pos: Option<[f64; 2]>,  // Mirror partner's original position
    pub drag_offset: [f64; 2],             // Offset from mouse to part center at drag start
    pub drag_valid: bool,                   // Whether current drag position is valid

    // Stats display settings
    pub twr_settings: TwrSettings,

    // Fairing build mode
    pub fairing_build_mode: Option<FairingBuildState>,
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
            ghost_rotation: 0.0,
            mirror_ghost_position: None,
            mirror_ghost_def_id: None,
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
            drag_partner_start_pos: None,
            drag_offset: [0.0, 0.0],
            drag_valid: true,
            twr_settings: TwrSettings::default(),
            fairing_build_mode: None,
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
        self.ghost_rotation = 0.0;
        self.mirror_ghost_position = None;
        self.mirror_ghost_def_id = None;
        self.stages.clear();
        self.staging_selected_engine = None;
        self.vessel_name = "Untitled Vessel".to_string();
        self.fairing_build_mode = None;
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

    /// Find all parts connected to the root via welding hitbox overlap (BFS)
    fn connected_part_ids(&self, part_defs: &PartDefinitions) -> HashSet<PlacedPartId> {
        let Some(root_id) = self.root_part else {
            return HashSet::new();
        };
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(root_id);
        visited.insert(root_id);
        while let Some(current_id) = queue.pop_front() {
            let current = &self.parts[&current_id];
            let Some(current_def) = part_defs.get(&current.definition_id) else {
                continue;
            };
            for (&other_id, other) in &self.parts {
                if visited.contains(&other_id) {
                    continue;
                }
                if let Some(other_def) = part_defs.get(&other.definition_id) {
                    if Self::weld_bounds_overlap(current.position, current_def, current.rotation, other.position, other_def, other.rotation) {
                        visited.insert(other_id);
                        queue.push_back(other_id);
                    }
                }
            }
        }
        visited
    }

    /// Convert editor state to a blueprint, filtering out disconnected parts
    pub fn to_blueprint(&self, part_defs: &PartDefinitions) -> Result<VesselBlueprint, String> {
        let root_id = self.root_part.ok_or("No root part (command pod) placed")?;

        if self.parts.is_empty() {
            return Err("No parts placed".to_string());
        }

        // Find connected parts and filter out disconnected ones
        let connected = self.connected_part_ids(part_defs);
        let filtered_parts: HashMap<PlacedPartId, PlacedPart> = self.parts.iter()
            .filter(|(&id, _)| connected.contains(&id))
            .map(|(&id, part)| (id, part.clone()))
            .collect();
        let filtered_stages: Vec<Vec<PlacedPartId>> = self.stages.iter()
            .map(|stage| stage.iter().copied().filter(|id| connected.contains(id)).collect())
            .filter(|stage: &Vec<PlacedPartId>| !stage.is_empty())
            .collect();

        Ok(parts_to_blueprint(
            &filtered_parts,
            root_id,
            self.vessel_name.clone(),
            &filtered_stages,
        ))
    }

    /// Get the center line X coordinate (root part's X position)
    pub fn center_line_x(&self) -> Option<f64> {
        self.root_part
            .and_then(|id| self.parts.get(&id))
            .map(|part| part.position[0])
    }

    /// Update ghost position based on mouse world coordinates
    pub fn update_ghost(&mut self, world_x: f64, world_y: f64, part_defs: &PartDefinitions) {
        let Some(ref def_id) = self.selected_part_def else {
            self.ghost_position = None;
            self.ghost_valid = false;
            self.mirror_ghost_position = None;
            self.mirror_ghost_def_id = None;
            return;
        };

        let Some(def) = part_defs.get(def_id) else {
            self.ghost_position = None;
            self.ghost_valid = false;
            self.mirror_ghost_position = None;
            self.mirror_ghost_def_id = None;
            return;
        };

        // Snap based on HITBOX dimensions (for proper alignment with other parts)
        // Use rotated dimensions for snapping
        let rot = self.ghost_rotation;
        let snap_grid_w = def.rotated_hitbox_grid_width(rot);
        let snap_grid_h = def.rotated_hitbox_grid_height(rot);
        // Odd dimensions snap to square center, even dimensions snap to grid line
        let snapped_x = if snap_grid_w % 2 == 1 {
            // Odd width: center on middle of square
            (world_x / GRID_SIZE).floor() * GRID_SIZE + GRID_SIZE / 2.0
        } else {
            // Even width: center on grid line
            (world_x / GRID_SIZE + 0.5).floor() * GRID_SIZE
        };

        let snapped_y = if snap_grid_h % 2 == 1 {
            // Odd height: center on middle of square
            (world_y / GRID_SIZE).floor() * GRID_SIZE + GRID_SIZE / 2.0
        } else {
            // Even height: center on grid line
            (world_y / GRID_SIZE + 0.5).floor() * GRID_SIZE
        };

        self.ghost_position = Some([snapped_x, snapped_y]);

        // Check if placement would overlap any existing part using HITBOX dimensions
        let rot_hitbox_w = def.rotated_hitbox_width(rot);
        let rot_hitbox_h = def.rotated_hitbox_height(rot);
        let new_bounds = Self::calc_bounds([snapped_x, snapped_y], rot_hitbox_w, rot_hitbox_h);

        let mut overlaps = false;
        for (_, part) in &self.parts {
            if let Some(existing_def) = part_defs.get(&part.definition_id) {
                let existing_bounds = Self::calc_bounds(part.position, existing_def.rotated_hitbox_width(part.rotation), existing_def.rotated_hitbox_height(part.rotation));
                if Self::bounds_overlap(&new_bounds, &existing_bounds) {
                    overlaps = true;
                    break;
                }
            }
        }

        // Compute mirror ghost if in Mirror mode with a center line
        self.mirror_ghost_position = None;
        self.mirror_ghost_def_id = None;
        if self.symmetry_mode == SymmetryMode::Mirror {
            if let Some(center_x) = self.center_line_x() {
                let mirror_x = center_x * 2.0 - snapped_x;
                // Only mirror if ghost is not on the center line
                if (snapped_x - center_x).abs() > GRID_SIZE * 0.1 {
                    self.mirror_ghost_position = Some([mirror_x, snapped_y]);

                    // Resolve mirror def id for asymmetric parts
                    self.mirror_ghost_def_id = def.mirror_def_id.as_ref()
                        .filter(|mid| part_defs.get(mid).is_some())
                        .cloned();

                    // Check mirror ghost overlap too
                    if !overlaps {
                        let mirror_bounds = Self::calc_bounds([mirror_x, snapped_y], rot_hitbox_w, rot_hitbox_h);
                        for (_, part) in &self.parts {
                            if let Some(existing_def) = part_defs.get(&part.definition_id) {
                                let existing_bounds = Self::calc_bounds(part.position, existing_def.rotated_hitbox_width(part.rotation), existing_def.rotated_hitbox_height(part.rotation));
                                if Self::bounds_overlap(&mirror_bounds, &existing_bounds) {
                                    overlaps = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Weld adjacency check: new part must touch at least one existing part
        // (skip for the very first part)
        // Decouplers can reach up to 10 grid squares above to attach
        let is_decoupler = def.decoupler.is_some();
        let extra_upward_reach = if is_decoupler { GRID_SIZE * 10.0 } else { 0.0 };

        let mut weld_connected = self.parts.is_empty();
        if !weld_connected {
            for (_, part) in &self.parts {
                if let Some(existing_def) = part_defs.get(&part.definition_id) {
                    if Self::weld_bounds_overlap_reach(
                        [snapped_x, snapped_y], def, rot, extra_upward_reach,
                        part.position, existing_def, part.rotation,
                    ) {
                        weld_connected = true;
                        break;
                    }
                }
            }
        }

        // Mirror ghost must also touch at least one existing part or the primary ghost
        if weld_connected && self.mirror_ghost_position.is_some() {
            let mirror_pos = self.mirror_ghost_position.unwrap();
            let mirror_rot = -rot;  // Mirror rotation
            let mut mirror_connected = false;
            // Check against existing parts
            for (_, part) in &self.parts {
                if let Some(existing_def) = part_defs.get(&part.definition_id) {
                    if Self::weld_bounds_overlap_reach(
                        mirror_pos, def, mirror_rot, extra_upward_reach,
                        part.position, existing_def, part.rotation,
                    ) {
                        mirror_connected = true;
                        break;
                    }
                }
            }
            // Check against primary ghost
            if !mirror_connected {
                mirror_connected = Self::weld_bounds_overlap(mirror_pos, def, mirror_rot, [snapped_x, snapped_y], def, rot);
            }
            if !mirror_connected {
                weld_connected = false;
            }
        }

        // Check if part crosses any completed fairing boundary
        let crosses_fairing = if !overlaps && weld_connected {
            // Don't check fairings against themselves
            let is_fairing = def.fairing.is_some();
            if is_fairing {
                false
            } else {
                self.part_crosses_any_fairing([snapped_x, snapped_y], def, part_defs, None)
            }
        } else {
            false
        };

        // Valid if no overlap with existing parts AND weld-connected AND not crossing fairing
        self.ghost_valid = !overlaps && weld_connected && !crosses_fairing;
    }

    /// Public bounds calculation (for use from main.rs rotation check)
    pub fn calc_bounds_pub(pos: [f64; 2], width: f64, height: f64) -> [f64; 4] {
        Self::calc_bounds(pos, width, height)
    }

    /// Public bounds overlap check (for use from main.rs rotation check)
    pub fn bounds_overlap_pub(a: &[f64; 4], b: &[f64; 4]) -> bool {
        Self::bounds_overlap(a, b)
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

    /// Check if two parts' welding hitboxes overlap (rotation-aware)
    fn weld_bounds_overlap(
        pos_a: [f64; 2], def_a: &crate::parts::PartDefinition, rot_a: f64,
        pos_b: [f64; 2], def_b: &crate::parts::PartDefinition, rot_b: f64,
    ) -> bool {
        let a = Self::calc_bounds(pos_a, def_a.rotated_weld_hitbox_width(rot_a), def_a.rotated_weld_hitbox_height(rot_a));
        let b = Self::calc_bounds(pos_b, def_b.rotated_weld_hitbox_width(rot_b), def_b.rotated_weld_hitbox_height(rot_b));
        Self::bounds_overlap(&a, &b)
    }

    /// Check weld overlap with extra upward reach on part A (for decoupler placement)
    fn weld_bounds_overlap_reach(
        pos_a: [f64; 2], def_a: &crate::parts::PartDefinition, rot_a: f64, extra_up: f64,
        pos_b: [f64; 2], def_b: &crate::parts::PartDefinition, rot_b: f64,
    ) -> bool {
        let mut a = Self::calc_bounds(pos_a, def_a.rotated_weld_hitbox_width(rot_a), def_a.rotated_weld_hitbox_height(rot_a));
        a[3] += extra_up; // extend top edge upward
        let b = Self::calc_bounds(pos_b, def_b.rotated_weld_hitbox_width(rot_b), def_b.rotated_weld_hitbox_height(rot_b));
        Self::bounds_overlap(&a, &b)
    }

    /// Interpolate fairing shell half-width at a given world-y coordinate.
    /// Returns None if y is outside the fairing's vertical range.
    fn fairing_half_width_at_y(
        shape: &FairingShape,
        base_top_y: f64,
        base_half_w: f64,
    ) -> impl Fn(f64) -> Option<f64> + '_ {
        let gs = GRID_SQUARE_SIZE;
        move |y: f64| {
            if y < base_top_y - 0.001 {
                return None; // Below fairing
            }
            let tip_y = base_top_y + shape.vertices.last().map(|v| v.1 * gs).unwrap_or(0.0);
            if y > tip_y + 0.001 {
                return None; // Above fairing
            }

            let mut prev_hw = base_half_w;
            let mut prev_y = base_top_y;

            for &(hw_grid, y_off_grid) in &shape.vertices {
                let seg_hw = hw_grid * gs;
                let seg_y = base_top_y + y_off_grid * gs;
                if y <= seg_y + 0.001 {
                    // Interpolate between prev and this segment
                    let span = seg_y - prev_y;
                    if span < 0.001 {
                        return Some(seg_hw);
                    }
                    let t = ((y - prev_y) / span).clamp(0.0, 1.0);
                    return Some(prev_hw + t * (seg_hw - prev_hw));
                }
                prev_hw = seg_hw;
                prev_y = seg_y;
            }
            Some(prev_hw)
        }
    }

    /// Check if a part's AABB crosses a completed fairing boundary.
    /// Returns true if the part straddles the boundary (partially inside, partially outside).
    /// Fully inside or fully outside is OK.
    fn part_crosses_fairing_boundary(
        part_pos: [f64; 2],
        part_def: &crate::parts::PartDefinition,
        fairing_pos: [f64; 2],
        fairing_def: &crate::parts::PartDefinition,
        fairing_shape: &FairingShape,
    ) -> bool {
        let gs = GRID_SQUARE_SIZE;
        let base_top_y = fairing_pos[1] + fairing_def.hitbox_height() / 2.0;
        let base_half_w = fairing_def.width() / 2.0;
        let tip_y = base_top_y + fairing_shape.vertices.last().map(|v| v.1 * gs).unwrap_or(0.0);
        let fairing_cx = fairing_pos[0];

        let part_half_w = part_def.hitbox_width() / 2.0;
        let part_half_h = part_def.hitbox_height() / 2.0;
        let part_left = part_pos[0] - part_half_w;
        let part_right = part_pos[0] + part_half_w;
        let part_bottom = part_pos[1] - part_half_h;
        let part_top = part_pos[1] + part_half_h;

        // No vertical overlap with fairing → fully outside → OK
        if part_top < base_top_y - 0.001 || part_bottom > tip_y + 0.001 {
            return false;
        }

        let half_w_at = Self::fairing_half_width_at_y(fairing_shape, base_top_y, base_half_w);

        // Collect y-values to sample: part bottom, part top, and each fairing vertex y within range
        let mut sample_ys = vec![part_bottom.max(base_top_y), part_top.min(tip_y)];
        for &(_, y_off_grid) in &fairing_shape.vertices {
            let fy = base_top_y + y_off_grid * gs;
            if fy > part_bottom + 0.001 && fy < part_top - 0.001 {
                sample_ys.push(fy);
            }
        }

        let mut any_inside = false;
        let mut any_outside = false;

        for y in &sample_ys {
            if let Some(hw) = half_w_at(*y) {
                let fairing_left = fairing_cx - hw;
                let fairing_right = fairing_cx + hw;

                // Part is inside if both edges are within fairing width
                let inside = part_left >= fairing_left - 0.001 && part_right <= fairing_right + 0.001;
                // Part is outside if both edges are outside fairing width
                let outside = part_right <= fairing_left + 0.001 || part_left >= fairing_right - 0.001;

                if inside {
                    any_inside = true;
                } else if outside {
                    any_outside = true;
                } else {
                    // Straddling at this height → crossing
                    return true;
                }
            }
        }

        // Mix of inside and outside across heights → crossing
        any_inside && any_outside
    }

    /// Check if a part crosses any completed fairing boundary
    fn part_crosses_any_fairing(
        &self,
        part_pos: [f64; 2],
        part_def: &crate::parts::PartDefinition,
        part_defs: &PartDefinitions,
        exclude_part_id: Option<PlacedPartId>,
    ) -> bool {
        for (&id, fairing_part) in &self.parts {
            if exclude_part_id == Some(id) {
                continue;
            }
            let Some(fairing_def) = part_defs.get(&fairing_part.definition_id) else { continue };
            if fairing_def.fairing.is_none() { continue; }
            let Some(ref shape) = fairing_part.fairing_shape else { continue };
            if !shape.closed { continue; }

            if Self::part_crosses_fairing_boundary(part_pos, part_def, fairing_part.position, fairing_def, shape) {
                return true;
            }
        }
        false
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
        let is_fairing = def.map(|d| d.fairing.is_some()).unwrap_or(false);
        let needs_staging = is_engine || is_decoupler || is_fairing;

        let mut part = PlacedPart::new(id, def_id.clone(), position);
        part.rotation = self.ghost_rotation;

        // First part becomes root
        if self.root_part.is_none() {
            self.root_part = Some(id);
        }

        // Mirror placement: place two linked parts if mirror ghost exists
        if let Some(mirror_pos) = self.mirror_ghost_position {
            let mirror_id = self.next_part_id;
            self.next_part_id += 1;

            // Use mirror def if available (e.g. right nose cone -> left nose cone)
            let mirror_def_id = self.mirror_ghost_def_id.clone().unwrap_or_else(|| def_id.clone());

            // Link the two parts
            part.mirror_partner = Some(mirror_id);
            let mut mirror_part = PlacedPart::new(mirror_id, mirror_def_id, mirror_pos);
            mirror_part.rotation = -self.ghost_rotation;  // Mirror rotation
            mirror_part.mirror_partner = Some(id);

            self.parts.insert(id, part);
            self.parts.insert(mirror_id, mirror_part);

            if needs_staging {
                if self.stages.is_empty() {
                    self.stages.push(Vec::new());
                }
                self.stages[0].push(id);
                self.stages[0].push(mirror_id);
            }
        } else {
            // Single placement (Off mode, or on center line in Mirror mode)
            self.parts.insert(id, part);

            if needs_staging {
                if self.stages.is_empty() {
                    self.stages.push(Vec::new());
                }
                self.stages[0].push(id);
            }
        }

        // Enter fairing build mode after placing a fairing base
        if is_fairing {
            if let Some(d) = def {
                let hitbox_half_h = d.hitbox_height() / 2.0;
                let base_top_y = position[1] + hitbox_half_h;
                let base_half_width = d.grid_width / 2.0;
                self.fairing_build_mode = Some(FairingBuildState {
                    part_id: id,
                    base_top_y,
                    base_center_x: position[0],
                    base_half_width,
                    vertices: Vec::new(),
                    ghost_point: None,
                    ghost_valid: false,
                });
                // Deselect the part def so no ghost part shows
                self.selected_part_def = None;
                self.ghost_position = None;
                self.ghost_valid = false;
                self.mirror_ghost_position = None;
                self.mirror_ghost_def_id = None;
            }
        }

        true
    }

    /// Delete a part (and its mirror partner if linked)
    pub fn delete_part(&mut self, part_id: PlacedPartId) {
        // Exit fairing build mode if deleting the fairing being built
        if self.fairing_build_mode.as_ref().map(|b| b.part_id) == Some(part_id) {
            self.fairing_build_mode = None;
        }

        // Check for mirror partner before removing
        let mirror_id = self.parts.get(&part_id).and_then(|p| p.mirror_partner);

        self.parts.remove(&part_id);

        // Also remove mirror partner
        if let Some(mid) = mirror_id {
            self.parts.remove(&mid);

            // Clear staging selection if it was the partner
            if self.staging_selected_engine == Some(mid) {
                self.staging_selected_engine = None;
            }
            // Clear selection if partner was selected
            if self.selected_placed_part == Some(mid) {
                self.selected_placed_part = None;
            }
        }

        // Update root if deleted
        if self.root_part == Some(part_id) || self.root_part == mirror_id {
            // Set new root to any remaining part
            self.root_part = self.parts.keys().next().copied();
        }

        // Remove both from stages and clean up empty stages
        for stage in &mut self.stages {
            stage.retain(|&sid| sid != part_id && Some(sid) != mirror_id);
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
        if self.fairing_build_mode.is_some() {
            self.exit_fairing_build_mode();
        }
        self.selected_part_def = None;
        self.selected_placed_part = None;
        self.ghost_position = None;
        self.ghost_valid = false;
        self.ghost_rotation = 0.0;
        self.mirror_ghost_position = None;
        self.mirror_ghost_def_id = None;
    }

    /// Select a placed part
    pub fn select_placed_part(&mut self, part_id: PlacedPartId) {
        self.selected_placed_part = Some(part_id);
        self.selected_part_def = None;
    }

    /// Start dragging a placed part (and its mirror partner if linked)
    pub fn start_drag(&mut self, part_id: PlacedPartId, mouse_world: [f64; 2]) {
        if let Some(part) = self.parts.get(&part_id) {
            self.dragging_part = Some(part_id);
            self.drag_start_pos = Some(part.position);
            self.drag_offset = [
                part.position[0] - mouse_world[0],
                part.position[1] - mouse_world[1],
            ];
            self.drag_valid = true;
            self.selected_placed_part = Some(part_id);
            self.selected_part_def = None;

            // Save mirror partner's position if linked
            self.drag_partner_start_pos = part.mirror_partner
                .and_then(|mid| self.parts.get(&mid))
                .map(|p| p.position);
        }
    }

    /// Update the position of the part being dragged (and mirror partner if linked)
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

        let mirror_id = part.mirror_partner;
        let part_rot = part.rotation;

        // Apply offset so the part doesn't jump to the cursor on first drag
        let target_x = world_x + self.drag_offset[0];
        let target_y = world_y + self.drag_offset[1];

        // Snap based on HITBOX dimensions (rotated)
        let snapped_x = if def.rotated_hitbox_grid_width(part_rot) % 2 == 1 {
            (target_x / GRID_SIZE).floor() * GRID_SIZE + GRID_SIZE / 2.0
        } else {
            (target_x / GRID_SIZE + 0.5).floor() * GRID_SIZE
        };

        let snapped_y = if def.rotated_hitbox_grid_height(part_rot) % 2 == 1 {
            (target_y / GRID_SIZE).floor() * GRID_SIZE + GRID_SIZE / 2.0
        } else {
            (target_y / GRID_SIZE + 0.5).floor() * GRID_SIZE
        };

        // Check if new position would overlap any other part
        let new_bounds = Self::calc_bounds([snapped_x, snapped_y], def.rotated_hitbox_width(part_rot), def.rotated_hitbox_height(part_rot));

        let mut overlaps = false;
        for (&other_id, other_part) in &self.parts {
            if other_id == part_id || Some(other_id) == mirror_id {
                continue; // Skip the part being dragged and its mirror partner
            }
            if let Some(other_def) = part_defs.get(&other_part.definition_id) {
                let other_bounds = Self::calc_bounds(other_part.position, other_def.rotated_hitbox_width(other_part.rotation), other_def.rotated_hitbox_height(other_part.rotation));
                if Self::bounds_overlap(&new_bounds, &other_bounds) {
                    overlaps = true;
                    break;
                }
            }
        }

        // Check mirror partner overlap if linked
        if let Some(mid) = mirror_id {
            if !overlaps {
                if let Some(center_x) = self.center_line_x() {
                    let mirror_x = center_x * 2.0 - snapped_x;
                    let mirror_bounds = Self::calc_bounds([mirror_x, snapped_y], def.rotated_hitbox_width(part_rot), def.rotated_hitbox_height(part_rot));

                    // Also check primary vs mirror overlap
                    if Self::bounds_overlap(&new_bounds, &mirror_bounds) {
                        overlaps = true;
                    }

                    if !overlaps {
                        for (&other_id, other_part) in &self.parts {
                            if other_id == part_id || other_id == mid {
                                continue;
                            }
                            if let Some(other_def) = part_defs.get(&other_part.definition_id) {
                                let other_bounds = Self::calc_bounds(other_part.position, other_def.rotated_hitbox_width(other_part.rotation), other_def.rotated_hitbox_height(other_part.rotation));
                                if Self::bounds_overlap(&mirror_bounds, &other_bounds) {
                                    overlaps = true;
                                    break;
                                }
                            }
                        }
                    }

                    // Update mirror partner position
                    if let Some(mirror_part) = self.parts.get_mut(&mid) {
                        mirror_part.position = [mirror_x, snapped_y];
                    }
                }
            }
        }

        // Also check fairing boundary crossing for non-fairing parts
        if !overlaps {
            let is_fairing = def.fairing.is_some();
            if !is_fairing && self.part_crosses_any_fairing([snapped_x, snapped_y], def, part_defs, Some(part_id)) {
                overlaps = true;
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

        let partner_start_pos = self.drag_partner_start_pos.take();

        // Check final validity
        let Some(part) = self.parts.get(&part_id) else {
            return;
        };

        let Some(def) = part_defs.get(&part.definition_id) else {
            return;
        };

        let mirror_id = part.mirror_partner;
        let current_pos = part.position;
        let part_rot = part.rotation;
        let bounds = Self::calc_bounds(current_pos, def.rotated_hitbox_width(part_rot), def.rotated_hitbox_height(part_rot));

        let mut overlaps = false;
        for (&other_id, other_part) in &self.parts {
            if other_id == part_id || Some(other_id) == mirror_id {
                continue;
            }
            if let Some(other_def) = part_defs.get(&other_part.definition_id) {
                let other_bounds = Self::calc_bounds(other_part.position, other_def.rotated_hitbox_width(other_part.rotation), other_def.rotated_hitbox_height(other_part.rotation));
                if Self::bounds_overlap(&bounds, &other_bounds) {
                    overlaps = true;
                    break;
                }
            }
        }

        // Check mirror partner overlap
        if !overlaps {
            if let Some(mid) = mirror_id {
                if let Some(mirror_part) = self.parts.get(&mid) {
                    let mirror_bounds = Self::calc_bounds(mirror_part.position, def.rotated_hitbox_width(part_rot), def.rotated_hitbox_height(part_rot));

                    // Check primary vs mirror
                    if Self::bounds_overlap(&bounds, &mirror_bounds) {
                        overlaps = true;
                    }

                    if !overlaps {
                        for (&other_id, other_part) in &self.parts {
                            if other_id == part_id || other_id == mid {
                                continue;
                            }
                            if let Some(other_def) = part_defs.get(&other_part.definition_id) {
                                let other_bounds = Self::calc_bounds(other_part.position, other_def.rotated_hitbox_width(other_part.rotation), other_def.rotated_hitbox_height(other_part.rotation));
                                if Self::bounds_overlap(&mirror_bounds, &other_bounds) {
                                    overlaps = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Revert to original positions if invalid
        if overlaps {
            if let Some(part) = self.parts.get_mut(&part_id) {
                part.position = start_pos;
            }
            if let (Some(mid), Some(partner_pos)) = (mirror_id, partner_start_pos) {
                if let Some(mirror_part) = self.parts.get_mut(&mid) {
                    mirror_part.position = partner_pos;
                }
            }
        }

        self.drag_valid = true;
    }

    /// Cancel dragging and revert to original position
    pub fn cancel_drag(&mut self) {
        if let (Some(part_id), Some(start_pos)) = (self.dragging_part.take(), self.drag_start_pos.take()) {
            // Get mirror partner before mutating
            let mirror_id = self.parts.get(&part_id).and_then(|p| p.mirror_partner);

            if let Some(part) = self.parts.get_mut(&part_id) {
                part.position = start_pos;
            }

            // Revert mirror partner position
            if let (Some(mid), Some(partner_pos)) = (mirror_id, self.drag_partner_start_pos.take()) {
                if let Some(mirror_part) = self.parts.get_mut(&mid) {
                    mirror_part.position = partner_pos;
                }
            }
        }
        self.drag_partner_start_pos = None;
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
        self.camera_zoom = self.camera_zoom.clamp(0.033, 16666.0);  // Zoom range
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
        self.camera_zoom = self.camera_zoom.clamp(0.033, 16666.0);
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

    /// Check if an engine is covered by a decoupler directly below it.
    /// A covered engine has a decoupler whose top edge is near/touching the engine's bottom
    /// edge and whose horizontal extent overlaps the engine's.
    fn is_editor_engine_covered(
        &self,
        engine_id: PlacedPartId,
        part_defs: &PartDefinitions,
    ) -> bool {
        self.is_editor_engine_covered_with_decoupled(engine_id, part_defs, &HashSet::new())
    }

    /// Check engine coverage considering a set of already-decoupled parts.
    /// Decouplers in the decoupled set no longer block engines.
    fn is_editor_engine_covered_with_decoupled(
        &self,
        engine_id: PlacedPartId,
        part_defs: &PartDefinitions,
        decoupled: &HashSet<PlacedPartId>,
    ) -> bool {
        let Some(engine_part) = self.parts.get(&engine_id) else { return false };
        let Some(engine_def) = part_defs.get(&engine_part.definition_id) else { return false };
        let engine_bottom = engine_part.position[1] - engine_def.hitbox_height() / 2.0;
        let engine_left = engine_part.position[0] - engine_def.hitbox_width() / 2.0;
        let engine_right = engine_part.position[0] + engine_def.hitbox_width() / 2.0;

        for (&other_id, other_part) in &self.parts {
            if other_id == engine_id || decoupled.contains(&other_id) {
                continue;
            }
            let Some(other_def) = part_defs.get(&other_part.definition_id) else { continue };
            if other_def.decoupler.is_none() {
                continue;
            }
            let decoupler_top = other_part.position[1] + other_def.hitbox_height() / 2.0;
            let decoupler_left = other_part.position[0] - other_def.hitbox_width() / 2.0;
            let decoupler_right = other_part.position[0] + other_def.hitbox_width() / 2.0;

            if (decoupler_top - engine_bottom).abs() < 0.3
                && engine_left < decoupler_right
                && engine_right > decoupler_left
            {
                return true;
            }
        }
        false
    }

    /// Calculate ship statistics from placed parts
    pub fn calculate_stats(&self, part_defs: &PartDefinitions) -> ShipStats {
        let mut stats = ShipStats::default();

        for (&part_id, part) in &self.parts {
            let Some(def) = part_defs.get(&part.definition_id) else {
                continue;
            };

            // Add dry mass
            stats.dry_mass += def.mass;

            // Add engine thrust (skip engines covered by a decoupler below)
            if let Some(ref engine) = def.engine {
                if !self.is_editor_engine_covered(part_id, part_defs) {
                    stats.thrust_vac += engine.thrust_vac;
                    stats.thrust_asl += engine.thrust_asl;
                }
            }

            // Add tank resources based on fill_fraction
            if let Some(ref tank) = def.tank {
                if part.fill_fraction > 0.0 && part.fuel_type != FuelType::Empty {
                    let (ox_kg, fuel_kg) = tank.propellant_capacity(part.fuel_type);

                    // Add oxygen
                    if ox_kg > 0.0 {
                        let ox_entry = stats.resources.entry("oxygen".to_string()).or_default();
                        ox_entry.current += ox_kg * part.fill_fraction;
                        ox_entry.max += ox_kg;
                    }

                    // Add fuel
                    if let Some(fuel_name) = part.fuel_type.fuel_resource_name() {
                        let fuel_entry = stats.resources.entry(fuel_name.to_string()).or_default();
                        fuel_entry.current += fuel_kg * part.fill_fraction;
                        fuel_entry.max += fuel_kg;
                    }
                }
            }

            // Battery capacity
            if let Some(ref battery) = def.battery {
                stats.electricity_capacity += battery.capacity_wh;
            }

            // Solar panel generation (at 1 AU)
            if let Some(ref solar) = def.solar_panel {
                stats.power_generation += solar.output_1au;
            }

            // RTG generation
            if let Some(ref rtg) = def.rtg {
                stats.power_generation += rtg.output_watts;
            }

            // Pod power consumption
            if let Some(ref pod) = def.pod {
                stats.power_consumption += pod.power_draw;
            }
        }

        // Calculate wet mass (dry mass + resource mass in tonnes)
        // Resources are in kg, convert to tonnes
        let resource_mass: f64 = stats.resources.values().map(|r| r.current / 1000.0).sum();
        stats.wet_mass = stats.dry_mass + resource_mass;

        stats
    }

    /// Compute fuel zones among non-decoupled editor parts.
    /// BFS through weld adjacency; non-crossfeed decouplers act as barriers.
    /// Returns a map from part ID to zone index.
    fn compute_editor_fuel_zones(
        &self,
        part_defs: &PartDefinitions,
        decoupled: &HashSet<PlacedPartId>,
    ) -> HashMap<PlacedPartId, usize> {
        let active_ids: Vec<PlacedPartId> = self.parts.keys()
            .copied()
            .filter(|id| !decoupled.contains(id))
            .collect();

        let mut zone_of: HashMap<PlacedPartId, usize> = HashMap::new();
        let mut current_zone = 0usize;

        for &start_id in &active_ids {
            if zone_of.contains_key(&start_id) { continue; }

            let mut queue = VecDeque::new();
            zone_of.insert(start_id, current_zone);
            queue.push_back(start_id);

            while let Some(current_id) = queue.pop_front() {
                let current = &self.parts[&current_id];
                let Some(current_def) = part_defs.get(&current.definition_id) else { continue };

                // Non-crossfeed decouplers are barriers: assigned to zone but don't propagate
                let is_barrier = current_def.decoupler.is_some() && !current.crossfeed_enabled;
                if is_barrier && current_id != start_id {
                    continue;
                }

                for &other_id in &active_ids {
                    if zone_of.contains_key(&other_id) { continue; }
                    let other = &self.parts[&other_id];
                    let Some(other_def) = part_defs.get(&other.definition_id) else { continue };
                    if Self::weld_bounds_overlap(current.position, current_def, current.rotation, other.position, other_def, other.rotation) {
                        zone_of.insert(other_id, current_zone);
                        queue.push_back(other_id);
                    }
                }
            }
            current_zone += 1;
        }

        zone_of
    }

    /// Calculate per-stage delta-v (vacuum) using the Tsiolkovsky rocket equation.
    /// Simulates staging sequentially: decouplers fire, engines activate.
    /// Fuel zones (divided by non-crossfeed decouplers) determine which fuel
    /// is accessible to active engines in each stage.
    pub fn calculate_stage_delta_v(&self, part_defs: &PartDefinitions) -> Vec<f64> {
        let g0 = 9.80665;
        let mut stage_dvs = Vec::new();

        // Track state across stages
        let mut decoupled: HashSet<PlacedPartId> = HashSet::new();
        let mut engines_enabled: HashSet<PlacedPartId> = HashSet::new();

        // Track remaining fuel per part (in tonnes)
        let mut fuel_remaining: HashMap<PlacedPartId, f64> = HashMap::new();
        for (&part_id, part) in &self.parts {
            let Some(def) = part_defs.get(&part.definition_id) else { continue };
            if let Some(ref tank) = def.tank {
                if part.fill_fraction > 0.0 && part.fuel_type != FuelType::Empty {
                    let (ox_kg, fuel_kg) = tank.propellant_capacity(part.fuel_type);
                    fuel_remaining.insert(part_id, (ox_kg + fuel_kg) * part.fill_fraction / 1000.0);
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
                let Some(part) = self.parts.get(&part_id) else { continue };
                let Some(def) = part_defs.get(&part.definition_id) else { continue };
                if def.engine.is_some() {
                    engines_enabled.insert(part_id);
                }
            }

            // 3. Compute fuel zones — non-crossfeed decouplers divide the rocket
            let zone_of = self.compute_editor_fuel_zones(part_defs, &decoupled);

            // 4. Find zones containing active (non-decoupled, non-covered) engines
            let engine_zones: HashSet<usize> = engines_enabled.iter()
                .filter(|id| !decoupled.contains(id))
                .filter(|id| !self.is_editor_engine_covered_with_decoupled(**id, part_defs, &decoupled))
                .filter_map(|id| zone_of.get(id).copied())
                .collect();

            // 5. Calculate wet mass of ALL remaining parts, but only count
            //    fuel from tanks in engine zones as burnable
            let mut wet_mass = 0.0;
            let mut burnable_fuel = 0.0;
            for (&part_id, part) in &self.parts {
                if decoupled.contains(&part_id) { continue; }
                let Some(def) = part_defs.get(&part.definition_id) else { continue };
                let part_fuel = fuel_remaining.get(&part_id).copied().unwrap_or(0.0);
                wet_mass += def.mass + part_fuel;

                if let Some(&zone) = zone_of.get(&part_id) {
                    if engine_zones.contains(&zone) {
                        burnable_fuel += part_fuel;
                    }
                }
            }

            // 6. Calculate thrust-weighted average Isp (skip covered engines)
            let mut total_thrust = 0.0;
            let mut weighted_isp = 0.0;
            for &engine_id in &engines_enabled {
                if decoupled.contains(&engine_id) { continue; }
                if self.is_editor_engine_covered_with_decoupled(engine_id, part_defs, &decoupled) { continue; }
                let Some(part) = self.parts.get(&engine_id) else { continue };
                let Some(def) = part_defs.get(&part.definition_id) else { continue };
                if let Some(ref engine) = def.engine {
                    total_thrust += engine.thrust_vac;
                    weighted_isp += engine.thrust_vac * engine.isp_vac;
                }
            }
            let isp = if total_thrust > 0.0 { weighted_isp / total_thrust } else { 0.0 };

            // 7. Δv = Isp * g0 * ln(wet / dry)
            let dry_mass = wet_mass - burnable_fuel;
            let dv = if isp > 0.0 && dry_mass > 0.0 && wet_mass > dry_mass {
                isp * g0 * (wet_mass / dry_mass).ln()
            } else {
                0.0
            };
            stage_dvs.push(dv);

            // 8. Consume only burnable fuel (in engine zones)
            for (&part_id, fuel) in fuel_remaining.iter_mut() {
                if decoupled.contains(&part_id) { continue; }
                if let Some(&zone) = zone_of.get(&part_id) {
                    if engine_zones.contains(&zone) {
                        *fuel = 0.0;
                    }
                }
            }
        }

        stage_dvs
    }

    // === Fairing Build Mode ===

    /// Exit fairing build mode, saving the current vertices to the part
    pub fn exit_fairing_build_mode(&mut self) {
        if let Some(build) = self.fairing_build_mode.take() {
            if let Some(part) = self.parts.get_mut(&build.part_id) {
                if !build.vertices.is_empty() {
                    part.fairing_shape = Some(FairingShape {
                        vertices: build.vertices,
                        closed: false, // not closed unless via center-line point
                    });
                }
            }
        }
    }

    /// Add a fairing vertex at the current ghost point
    pub fn add_fairing_vertex(&mut self, part_defs: &PartDefinitions) {
        let (ghost, base_center_x, base_top_y, ghost_valid, part_id) = {
            let Some(ref build) = self.fairing_build_mode else { return };
            (build.ghost_point, build.base_center_x, build.base_top_y, build.ghost_valid, build.part_id)
        };

        if !ghost_valid {
            return;
        }

        let Some([gx, gy]) = ghost else { return };

        // Convert from world coords to grid-relative coords (half_width_grid, y_offset_grid)
        let half_width_world = (gx - base_center_x).abs();
        let y_offset_world = gy - base_top_y;
        let half_width_grid = half_width_world / GRID_SQUARE_SIZE;
        let y_offset_grid = y_offset_world / GRID_SQUARE_SIZE;

        // Check if this is a closing point (on center line)
        let is_closing = half_width_grid < 0.25; // Within tolerance of center

        // On close, validate that no existing part crosses the about-to-be-closed fairing boundary
        if is_closing {
            let fairing_pos = self.parts.get(&part_id).map(|p| p.position).unwrap_or([0.0, 0.0]);
            let Some(fairing_def) = self.parts.get(&part_id)
                .and_then(|p| part_defs.get(&p.definition_id)) else { return };

            // Build the closing shape
            let build = self.fairing_build_mode.as_ref().unwrap();
            let mut closing_verts = build.vertices.clone();
            closing_verts.push((0.0, y_offset_grid));
            let closing_shape = FairingShape { vertices: closing_verts, closed: true };

            // Check all parts against this envelope
            for (&id, part) in &self.parts {
                if id == part_id { continue; }
                let Some(pdef) = part_defs.get(&part.definition_id) else { continue };
                if Self::part_crosses_fairing_boundary(part.position, pdef, fairing_pos, fairing_def, &closing_shape) {
                    // Reject the close
                    return;
                }
            }
        }

        let build = self.fairing_build_mode.as_mut().unwrap();

        if is_closing {
            // Close the fairing with a tip point at center
            build.vertices.push((0.0, y_offset_grid));

            // Save to part and exit
            if let Some(part) = self.parts.get_mut(&build.part_id) {
                part.fairing_shape = Some(FairingShape {
                    vertices: build.vertices.clone(),
                    closed: true,
                });
            }
            self.fairing_build_mode = None;
        } else {
            build.vertices.push((half_width_grid, y_offset_grid));
        }
    }

    /// Undo the last fairing vertex
    pub fn undo_fairing_vertex(&mut self) {
        let should_exit = if let Some(ref mut build) = self.fairing_build_mode {
            if build.vertices.is_empty() {
                true // No vertices to undo, exit build mode
            } else {
                build.vertices.pop();
                false
            }
        } else {
            false
        };
        if should_exit {
            self.exit_fairing_build_mode();
        }
    }

    /// Update the fairing ghost point based on cursor position
    pub fn update_fairing_ghost(&mut self, world_x: f64, world_y: f64) {
        let Some(ref mut build) = self.fairing_build_mode else { return };

        // Snap to half-grid positions (every 0.25m = every half grid square)
        let snap = |v: f64| -> f64 { (v / 0.25).round() * 0.25 };
        let snapped_x = snap(world_x);
        let snapped_y = snap(world_y);

        // Calculate half_width from center
        let half_width_world = (snapped_x - build.base_center_x).abs();
        let half_width_grid = half_width_world / GRID_SQUARE_SIZE;
        let y_offset_world = snapped_y - build.base_top_y;

        // Validate:
        // 1. y must be above last vertex (or base top if first)
        let min_y_offset = if let Some(&(_, last_y)) = build.vertices.last() {
            last_y * GRID_SQUARE_SIZE + 0.25 // Must be at least half-grid above last point
        } else {
            0.25 // Must be at least half-grid above base top
        };
        let y_valid = y_offset_world >= min_y_offset;

        // 2. half_width can extend up to base_half_width beyond the base edge
        //    (i.e., max half_width = base_half_width * 2)
        let max_half_width = build.base_half_width * 2.0;
        let width_valid = half_width_grid <= max_half_width + 0.01;

        // 3. y must be <= base_top_y + 10 * base_width (reasonable max height)
        let max_y_offset = build.base_half_width * 2.0 * 10.0 * GRID_SQUARE_SIZE;
        let height_valid = y_offset_world <= max_y_offset;

        build.ghost_valid = y_valid && width_valid && height_valid;

        // Mirror the x to center (use positive side, snap to center if close)
        let mirrored_x = if half_width_grid < 0.25 {
            build.base_center_x // Snap to center for closing
        } else {
            // Mirror to the side closest to cursor
            if snapped_x >= build.base_center_x {
                build.base_center_x + half_width_world
            } else {
                build.base_center_x - half_width_world
            }
        };
        let _ = mirrored_x; // We use the absolute half_width, drawing is symmetric

        // Store the ghost point (always on positive side for rendering)
        build.ghost_point = Some([
            build.base_center_x + half_width_world,
            snapped_y,
        ]);
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
