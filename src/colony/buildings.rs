use serde::{Deserialize, Serialize};

use super::resources::{ResourceInventory, ResourceType};

/// All colony building types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingType {
    Habitat,
    BasicGreenhouse,
    AdvancedGreenhouse,
    SmallSolarFarm,
    MediumSolarFarm,
    LargeSolarFarm,
    FissionReactor,
    FusionReactor,
    Mine,
    AtmosphericCollector,
    Factory,
    Launchpad,
    Railgun,
    LightConstructionRobot,
    ConstructionRobot,
    ScienceLab,
    Stockpile,
    FoodStorage,
    ParticleAcceleratorMk1,
    ParticleAcceleratorMk2,
    ParticleAcceleratorMk3,
    ParticleAcceleratorMk4,
}

impl BuildingType {
    /// All building types.
    pub fn all() -> &'static [BuildingType] {
        &[
            Self::Habitat,
            Self::BasicGreenhouse,
            Self::AdvancedGreenhouse,
            Self::SmallSolarFarm,
            Self::MediumSolarFarm,
            Self::LargeSolarFarm,
            Self::FissionReactor,
            Self::FusionReactor,
            Self::Mine,
            Self::AtmosphericCollector,
            Self::Factory,
            Self::Launchpad,
            Self::Railgun,
            Self::LightConstructionRobot,
            Self::ConstructionRobot,
            Self::ScienceLab,
            Self::Stockpile,
            Self::FoodStorage,
            Self::ParticleAcceleratorMk1,
            Self::ParticleAcceleratorMk2,
            Self::ParticleAcceleratorMk3,
            Self::ParticleAcceleratorMk4,
        ]
    }

    /// Look up a BuildingType by its display name.
    pub fn from_display_name(name: &str) -> Option<BuildingType> {
        Self::all().iter().find(|bt| bt.display_name() == name).copied()
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Habitat => "Habitat",
            Self::BasicGreenhouse => "Basic Greenhouse",
            Self::AdvancedGreenhouse => "Advanced Greenhouse",
            Self::SmallSolarFarm => "Small Solar Farm",
            Self::MediumSolarFarm => "Medium Solar Farm",
            Self::LargeSolarFarm => "Large Solar Farm",
            Self::FissionReactor => "Fission Reactor",
            Self::FusionReactor => "Fusion Reactor",
            Self::Mine => "Mine",
            Self::AtmosphericCollector => "Atmospheric Collector",
            Self::Factory => "Factory",
            Self::Launchpad => "Launchpad",
            Self::Railgun => "Railgun",
            Self::LightConstructionRobot => "Light Construction Robot",
            Self::ConstructionRobot => "Construction Robot",
            Self::ScienceLab => "Science Lab",
            Self::Stockpile => "Stockpile",
            Self::FoodStorage => "Food Storage",
            Self::ParticleAcceleratorMk1 => "Particle Accelerator Mk I",
            Self::ParticleAcceleratorMk2 => "Particle Accelerator Mk II",
            Self::ParticleAcceleratorMk3 => "Particle Accelerator Mk III",
            Self::ParticleAcceleratorMk4 => "Particle Accelerator Mk IV",
        }
    }

    /// Base build cost in kg: Vec of (ResourceType, amount).
    /// For Habitat and greenhouses, these are pre-multiplier costs.
    pub fn build_cost(&self) -> Vec<(ResourceType, f64)> {
        use ResourceType::*;
        match self {
            Self::Habitat => vec![
                (StructuralMetal, 8_000.0),
                (Electronics, 1_000.0),
            ],
            Self::BasicGreenhouse => vec![
                (StructuralMetal, 5_000.0),
                (Electronics, 3_000.0),
            ],
            Self::AdvancedGreenhouse => vec![
                (StructuralMetal, 5_000.0),
                (Electronics, 3_000.0),
            ],
            Self::SmallSolarFarm => vec![
                (StructuralMetal, 10_000.0),
                (Electronics, 5_000.0),
            ],
            Self::MediumSolarFarm => vec![
                (StructuralMetal, 100_000.0),
                (Electronics, 50_000.0),
            ],
            Self::LargeSolarFarm => vec![
                (StructuralMetal, 1_000_000.0),
                (Electronics, 500_000.0),
            ],
            Self::FissionReactor => vec![
                (StructuralMetal, 200_000.0),
                (HighTempAlloys, 100_000.0),
                (Electronics, 100_000.0),
                (Superconductors, 50_000.0),
            ],
            Self::FusionReactor => vec![
                (StructuralMetal, 500_000.0),
                (HighTempAlloys, 200_000.0),
                (Electronics, 300_000.0),
                (Superconductors, 400_000.0),
                (PrecisionInstruments, 200.0),
            ],
            Self::Mine => vec![
                (StructuralMetal, 20_000.0),
                (HighTempAlloys, 5_000.0),
                (Electronics, 5_000.0),
            ],
            Self::AtmosphericCollector => vec![
                (StructuralMetal, 15_000.0),
                (HighTempAlloys, 3_000.0),
                (Electronics, 8_000.0),
            ],
            Self::Factory => vec![
                (StructuralMetal, 50_000.0),
                (HighTempAlloys, 10_000.0),
                (Electronics, 30_000.0),
            ],
            Self::Launchpad => vec![
                (StructuralMetal, 30_000.0),
                (HighTempAlloys, 5_000.0),
                (Electronics, 5_000.0),
            ],
            Self::Railgun => vec![
                (StructuralMetal, 200_000.0),
                (HighTempAlloys, 40_000.0),
                (Electronics, 60_000.0),
                (Superconductors, 40_000.0),
            ],
            Self::LightConstructionRobot => vec![
                (StructuralMetal, 3_000.0),
                (HighTempAlloys, 1_500.0),
                (Electronics, 5_000.0),
            ],
            Self::ConstructionRobot => vec![
                (StructuralMetal, 10_000.0),
                (HighTempAlloys, 5_000.0),
                (Electronics, 15_000.0),
            ],
            Self::ScienceLab => vec![
                (StructuralMetal, 30_000.0),
                (HighTempAlloys, 5_000.0),
                (Electronics, 20_000.0),
            ],
            Self::Stockpile => vec![
                (StructuralMetal, 20_000.0),
            ],
            Self::FoodStorage => vec![
                (StructuralMetal, 5_000.0),
                (Electronics, 500.0),
            ],
            Self::ParticleAcceleratorMk1 => vec![
                (StructuralMetal, 1_500_000.0),
                (HighTempAlloys, 500_000.0),
                (Electronics, 1_000_000.0),
                (Superconductors, 2_000_000.0),
                (PrecisionInstruments, 500.0),
            ],
            Self::ParticleAcceleratorMk2 => vec![
                (StructuralMetal, 15_000_000.0),
                (HighTempAlloys, 5_000_000.0),
                (Electronics, 10_000_000.0),
                (Superconductors, 20_000_000.0),
                (PrecisionInstruments, 5_000.0),
            ],
            Self::ParticleAcceleratorMk3 => vec![
                (StructuralMetal, 150_000_000.0),
                (HighTempAlloys, 50_000_000.0),
                (Electronics, 100_000_000.0),
                (Superconductors, 200_000_000.0),
                (PrecisionInstruments, 50_000.0),
            ],
            // Mk IV costs are per-km and scale with body circumference.
            // Base values per km stored here; simulation multiplies by circumference.
            Self::ParticleAcceleratorMk4 => vec![
                (StructuralMetal, 150_000.0),
                (HighTempAlloys, 50_000.0),
                (Electronics, 100_000.0),
                (Superconductors, 200_000.0),
                (PrecisionInstruments, 50.0),
            ],
        }
    }

    /// Power draw in kW. Factory power varies by recipe and is not included here.
    pub fn power_draw_kw(&self) -> f64 {
        match self {
            Self::Habitat => 10.0,
            Self::BasicGreenhouse => 50.0,
            Self::AdvancedGreenhouse => 50.0,
            Self::SmallSolarFarm => 0.0,
            Self::MediumSolarFarm => 0.0,
            Self::LargeSolarFarm => 0.0,
            Self::FissionReactor => 0.0,
            Self::FusionReactor => 0.0,
            Self::Mine => 100.0,
            Self::AtmosphericCollector => 100.0,
            Self::Factory => 0.0, // Varies by recipe
            Self::Launchpad => 10.0,
            Self::Railgun => 10_000.0, // 10 MW
            Self::LightConstructionRobot => 100.0,
            Self::ConstructionRobot => 500.0,
            Self::ScienceLab => 200.0,
            Self::Stockpile => 0.0,
            Self::FoodStorage => 5.0,
            Self::ParticleAcceleratorMk1 => 50_000_000.0, // 50 GW
            Self::ParticleAcceleratorMk2 => 500_000_000.0, // 500 GW
            Self::ParticleAcceleratorMk3 => 5_000_000_000.0, // 5 TW
            Self::ParticleAcceleratorMk4 => 50_000.0, // 50 GW per km (scaled by circumference)
        }
    }

    /// Power output in kW. Solar farms output at 1 AU; actual output scales with distance.
    pub fn power_output_kw(&self) -> f64 {
        match self {
            Self::SmallSolarFarm => 10_000.0, // 10 MW @ 1 AU
            Self::MediumSolarFarm => 100_000.0, // 100 MW @ 1 AU
            Self::LargeSolarFarm => 1_000_000.0, // 1 GW @ 1 AU
            Self::FissionReactor => 500_000.0, // 500 MW
            Self::FusionReactor => 5_000_000.0, // 5 GW
            _ => 0.0,
        }
    }

    /// Maintenance cost per 30 days in kg: Vec of (ResourceType, amount).
    /// For Habitat and greenhouses, these are pre-multiplier costs.
    pub fn maintenance_cost_per_30d(&self) -> Vec<(ResourceType, f64)> {
        use ResourceType::*;
        match self {
            Self::Habitat => vec![
                (StructuralMetal, 40.0),
                (Electronics, 8.0),
            ],
            Self::BasicGreenhouse => vec![
                (StructuralMetal, 25.0),
                (Electronics, 13.0),
            ],
            Self::AdvancedGreenhouse => vec![
                (StructuralMetal, 25.0),
                (Electronics, 13.0),
            ],
            Self::SmallSolarFarm => vec![
                (StructuralMetal, 13.0),
                (Electronics, 5.0),
            ],
            Self::MediumSolarFarm => vec![
                (StructuralMetal, 125.0),
                (Electronics, 50.0),
            ],
            Self::LargeSolarFarm => vec![
                (StructuralMetal, 1_250.0),
                (Electronics, 500.0),
            ],
            Self::FissionReactor => vec![
                (StructuralMetal, 750.0),
                (HighTempAlloys, 250.0),
            ],
            Self::FusionReactor => vec![
                (StructuralMetal, 1_250.0),
                (HighTempAlloys, 500.0),
                (Superconductors, 250.0),
                (PrecisionInstruments, 2.0),
            ],
            Self::Mine => vec![
                (StructuralMetal, 75.0),
                (HighTempAlloys, 13.0),
            ],
            Self::AtmosphericCollector => vec![
                (StructuralMetal, 50.0),
                (Electronics, 13.0),
            ],
            Self::Factory => vec![
                (StructuralMetal, 125.0),
                (HighTempAlloys, 25.0),
                (Electronics, 25.0),
            ],
            Self::Launchpad => vec![
                (StructuralMetal, 125.0),
                (HighTempAlloys, 25.0),
            ],
            Self::Railgun => vec![
                (StructuralMetal, 500.0),
                (HighTempAlloys, 125.0),
                (Superconductors, 125.0),
            ],
            Self::LightConstructionRobot => vec![
                (StructuralMetal, 15.0),
                (HighTempAlloys, 8.0),
                (Electronics, 15.0),
            ],
            Self::ConstructionRobot => vec![
                (StructuralMetal, 50.0),
                (HighTempAlloys, 25.0),
                (Electronics, 50.0),
            ],
            Self::ScienceLab => vec![
                (StructuralMetal, 100.0),
                (Electronics, 25.0),
            ],
            Self::Stockpile => vec![],
            Self::FoodStorage => vec![
                (StructuralMetal, 10.0),
            ],
            Self::ParticleAcceleratorMk1 => vec![
                (StructuralMetal, 12_500.0),
                (HighTempAlloys, 5_000.0),
                (Electronics, 7_500.0),
                (Superconductors, 12_500.0),
                (PrecisionInstruments, 5.0),
            ],
            Self::ParticleAcceleratorMk2 => vec![
                (StructuralMetal, 125_000.0),
                (HighTempAlloys, 50_000.0),
                (Electronics, 75_000.0),
                (Superconductors, 125_000.0),
                (PrecisionInstruments, 50.0),
            ],
            Self::ParticleAcceleratorMk3 => vec![
                (StructuralMetal, 1_250_000.0),
                (HighTempAlloys, 500_000.0),
                (Electronics, 750_000.0),
                (Superconductors, 1_250_000.0),
                (PrecisionInstruments, 500.0),
            ],
            // Per-km maintenance; scaled by body circumference in simulation
            Self::ParticleAcceleratorMk4 => vec![
                (StructuralMetal, 250.0),
                (HighTempAlloys, 75.0),
                (Electronics, 125.0),
                (Superconductors, 250.0),
                (PrecisionInstruments, 0.05),
            ],
        }
    }

    /// Total build mass in kg (sum of build_cost amounts + pre-stocked supplies).
    /// Habitat includes 1,000 kg of pre-stocked food.
    pub fn total_build_mass(&self) -> f64 {
        let material_mass: f64 = self.build_cost().iter().map(|(_, amt)| amt).sum();
        match self {
            Self::Habitat => material_mass + 1_000.0,
            _ => material_mass,
        }
    }

    /// Whether this building's costs/power/maintenance are affected by the habitability multiplier.
    /// Only Habitats and Greenhouses are affected.
    pub fn affected_by_habitability(&self) -> bool {
        matches!(
            self,
            Self::Habitat | Self::BasicGreenhouse | Self::AdvancedGreenhouse
        )
    }
}

