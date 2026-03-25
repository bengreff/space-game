use serde::{Deserialize, Serialize};

/// Types of contracts available to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractType {
    SuborbitalPayload,
    OrbitalPayload,
    LunarOrbitPayload,
    LunarSurfaceDelivery,
    MarsOrbitPayload,
    MarsSurfaceDelivery,
    SuborbitalTourism,
    OrbitalTourism,
    LunarTourism,
}

impl ContractType {
    pub fn all() -> &'static [ContractType] {
        &[
            ContractType::SuborbitalPayload,
            ContractType::OrbitalPayload,
            ContractType::LunarOrbitPayload,
            ContractType::LunarSurfaceDelivery,
            ContractType::MarsOrbitPayload,
            ContractType::MarsSurfaceDelivery,
            ContractType::SuborbitalTourism,
            ContractType::OrbitalTourism,
            ContractType::LunarTourism,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ContractType::SuborbitalPayload => "Suborbital Payload",
            ContractType::OrbitalPayload => "Orbital Payload",
            ContractType::LunarOrbitPayload => "Lunar Orbit Payload",
            ContractType::LunarSurfaceDelivery => "Lunar Surface Delivery",
            ContractType::MarsOrbitPayload => "Mars Orbit Payload",
            ContractType::MarsSurfaceDelivery => "Mars Surface Delivery",
            ContractType::SuborbitalTourism => "Suborbital Tourism",
            ContractType::OrbitalTourism => "Orbital Tourism",
            ContractType::LunarTourism => "Lunar Tourism",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ContractType::SuborbitalPayload => "Reach 100 km altitude above Earth",
            ContractType::OrbitalPayload => "Achieve stable orbit around Earth",
            ContractType::LunarOrbitPayload => "Achieve orbit around the Moon",
            ContractType::LunarSurfaceDelivery => "Land on the lunar surface",
            ContractType::MarsOrbitPayload => "Achieve orbit around Mars",
            ContractType::MarsSurfaceDelivery => "Land on the Martian surface",
            ContractType::SuborbitalTourism => "Take a tourist on a suborbital flight (100+ km)",
            ContractType::OrbitalTourism => "Take a tourist to Earth orbit",
            ContractType::LunarTourism => "Take a tourist to lunar orbit",
        }
    }

    pub fn payout(&self) -> f64 {
        match self {
            ContractType::SuborbitalPayload => 500_000.0,
            ContractType::OrbitalPayload => 2_000_000.0,
            ContractType::LunarOrbitPayload => 4_000_000.0,
            ContractType::LunarSurfaceDelivery => 6_000_000.0,
            ContractType::MarsOrbitPayload => 8_000_000.0,
            ContractType::MarsSurfaceDelivery => 12_000_000.0,
            ContractType::SuborbitalTourism => 1_000_000.0,
            ContractType::OrbitalTourism => 3_000_000.0,
            ContractType::LunarTourism => 10_000_000.0,
        }
    }

    /// Whether this contract requires crew (tourism contracts).
    pub fn requires_crew(&self) -> bool {
        matches!(
            self,
            ContractType::SuborbitalTourism
                | ContractType::OrbitalTourism
                | ContractType::LunarTourism
        )
    }
}

/// A single active contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub contract_type: ContractType,
    /// Whether the altitude/orbit requirement has been met.
    pub objective_met: bool,
}

/// Manages all active contracts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractManager {
    pub active: Vec<Contract>,
    #[serde(default)]
    pub show_board: bool,
}

impl ContractManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accept a new contract.
    pub fn accept(&mut self, contract_type: ContractType) {
        // Don't accept duplicates
        if self.active.iter().any(|c| c.contract_type == contract_type) {
            return;
        }
        self.active.push(Contract {
            contract_type,
            objective_met: false,
        });
    }

    /// Check and update contract completion based on current game state.
    /// Returns total payout earned.
    pub fn check_completion(
        &mut self,
        ship_state: &crate::ship::ShipState,
        ship_soi_body: usize,
        ship_altitude: f64,
        is_suborbital: bool,
        earth_index: usize,
        moon_index: usize,
        mars_index: usize,
        has_crew: bool,
    ) -> f64 {
        let mut payout = 0.0;

        for contract in &mut self.active {
            if contract.objective_met {
                continue;
            }

            let met = match contract.contract_type {
                ContractType::SuborbitalPayload => {
                    ship_soi_body == earth_index
                        && ship_altitude > 100_000.0
                        && matches!(ship_state, crate::ship::ShipState::Flying)
                }
                ContractType::OrbitalPayload => {
                    ship_soi_body == earth_index
                        && !is_suborbital
                        && matches!(ship_state, crate::ship::ShipState::Flying)
                }
                ContractType::LunarOrbitPayload => {
                    ship_soi_body == moon_index
                        && !is_suborbital
                        && matches!(ship_state, crate::ship::ShipState::Flying)
                }
                ContractType::LunarSurfaceDelivery => {
                    matches!(ship_state, crate::ship::ShipState::Landed { body_index, .. } if *body_index == moon_index)
                }
                ContractType::MarsOrbitPayload => {
                    ship_soi_body == mars_index
                        && !is_suborbital
                        && matches!(ship_state, crate::ship::ShipState::Flying)
                }
                ContractType::MarsSurfaceDelivery => {
                    matches!(ship_state, crate::ship::ShipState::Landed { body_index, .. } if *body_index == mars_index)
                }
                ContractType::SuborbitalTourism => {
                    has_crew
                        && ship_soi_body == earth_index
                        && ship_altitude > 100_000.0
                        && matches!(ship_state, crate::ship::ShipState::Flying)
                }
                ContractType::OrbitalTourism => {
                    has_crew
                        && ship_soi_body == earth_index
                        && !is_suborbital
                        && matches!(ship_state, crate::ship::ShipState::Flying)
                }
                ContractType::LunarTourism => {
                    has_crew
                        && ship_soi_body == moon_index
                        && !is_suborbital
                        && matches!(ship_state, crate::ship::ShipState::Flying)
                }
            };

            if met {
                contract.objective_met = true;
                payout += contract.contract_type.payout();
            }
        }

        // Remove completed contracts
        self.active.retain(|c| !c.objective_met);

        payout
    }
}
