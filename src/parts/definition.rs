use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Size categories for parts (width in grid squares)
/// Tiny=1, Small=3, Medium=5, Large=9, XL=13
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartSize {
    Tiny,   // 1 grid square wide
    Small,  // 3 grid squares wide
    Medium, // 5 grid squares wide
    Large,  // 9 grid squares wide
    XL,     // 13 grid squares wide
}

impl PartSize {
    /// Width in grid squares for this part size
    pub fn grid_width(&self) -> u32 {
        match self {
            PartSize::Tiny => 1,
            PartSize::Small => 3,
            PartSize::Medium => 5,
            PartSize::Large => 9,
            PartSize::XL => 13,
        }
    }

    /// Display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            PartSize::Tiny => "Tiny",
            PartSize::Small => "Small",
            PartSize::Medium => "Medium",
            PartSize::Large => "Large",
            PartSize::XL => "XL",
        }
    }

    /// All sizes in order
    pub fn all() -> &'static [PartSize] {
        &[PartSize::Tiny, PartSize::Small, PartSize::Medium, PartSize::Large, PartSize::XL]
    }
}

/// Shape of a part for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartShape {
    Rectangle,     // Simple rectangle (width x height)
    Triangle,      // Triangle with base at bottom
    Trapezoid,     // Trapezoid (top_width, bottom_width, height)
    TriangleRight, // Right triangle: vertical edge on right, hypotenuse on left
    TriangleLeft,  // Right triangle: vertical edge on left, hypotenuse on right
}

/// Part categories for organizing in the editor palette
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartCategory {
    Pods,
    Propulsion,
    FuelTanks,
    Structural,
    Aerodynamic,
    Utility,
}

impl PartCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            PartCategory::Pods => "Command",
            PartCategory::Propulsion => "Engines",
            PartCategory::FuelTanks => "Fuel Tanks",
            PartCategory::Structural => "Structural",
            PartCategory::Aerodynamic => "Aerodynamic",
            PartCategory::Utility => "Utility",
        }
    }

    pub fn all() -> &'static [PartCategory] {
        &[
            PartCategory::Pods,
            PartCategory::Propulsion,
            PartCategory::FuelTanks,
            PartCategory::Structural,
            PartCategory::Aerodynamic,
            PartCategory::Utility,
        ]
    }
}

/// Grid square size in meters (for converting grid units to world units)
pub const GRID_SQUARE_SIZE: f64 = 0.5;

/// Propellant types for engines
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Propellant {
    #[default]
    Kerolox,    // LOX + RP-1 (kerosene)
    Methalox,   // LOX + Methane
    Hydrolox,   // LOX + Hydrogen
}

impl Propellant {
    pub fn display_name(&self) -> &'static str {
        match self {
            Propellant::Kerolox => "LOX/RP-1",
            Propellant::Methalox => "LOX/CH4",
            Propellant::Hydrolox => "LOX/LH2",
        }
    }

    /// Get the corresponding tank fuel type
    pub fn fuel_type(&self) -> FuelType {
        match self {
            Propellant::Kerolox => FuelType::Rp1,
            Propellant::Methalox => FuelType::Methane,
            Propellant::Hydrolox => FuelType::Hydrogen,
        }
    }
}

/// Engine-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineData {
    pub thrust_vac: f64,      // kN vacuum thrust
    pub thrust_asl: f64,      // kN sea-level thrust
    pub isp_vac: f64,         // Specific impulse in vacuum (seconds)
    pub isp_asl: f64,         // Specific impulse at sea level
    pub throttleable: bool,   // Can throttle?
    #[serde(default)]
    pub gimbal_range: f64,    // Gimbal range in degrees (0 = fixed)
    #[serde(default)]
    pub propellant: Propellant,  // Propellant type
}

/// Fuel types for tanks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FuelType {
    #[default]
    Empty,          // No fuel loaded
    Rp1,            // LOX + RP-1 (kerosene)
    Methane,        // LOX + Methane
    Hydrogen,       // LOX + Hydrogen
    Monopropellant, // Monopropellant (no oxidizer)
}