/// Factory recipes that can be assigned to a Factory building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactoryRecipe {
    MetalSmelting,
    AlloyForging,
    ElectronicsManufacturing,
    SuperconductorFabrication,
    PrecisionInstrumentsManufacturing,
    Electrolysis,
    DeuteriumExtraction,
    SabatierReaction,
    MethanePurification,
    KeroseneRefining,
    UraniumEnrichment,
    TritiumBreeding,
    NpuAssembly,
    RegolithHe3Extraction,
    GasGiantHe3Separation,
}

impl FactoryRecipe {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::MetalSmelting => "Metal Smelting",
            Self::AlloyForging => "Alloy Forging",
            Self::ElectronicsManufacturing => "Electronics Manufacturing",
            Self::SuperconductorFabrication => "Superconductor Fabrication",
            Self::PrecisionInstrumentsManufacturing => "Precision Instruments Mfg",
            Self::Electrolysis => "Electrolysis",
            Self::DeuteriumExtraction => "Deuterium Extraction",
            Self::SabatierReaction => "Sabatier Reaction",
            Self::MethanePurification => "Methane Purification",
            Self::KeroseneRefining => "Kerosene Refining",
            Self::UraniumEnrichment => "Uranium Enrichment",
            Self::TritiumBreeding => "Tritium Breeding",
            Self::NpuAssembly => "NPU Assembly",
            Self::RegolithHe3Extraction => "Regolith He-3 Extraction",
            Self::GasGiantHe3Separation => "Gas Giant He-3 Separation",
        }
    }

    /// Inputs consumed per batch: Vec of (ResourceType, kg).
    pub fn inputs(&self) -> Vec<(ResourceType, f64)> {
        use ResourceType::*;
        match self {
            Self::MetalSmelting => vec![(MetalOre, 1_000.0)],
            Self::AlloyForging => vec![(MetalOre, 200.0), (StructuralMetal, 40.0)],
            Self::ElectronicsManufacturing => vec![
                (StructuralMetal, 10.0),
                (HighTempAlloys, 2.0),
                (RareEarthElements, 1.0),
            ],
            Self::SuperconductorFabrication => vec![
                (StructuralMetal, 6.0),
                (HighTempAlloys, 2.0),
                (Electronics, 1.0),
            ],
            Self::PrecisionInstrumentsManufacturing => vec![
                (Electronics, 50.0),
                (Superconductors, 10.0),
                (HighTempAlloys, 5.0),
            ],
            Self::Electrolysis => vec![(Water, 200.0)],
            Self::DeuteriumExtraction => vec![(Water, 10_000.0)],
            Self::SabatierReaction => vec![(AtmosphericCo2, 88.0), (LiquidHydrogen, 16.0)],
            Self::MethanePurification => vec![(Hydrocarbons, 200.0)],
            Self::KeroseneRefining => vec![(Hydrocarbons, 400.0)],
            Self::UraniumEnrichment => vec![(UraniumOre, 100.0)],
            Self::TritiumBreeding => vec![(LithiumOre, 20.0)],
            Self::NpuAssembly => vec![
                (StructuralMetal, 100.0),
                (HighTempAlloys, 40.0),
                (Tritium, 0.5),
            ],
            Self::RegolithHe3Extraction => vec![(Regolith, 20_000.0)],
            Self::GasGiantHe3Separation => vec![(GasGiantAtmosphere, 10_000.0)],
        }
    }

    /// Outputs produced per batch: Vec of (ResourceType, kg).
    pub fn outputs(&self) -> Vec<(ResourceType, f64)> {
        use ResourceType::*;
        match self {
            Self::MetalSmelting => vec![(StructuralMetal, 200.0)],
            Self::AlloyForging => vec![(HighTempAlloys, 30.0)],
            Self::ElectronicsManufacturing => vec![(Electronics, 8.0)],
            Self::SuperconductorFabrication => vec![(Superconductors, 2.0)],
            Self::PrecisionInstrumentsManufacturing => vec![(PrecisionInstruments, 1.0)],
            Self::Electrolysis => vec![(LiquidHydrogen, 22.0), (Lox, 178.0)],
            Self::DeuteriumExtraction => vec![
                (Deuterium, 2.0),
                (LiquidHydrogen, 1_098.0),
                (Lox, 8_900.0),
            ],
            Self::SabatierReaction => vec![(Methane, 32.0), (Water, 72.0)],
            Self::MethanePurification => vec![(Methane, 180.0)],
            Self::KeroseneRefining => vec![(Rp1, 100.0)],
            Self::UraniumEnrichment => vec![(EnrichedUranium, 1.0)],
            Self::TritiumBreeding => vec![(Tritium, 1.0)],
            Self::NpuAssembly => vec![(NuclearPulseUnits, 50.0)],
            Self::RegolithHe3Extraction => vec![(Helium3, 0.2)],
            Self::GasGiantHe3Separation => vec![(Helium3, 1.0)],
        }
    }

    /// Power draw in kW while this recipe is running.
    pub fn power_draw_kw(&self) -> f64 {
        match self {
            Self::MetalSmelting => 150.0,
            Self::AlloyForging => 250.0,
            Self::ElectronicsManufacturing => 300.0,
            Self::SuperconductorFabrication => 500.0,
            Self::PrecisionInstrumentsManufacturing => 500.0,
            Self::Electrolysis => 50.0,
            Self::DeuteriumExtraction => 100.0,
            Self::SabatierReaction => 75.0,
            Self::MethanePurification => 30.0,
            Self::KeroseneRefining => 75.0,
            Self::UraniumEnrichment => 500.0,
            Self::TritiumBreeding => 200.0,
            Self::NpuAssembly => 500.0,
            Self::RegolithHe3Extraction => 500.0,
            Self::GasGiantHe3Separation => 200.0,
        }
    }

    /// Batch time in hours.
    pub fn batch_time_hours(&self) -> f64 {
        match self {
            Self::MetalSmelting => 12.0,
            Self::AlloyForging => 24.0,
            Self::ElectronicsManufacturing => 24.0,
            Self::SuperconductorFabrication => 48.0,
            Self::PrecisionInstrumentsManufacturing => 120.0,
            Self::Electrolysis => 8.0,
            Self::DeuteriumExtraction => 120.0,
            Self::SabatierReaction => 12.0,
            Self::MethanePurification => 4.0,
            Self::KeroseneRefining => 12.0,
            Self::UraniumEnrichment => 48.0,
            Self::TritiumBreeding => 48.0,
            Self::NpuAssembly => 240.0,
            Self::RegolithHe3Extraction => 24.0,
            Self::GasGiantHe3Separation => 24.0,
        }
    }

    /// If Some, this recipe requires the given building type to be co-located on the colony.
    pub fn requires_colocation(&self) -> Option<BuildingType> {
        match self {
            Self::TritiumBreeding => Some(BuildingType::FissionReactor),
            Self::NpuAssembly => Some(BuildingType::FissionReactor),
            _ => None,
        }
    }
}

