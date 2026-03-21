use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// All resource types in the game economy.
/// Raw resources are mined or collected; processed resources are manufactured in factories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    // Raw resources
    MetalOre,
    Regolith,
    Water,
    LithiumOre,
    Hydrocarbons,
    AtmosphericCo2,
    GasGiantAtmosphere,
    RareEarthElements,
    UraniumOre,
    // Processed resources
    StructuralMetal,
    HighTempAlloys,
    Electronics,
    Superconductors,
    PrecisionInstruments,
    // Fuels
    Rp1,
    Methane,
    LiquidHydrogen,
    Lox,
    Xenon,
    Deuterium,
    Tritium,
    EnrichedUranium,
    NuclearPulseUnits,
    Helium3,
    Antimatter,
    // Consumables
    Food,
}

impl ResourceType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::MetalOre => "Metal Ore",
            Self::Regolith => "Regolith",
            Self::Water => "Water",
            Self::LithiumOre => "Lithium Ore",
            Self::Hydrocarbons => "Hydrocarbons",
            Self::AtmosphericCo2 => "Atmospheric CO₂",
            Self::GasGiantAtmosphere => "Gas Giant Atmosphere",
            Self::RareEarthElements => "Rare Earth Elements",
            Self::UraniumOre => "Uranium Ore",
            Self::StructuralMetal => "Structural Metal",
            Self::HighTempAlloys => "High-Temp Alloys",
            Self::Electronics => "Electronics",
            Self::Superconductors => "Superconductors",
            Self::PrecisionInstruments => "Precision Instruments",
            Self::Rp1 => "RP-1",
            Self::Methane => "Methane",
            Self::LiquidHydrogen => "Liquid Hydrogen",
            Self::Lox => "LOX",
            Self::Xenon => "Xenon",
            Self::Deuterium => "Deuterium",
            Self::Tritium => "Tritium",
            Self::EnrichedUranium => "Enriched Uranium",
            Self::NuclearPulseUnits => "Nuclear Pulse Units",
            Self::Helium3 => "Helium-3",
            Self::Antimatter => "Antimatter",
            Self::Food => "Food",
        }
    }

    /// Earth purchase price in $/kg. None if the resource cannot be purchased on Earth.
    pub fn earth_price(&self) -> Option<f64> {
        match self {
            Self::MetalOre => Some(5.0),
            Self::Regolith => None,
            Self::Water => Some(2.0),
            Self::LithiumOre => Some(50.0),
            Self::Hydrocarbons => Some(1.0),
            Self::AtmosphericCo2 => None,
            Self::GasGiantAtmosphere => None,
            Self::RareEarthElements => Some(500.0),
            Self::UraniumOre => Some(100.0),
            Self::StructuralMetal => Some(100.0),
            Self::HighTempAlloys => Some(1_000.0),
            Self::Electronics => Some(10_000.0),
            Self::Superconductors => Some(50_000.0),
            Self::PrecisionInstruments => Some(200_000.0),
            Self::Rp1 => Some(1.0),
            Self::Methane => Some(2.0),
            Self::LiquidHydrogen => Some(6.0),
            Self::Lox => Some(0.50),
            Self::Xenon => Some(3_000.0),
            Self::Deuterium => Some(20.0),
            Self::Tritium => Some(30_000.0),
            Self::EnrichedUranium => Some(15_000.0),
            Self::NuclearPulseUnits => Some(100_000.0),
            Self::Helium3 => None,
            Self::Antimatter => None,
            Self::Food => Some(50.0),
        }
    }
}

/// A collection of resources tracked by mass (kg).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceInventory {
    #[serde(default)]
    resources: HashMap<ResourceType, f64>,
}

impl ResourceInventory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, resource: ResourceType) -> f64 {
        self.resources.get(&resource).copied().unwrap_or(0.0)
    }

    pub fn set(&mut self, resource: ResourceType, amount: f64) {
        if amount <= 0.0 {
            self.resources.remove(&resource);
        } else {
            self.resources.insert(resource, amount);
        }
    }

    pub fn add(&mut self, resource: ResourceType, amount: f64) {
        if amount <= 0.0 {
            return;
        }
        let entry = self.resources.entry(resource).or_insert(0.0);
        *entry += amount;
    }

    /// Remove up to `amount` kg of a resource. Returns false if insufficient.
    pub fn remove(&mut self, resource: ResourceType, amount: f64) -> bool {
        if amount <= 0.0 {
            return true;
        }
        let current = self.get(resource);
        if current < amount - 1e-9 {
            return false;
        }
        self.set(resource, (current - amount).max(0.0));
        true
    }

    pub fn has_enough(&self, resource: ResourceType, amount: f64) -> bool {
        self.get(resource) >= amount - 1e-9
    }

    pub fn has_enough_all(&self, requirements: &[(ResourceType, f64)]) -> bool {
        requirements.iter().all(|&(r, amt)| self.has_enough(r, amt))
    }

    /// Atomically remove all listed resources. Returns false (and removes nothing) if any is insufficient.
    pub fn remove_all(&mut self, requirements: &[(ResourceType, f64)]) -> bool {
        if !self.has_enough_all(requirements) {
            return false;
        }
        for &(r, amt) in requirements {
            self.remove(r, amt);
        }
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ResourceType, &f64)> {
        self.resources.iter()
    }

    pub fn total_mass(&self) -> f64 {
        self.resources.values().sum()
    }
}

/// The player's company — manages money and R&D spending.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    /// Current cash balance in dollars
    pub money: f64,
    /// Annual R&D budget in dollars/year
    pub rd_budget: f64,
}

impl Default for Company {
    fn default() -> Self {
        Self {
            money: 25_000_000.0,
            rd_budget: 1_000_000.0,
        }
    }
}