impl FuelType {
    pub fn display_name(&self) -> &'static str {
        match self {
            FuelType::Empty => "Empty",
            FuelType::Rp1 => "LOX/RP-1",
            FuelType::Methane => "LOX/CH4",
            FuelType::Hydrogen => "LOX/LH2",
            FuelType::Monopropellant => "Monopropellant",
        }
    }

    pub fn all() -> &'static [FuelType] {
        &[FuelType::Empty, FuelType::Rp1, FuelType::Methane, FuelType::Hydrogen, FuelType::Monopropellant]
    }

    /// Get propellant masses per grid square (in kg)
    /// Returns (oxygen_kg, fuel_kg)
    pub fn propellant_per_grid_square(&self) -> (f64, f64) {
        match self {
            FuelType::Empty => (0.0, 0.0),
            FuelType::Rp1 => (470.0, 185.0),
            FuelType::Methane => (270.0, 75.0),
            FuelType::Hydrogen => (155.0, 25.0),
            FuelType::Monopropellant => (0.0, 200.0),
        }
    }

    /// Get the fuel resource name
    pub fn fuel_resource_name(&self) -> Option<&'static str> {
        match self {
            FuelType::Empty => None,
            FuelType::Rp1 => Some("rp1"),
            FuelType::Methane => Some("methane"),
            FuelType::Hydrogen => Some("hydrogen"),
            FuelType::Monopropellant => Some("monopropellant"),
        }
    }
}

/// Tank-specific data (for fuel tanks)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TankData {
    pub grid_area: u32,  // Tank area in grid squares (for capacity calculation)
}

impl TankData {
    /// Dry mass in kg (35 kg per grid square)
    pub fn dry_mass_kg(&self) -> f64 {
        self.grid_area as f64 * 35.0
    }

    /// Get propellant masses for a given fuel type (in kg)
    /// Returns (oxygen_kg, fuel_kg)
    pub fn propellant_capacity(&self, fuel_type: FuelType) -> (f64, f64) {
        let (ox_per_sq, fuel_per_sq) = fuel_type.propellant_per_grid_square();
        (ox_per_sq * self.grid_area as f64, fuel_per_sq * self.grid_area as f64)
    }
}

/// Decoupler-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecouplerData {
    pub ejection_force: f64,  // Force in kN when decoupling
    #[serde(default)]
    pub is_radial: bool,      // Radial decoupler (separates sideways, not by Y position)
}

/// Pod-specific data (for command modules)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodData {
    pub crew_capacity: u32,
}

/// Fairing-specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FairingData {
    pub ejection_force: f64,  // Force in kN when jettisoning
}

/// RCS thruster data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RcsData {
    pub thrust: f64,  // kN per axis
    pub isp: f64,     // Specific impulse (seconds)
    #[serde(default)]
    pub is_mirrored: bool,  // true = left-mount (nozzles point right/up/down)
}

/// A part definition loaded from RON files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PartCategory,
    pub mass: f64,            // Dry mass in tonnes
    pub cost: u32,
    pub size: PartSize,
    pub shape: PartShape,
    // Visual dimensions (what is drawn) - can be decimals for fine-tuning
    pub grid_width: f64,      // Visual width in grid squares (can be decimal)
    pub grid_height: f64,     // Visual height in grid squares (can be decimal)
    #[serde(default)]
    pub top_width: Option<f64>, // For trapezoids: visual width at top in grid squares
    // Hitbox dimensions (for placement/collision) - defaults to ceiling of visual if not specified
    #[serde(default)]
    pub hitbox_width: Option<u32>,  // Hitbox width in grid squares
    #[serde(default)]
    pub hitbox_height: Option<u32>, // Hitbox height in grid squares
    #[serde(default)]
    pub tech_required: String,
    #[serde(default)]
    pub engine: Option<EngineData>,
    #[serde(default)]
    pub tank: Option<TankData>,
    #[serde(default)]
    pub pod: Option<PodData>,
    #[serde(default)]
    pub decoupler: Option<DecouplerData>,
    #[serde(default)]
    pub rcs: Option<RcsData>,
    #[serde(default)]
    pub fairing: Option<FairingData>,
    #[serde(default)]
    pub resources: HashMap<String, f64>,
    // Thermal properties
    #[serde(default = "default_heat_tolerance")]
    pub max_heat_tolerance: f64,
    #[serde(default = "default_specific_heat")]
    pub specific_heat: f64,
    #[serde(default = "default_emissivity")]
    pub emissivity: f64,
    #[serde(default)]
    pub is_heat_shield: bool,
    #[serde(default)]
    pub mirror_def_id: Option<String>,
}

pub fn default_heat_tolerance() -> f64 { 1000.0 }
fn default_specific_heat() -> f64 { 900.0 }
fn default_emissivity() -> f64 { 0.8 }

/// Welding hitbox extends 5% past the build/flight hitbox
const WELD_HITBOX_PADDING: f64 = 0.05;

impl PartDefinition {
    // --- Visual dimensions (what is drawn) ---

    /// Visual width in meters
    pub fn width(&self) -> f64 {
        self.grid_width * GRID_SQUARE_SIZE
    }

    /// Visual height in meters
    pub fn height(&self) -> f64 {
        self.grid_height * GRID_SQUARE_SIZE
    }

    /// Visual top width in meters (for trapezoids)
    pub fn top_width(&self) -> f64 {
        self.top_width.unwrap_or(self.grid_width) * GRID_SQUARE_SIZE
    }