/// A single building instance within a colony.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildingInstance {
    pub building_type: BuildingType,
    /// For mines: which resource to extract. For atmospheric collectors: which atmospheric resource.
    pub assigned_resource: Option<ResourceType>,
    /// For factories: which recipe to run.
    pub assigned_recipe: Option<FactoryRecipe>,
    /// Whether the building is currently operational (powered and maintained).
    pub operational: bool,
    /// Degradation level (0.0 = pristine, 1.0 = fully degraded/non-functional).
    pub degradation: f64,
    /// Water fill level for greenhouses (kg). Scales food output linearly.
    pub water_fill: f64,
}

impl Default for BuildingInstance {
    fn default() -> Self {
        Self {
            building_type: BuildingType::Habitat,
            assigned_resource: None,
            assigned_recipe: None,
            operational: true,
            degradation: 0.0,
            water_fill: 0.0,
        }
    }
}

impl BuildingInstance {
    pub fn new(building_type: BuildingType) -> Self {
        Self {
            building_type,
            assigned_resource: None,
            assigned_recipe: None,
            operational: true,
            degradation: 0.0,
            water_fill: 0.0,
        }
    }
}

/// An item in the colony construction queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConstructionQueueItem {
    pub building_type: BuildingType,
    /// Resources that have been reserved from colony inventory for this construction.
    pub reserved_resources: ResourceInventory,
    /// Mass assembled so far (kg).
    pub mass_assembled: f64,
    /// Total mass to assemble (kg).
    pub total_mass: f64,
}

impl Default for ConstructionQueueItem {
    fn default() -> Self {
        Self {
            building_type: BuildingType::Habitat,
            reserved_resources: ResourceInventory::default(),
            mass_assembled: 0.0,
            total_mass: 0.0,
        }
    }
}

/// A colony on a celestial body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Colony {
    /// Index of the body this colony is on.
    pub body_index: usize,
    pub name: String,
    pub buildings: Vec<BuildingInstance>,
    pub resources: ResourceInventory,
    pub crew: u32,
    pub food_stored: f64,
    pub power_generated: f64,
    pub power_consumed: f64,
    pub construction_queue: Vec<ConstructionQueueItem>,
    /// Whether this is an orbital station (gas giant scooping stations).
    pub is_orbital_station: bool,
    /// Cumulative science extracted by labs on this body.
    pub lab_science_extracted: f64,
    /// Cumulative time (years) that labs have been running on this colony.
    #[serde(default)]
    pub lab_elapsed_years: f64,
    /// Whether a food-depleted notification has already been sent (prevents spam).
    #[serde(default)]
    pub food_depleted_notified: bool,
    /// Fraction of habitat power demand that is met (0.0–1.0).
    #[serde(default = "default_one")]
    pub habitat_power_fraction: f64,
    /// Fraction of non-habitat building power demand that is met (0.0–1.0).
    #[serde(default = "default_one")]
    pub other_power_fraction: f64,
    /// Whether a habitat-unpowered notification has been sent (dedup flag).
    #[serde(default)]
    pub habitat_unpowered_notified: bool,
    /// Crew count when a food/power crisis began (for linear death rate).
    #[serde(default)]
    pub crew_at_crisis_start: Option<u32>,
    /// Fractional death accumulator — deaths only applied when this reaches >= 1.0.
    #[serde(default)]
    pub crew_death_accumulator: f64,
}