    // --- Hitbox dimensions (for placement, collision, centering) ---

    /// Hitbox width in grid squares (defaults to ceiling of visual width)
    pub fn hitbox_grid_width(&self) -> u32 {
        self.hitbox_width.unwrap_or_else(|| self.grid_width.ceil() as u32)
    }

    /// Hitbox height in grid squares (defaults to ceiling of visual height)
    pub fn hitbox_grid_height(&self) -> u32 {
        self.hitbox_height.unwrap_or_else(|| self.grid_height.ceil() as u32)
    }

    /// Hitbox width in meters
    pub fn hitbox_width(&self) -> f64 {
        self.hitbox_grid_width() as f64 * GRID_SQUARE_SIZE
    }

    /// Hitbox height in meters
    pub fn hitbox_height(&self) -> f64 {
        self.hitbox_grid_height() as f64 * GRID_SQUARE_SIZE
    }

    // --- Welding hitbox (5% larger than build/flight hitbox) ---

    /// Welding hitbox width in meters
    pub fn weld_hitbox_width(&self) -> f64 {
        self.hitbox_width() * (1.0 + WELD_HITBOX_PADDING)
    }

    /// Welding hitbox height in meters
    pub fn weld_hitbox_height(&self) -> f64 {
        self.hitbox_height() * (1.0 + WELD_HITBOX_PADDING)
    }

    /// Whether this part can be a root part (command pod)
    pub fn can_be_root(&self) -> bool {
        self.category == PartCategory::Pods
    }

    /// Get total mass including resources
    pub fn wet_mass(&self) -> f64 {
        self.mass + self.resources.values().sum::<f64>() * 0.001 // resources in kg
    }
}

/// Container for part definitions from a RON file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartDefinitionFile {
    pub parts: Vec<PartDefinition>,
}

/// Registry of all loaded part definitions
#[derive(Debug, Clone, Default)]
pub struct PartDefinitions {
    parts: HashMap<String, PartDefinition>,
}

impl PartDefinitions {
    pub fn new() -> Self {
        Self {
            parts: HashMap::new(),
        }
    }

    /// Load all part definition files from a directory
    pub fn load_from_directory<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut definitions = Self::new();
        let dir = path.as_ref();

        if !dir.exists() {
            return Err(format!("Parts directory does not exist: {:?}", dir));
        }

        for entry in fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "ron") {
                log::info!("Loading parts from: {:?}", path);
                definitions.load_file(&path)?;
            }
        }

        log::info!("Loaded {} part definitions", definitions.parts.len());
        Ok(definitions)
    }

    /// Load a single RON file
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {:?}: {}", path.as_ref(), e))?;

        let file: PartDefinitionFile = ron::from_str(&content)
            .map_err(|e| format!("Failed to parse {:?}: {}", path.as_ref(), e))?;

        for part in file.parts {
            log::debug!("  Loaded part: {} ({})", part.name, part.id);
            self.parts.insert(part.id.clone(), part);
        }

        Ok(())
    }

    /// Get a part definition by ID
    pub fn get(&self, id: &str) -> Option<&PartDefinition> {
        self.parts.get(id)
    }

    /// Get all parts in a category, sorted by width then height
    pub fn by_category(&self, category: PartCategory) -> Vec<&PartDefinition> {
        let mut parts: Vec<_> = self.parts
            .values()
            .filter(|p| p.category == category)
            .collect();

        // Sort by width first, then by height
        parts.sort_by(|a, b| {
            a.grid_width.total_cmp(&b.grid_width)
                .then(a.grid_height.total_cmp(&b.grid_height))
        });

        parts
    }

    /// Get all parts in a category and size, sorted by height
    pub fn by_category_and_size(&self, category: PartCategory, size: PartSize) -> Vec<&PartDefinition> {
        let mut parts: Vec<_> = self.parts
            .values()
            .filter(|p| p.category == category && p.size == size)
            .collect();

        // Sort by height
        parts.sort_by(|a, b| a.grid_height.total_cmp(&b.grid_height));

        parts
    }

    /// Check if any parts exist for a category and size
    pub fn has_parts_for_size(&self, category: PartCategory, size: PartSize) -> bool {
        self.parts.values().any(|p| p.category == category && p.size == size)
    }

    /// Get all part definitions
    pub fn all(&self) -> impl Iterator<Item = &PartDefinition> {
        self.parts.values()
    }

    /// Check if any parts are loaded
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part_size_grid() {
        assert_eq!(PartSize::Tiny.grid_width(), 1);
        assert_eq!(PartSize::Small.grid_width(), 3);
        assert_eq!(PartSize::Medium.grid_width(), 5);
        assert_eq!(PartSize::Large.grid_width(), 9);
        assert_eq!(PartSize::XL.grid_width(), 13);
    }
}