fn default_one() -> f64 {
    1.0
}

impl Default for Colony {
    fn default() -> Self {
        Self {
            body_index: 0,
            name: String::new(),
            buildings: Vec::new(),
            resources: ResourceInventory::default(),
            crew: 0,
            food_stored: 0.0,
            power_generated: 0.0,
            power_consumed: 0.0,
            construction_queue: Vec::new(),
            is_orbital_station: false,
            lab_science_extracted: 0.0,
            lab_elapsed_years: 0.0,
            food_depleted_notified: false,
            habitat_power_fraction: 1.0,
            other_power_fraction: 1.0,
            habitat_unpowered_notified: false,
            crew_at_crisis_start: None,
            crew_death_accumulator: 0.0,
        }
    }
}

impl Colony {
    pub fn new(body_index: usize, name: String) -> Self {
        Self {
            body_index,
            name,
            buildings: Vec::new(),
            resources: ResourceInventory::new(),
            crew: 0,
            food_stored: 0.0,
            power_generated: 0.0,
            power_consumed: 0.0,
            construction_queue: Vec::new(),
            is_orbital_station: false,
            lab_science_extracted: 0.0,
            lab_elapsed_years: 0.0,
            food_depleted_notified: false,
            habitat_power_fraction: 1.0,
            other_power_fraction: 1.0,
            habitat_unpowered_notified: false,
            crew_at_crisis_start: None,
            crew_death_accumulator: 0.0,
        }
    }

    /// Check whether the colony can queue a building (has enough resources).
    pub fn can_queue_building(&self, bt: BuildingType, hab_score: u32) -> bool {
        let costs = bt.build_cost();
        let mult = if bt.affected_by_habitability() {
            crate::colony::simulation::habitability_multiplier(hab_score)
        } else {
            1.0
        };
        costs.iter().all(|&(res, amount)| {
            self.resources.get(res) >= amount * mult
        })
    }

    /// Queue a building for construction, consuming resources from inventory.
    pub fn queue_building(&mut self, bt: BuildingType, hab_score: u32) -> Result<(), String> {
        let costs = bt.build_cost();
        let mult = if bt.affected_by_habitability() {
            crate::colony::simulation::habitability_multiplier(hab_score)
        } else {
            1.0
        };

        // Check resources
        for &(res, amount) in &costs {
            let needed = amount * mult;
            if self.resources.get(res) < needed {
                return Err(format!("Not enough {}", res.display_name()));
            }
        }

        // Consume resources
        let mut reserved = ResourceInventory::new();
        for &(res, amount) in &costs {
            let needed = amount * mult;
            self.resources.remove(res, needed);
            reserved.add(res, needed);
        }

        let total_mass = bt.total_build_mass() * mult;

        self.construction_queue.push(ConstructionQueueItem {
            building_type: bt,
            reserved_resources: reserved,
            mass_assembled: 0.0,
            total_mass,
        });

        Ok(())
    }

    /// Days of food remaining at current crew level.
    pub fn food_days_remaining(&self) -> f64 {
        if self.crew == 0 {
            return f64::INFINITY;
        }
        self.food_stored / (0.5 * self.crew as f64)
    }
}

/// Manages all colonies in the game.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColonyManager {
    pub colonies: Vec<Colony>,
}

impl ColonyManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_by_body(&self, body_index: usize) -> Option<&Colony> {
        self.colonies.iter().find(|c| c.body_index == body_index)
    }

    pub fn get_by_body_mut(&mut self, body_index: usize) -> Option<&mut Colony> {
        self.colonies.iter_mut().find(|c| c.body_index == body_index)
    }

    pub fn has_colony(&self, body_index: usize) -> bool {
        self.colonies.iter().any(|c| c.body_index == body_index)
    }

    pub fn add_colony(&mut self, colony: Colony) {
        self.colonies.push(colony);
    }
}
