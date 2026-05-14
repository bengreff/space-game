//! Generate a save file configured for testing mass driver & Dyson swarm features.
//! Run with: cargo test --test generate_mass_driver_save -- --nocapture
//!
//! Mercury colony — fully self-sustaining industrial base with mirror + collector production.
//! Full tech tree unlocked (all nodes, all efficiency lines at tier 15).

mod common;

use std::collections::{HashMap, HashSet};

use sunscatter_app::colony::{
    BuildingInstance, BuildingType, Colony, ColonyManager,
    DeployingMirror, FactoryRecipe, Company, ResourceInventory, ResourceType,
    ScienceState, DysonSwarm, FleetManager, ContractManager,
};
use sunscatter_app::save::{SaveGame, SavedVessel};
use sunscatter_app::ship::Ship;

fn make_building(bt: BuildingType) -> BuildingInstance {
    BuildingInstance {
        building_type: bt,
        assigned_resource: None,
        assigned_recipe: None,
        operational: true,
        degradation: 0.0,
        water_fill: 0.0,
    }
}

fn make_building_degraded(bt: BuildingType, degradation: f64) -> BuildingInstance {
    BuildingInstance {
        building_type: bt,
        assigned_resource: None,
        assigned_recipe: None,
        operational: true,
        degradation,
        water_fill: 0.0,
    }
}

fn make_mine(resource: ResourceType) -> BuildingInstance {
    BuildingInstance {
        building_type: BuildingType::Mine,
        assigned_resource: Some(resource),
        assigned_recipe: None,
        operational: true,
        degradation: 0.0,
        water_fill: 0.0,
    }
}

fn make_factory(recipe: FactoryRecipe) -> BuildingInstance {
    BuildingInstance {
        building_type: BuildingType::Factory,
        assigned_resource: None,
        assigned_recipe: Some(recipe),
        operational: true,
        degradation: 0.0,
        water_fill: 0.0,
    }
}

fn make_greenhouse(water_fill: f64) -> BuildingInstance {
    BuildingInstance {
        building_type: BuildingType::AdvancedGreenhouse,
        assigned_resource: None,
        assigned_recipe: None,
        operational: true,
        degradation: 0.0,
        water_fill,
    }
}

#[test]
fn generate_mass_driver_test_save() {
    let mercury_idx = 2;
    let moon_idx = 5;

    // =========================================================================
    // Mercury Colony — primary mass driver base
    // =========================================================================
    // Habitability score 8 → hab_mult 1.92. life_support T2 → 1.23×.
    // Effective hab multiplier: 1.92/1.23 = 1.56× on Habitat/Greenhouse maintenance.
    // Mineable: MetalOre, Regolith, RareEarthElements, UraniumOre (NO Water).
    // Solar at 0.387 AU: SmallSolarFarm outputs 10 MW × 6.68 = 66.8 MW.
    let mut mercury = Colony::new(mercury_idx, "Mercury Base".to_string());

    // --- Habitats: 2 × 20 = 40 crew ---
    for _ in 0..2 {
        mercury.buildings.push(make_building(BuildingType::Habitat));
    }
    mercury.crew = 40;

    // --- Food: 40 × 0.5 = 20 kg/day ---
    // AdvGH: 2.5 × agriculture T3 (1.37×) = 3.42 kg/day each
    // 7 × 3.42 = 23.9 > 20 ✓
    // Mercury has no water — pre-fill (water is permanent, not consumed)
    for _ in 0..7 {
        mercury.buildings.push(make_greenhouse(5_000.0));
    }
    mercury.buildings.push(make_building(BuildingType::FoodStorage));
    mercury.food_stored = 16_000.0; // 2×3000 (hab) + 10000 (storage)

    // --- Power: 2 SmallSolarFarm = 134 MW @ Mercury ---
    // Demand: ~70 MW (148 factories + 18 mines + mass driver + support)
    for _ in 0..2 {
        mercury.buildings.push(make_building(BuildingType::SmallSolarFarm));
    }

    // --- Mines ---
    // MetalOre: feeds smelting + forging. 17 mines = 46,499/day > 43,763 consumed.
    for _ in 0..17 {
        mercury.buildings.push(make_mine(ResourceType::MetalOre));
    }
    // RareEarth: feeds ElectronicsMfg. 1 mine = 2,735/day >> 67 consumed.
    mercury.buildings.push(make_mine(ResourceType::RareEarthElements));

    // --- Factories (148 total) ---
    // MetalSmelting: 200 SM/batch, 12h, metallurgy T3 → 547 SM/factory/day
    for _ in 0..13 {
        mercury.buildings.push(make_factory(FactoryRecipe::MetalSmelting));
    }
    // AlloyForging: 30 HTA/batch, 24h, metallurgy T3 → 41 HTA/factory/day
    for _ in 0..30 {
        mercury.buildings.push(make_factory(FactoryRecipe::AlloyForging));
    }
    // ElectronicsMfg: 8 Elec/batch, 24h, electronics T8 → 18.4 Elec/factory/day
    for _ in 0..29 {
        mercury.buildings.push(make_factory(FactoryRecipe::ElectronicsManufacturing));
    }
    // SuperconductorFab: 2 Super/batch, 48h, electronics T8 → 2.3 Super/factory/day
    // 74 fabs = 171/day > 168 consumed (mirror assembly + mass driver maintenance)
    for _ in 0..74 {
        mercury.buildings.push(make_factory(FactoryRecipe::SuperconductorFabrication));
    }
    // MirrorSegmentAssembly
    for _ in 0..2 {
        mercury.buildings.push(make_factory(FactoryRecipe::MirrorSegmentAssembly));
    }
    // CollectorStationAssembly
    mercury.buildings.push(make_factory(FactoryRecipe::CollectorStationAssembly));
    // PrecisionInstruments — feeds collector station recipe
    mercury.buildings.push(make_factory(FactoryRecipe::PrecisionInstrumentsManufacturing));

    // --- Mass Driver Mk III ---
    mercury.buildings.push(make_building(BuildingType::MassDriverMk3));

    // --- Receiver Arrays (20 total) ---
    // With 50 mirrors + 2 collectors at sail tier 15, laser ≈ 500 GW.
    // 20 receivers × 50 GW = 1 TW capacity > 500 GW laser → undersaturated.
    // Actual power ≈ 500 × 0.9 = 450 GW (not 20 × 45 GW = 900 GW hardcoded).
    for _ in 0..17 {
        mercury.buildings.push(make_building(BuildingType::ReceiverArray));
    }
    // 2 degraded receivers (30% and 50%) to test degradation affecting capacity
    mercury.buildings.push(make_building_degraded(BuildingType::ReceiverArray, 0.30));
    mercury.buildings.push(make_building_degraded(BuildingType::ReceiverArray, 0.50));

    // --- Support ---
    for _ in 0..2 {
        mercury.buildings.push(make_building(BuildingType::ConstructionRobot));
    }
    for _ in 0..2 {
        mercury.buildings.push(make_building(BuildingType::Stockpile));
    }
    mercury.buildings.push(make_building(BuildingType::Launchpad));

    // --- Resources: colony is self-sustaining, stockpiles are buffer ---
    let mut mercury_res = ResourceInventory::new();
    mercury_res.add(ResourceType::MetalOre, 500_000.0);
    mercury_res.add(ResourceType::RareEarthElements, 50_000.0);
    mercury_res.add(ResourceType::Regolith, 50_000.0);
    mercury_res.add(ResourceType::StructuralMetal, 200_000.0);
    mercury_res.add(ResourceType::HighTempAlloys, 50_000.0);
    mercury_res.add(ResourceType::Electronics, 50_000.0);
    mercury_res.add(ResourceType::Superconductors, 50_000.0);
    mercury_res.add(ResourceType::PrecisionInstruments, 5_000.0);
    mercury_res.add(ResourceType::MirrorSegment, 50.0);
    mercury_res.add(ResourceType::CollectorStation, 10.0);
    mercury_res.add(ResourceType::Food, 10_000.0);
    mercury.resources = mercury_res;

    // =========================================================================
    // Moon Colony — secondary base
    // =========================================================================
    // Habitability score 15 → hab_mult 1.85. life_support T2 → eff 1.50×.
    // Mineable: MetalOre, Regolith, Water, RareEarthElements, UraniumOre.
    // Solar at ~1 AU: SmallSolarFarm = 10 MW.
    let mut moon = Colony::new(moon_idx, "Lunar Station".to_string());

    // --- Habitats: 2 × 20 = 40 capacity, 30 crew ---
    for _ in 0..2 {
        moon.buildings.push(make_building(BuildingType::Habitat));
    }
    moon.crew = 30;

    // --- Food: 30 × 0.5 = 15 kg/day ---
    // 5 AdvGH × 3.42 = 17.1 > 15 ✓
    for _ in 0..5 {
        moon.buildings.push(make_greenhouse(5_000.0));
    }
    moon.buildings.push(make_building(BuildingType::FoodStorage));
    moon.food_stored = 16_000.0;

    // --- Power: 1 SmallSolarFarm = 10 MW @ Moon ---
    // Demand: ~2.7 MW
    moon.buildings.push(make_building(BuildingType::SmallSolarFarm));

    // --- Mines ---
    // MetalOre: feeds smelting + alloy. 4 mines = 10,944/day > 8,482 consumed.
    for _ in 0..4 {
        moon.buildings.push(make_mine(ResourceType::MetalOre));
    }
    moon.buildings.push(make_mine(ResourceType::RareEarthElements));
    moon.buildings.push(make_mine(ResourceType::Water));

    // --- Factories ---
    // MetalSmelting: 3 factories → 1,642 Struct/day (maintenance needs ~57)
    for _ in 0..3 {
        moon.buildings.push(make_factory(FactoryRecipe::MetalSmelting));
    }
    // AlloyForging: 1 factory → 41 HTA/day (maintenance needs ~9)
    moon.buildings.push(make_factory(FactoryRecipe::AlloyForging));
    // ElectronicsMfg: 2 factories → 37 Elec/day (maintenance needs ~11)
    for _ in 0..2 {
        moon.buildings.push(make_factory(FactoryRecipe::ElectronicsManufacturing));
    }

    // --- Receiver Arrays (3 total, receiver-saturated: 150 GW cap < 500 GW laser) ---
    // Electricity = 150 × 0.9 = 135 GW = 3 × 45 GW (matches hardcoded, contrast with Mercury)
    for _ in 0..3 {
        moon.buildings.push(make_building(BuildingType::ReceiverArray));
    }

    // --- Support ---
    moon.buildings.push(make_building(BuildingType::ConstructionRobot));
    moon.buildings.push(make_building(BuildingType::Stockpile));
    moon.buildings.push(make_building(BuildingType::Launchpad));

    // --- Resources ---
    let mut moon_res = ResourceInventory::new();
    moon_res.add(ResourceType::MetalOre, 100_000.0);
    moon_res.add(ResourceType::RareEarthElements, 50_000.0);
    moon_res.add(ResourceType::Water, 50_000.0);
    moon_res.add(ResourceType::StructuralMetal, 50_000.0);
    moon_res.add(ResourceType::HighTempAlloys, 20_000.0);
    moon_res.add(ResourceType::Electronics, 20_000.0);
    moon_res.add(ResourceType::Food, 5_000.0);
    moon.resources = moon_res;

    // =========================================================================
    // Colony Manager
    // =========================================================================
    let mut colony_manager = ColonyManager::default();
    colony_manager.colonies.push(mercury);
    colony_manager.colonies.push(moon);

    // =========================================================================
    // Tech
    // =========================================================================
    let tech_unlocked: HashSet<String> = [
        // Era 1
        "basic_rocketry", "structural_engineering", "kerolox_propulsion",
        "methalox_propulsion", "hydrolox_propulsion", "crewed_spaceflight",
        // Era 2
        "medium_launch", "medium_hydrolox", "advanced_life_support",
        "heavy_lift", "large_cryogenic", "colony_engineering",
        // Era 3
        "nuclear_thermal", "advanced_ntr", "ion_propulsion", "advanced_electric",
        "compact_fission", "heavy_fission", "super_heavy", "extended_missions",
        // Era 4
        "mpd_propulsion", "deep_space_hab", "science_laboratory",
        // Era 5
        "nuclear_pulse", "interstellar_fission", "passive_shielding",
        // Era 6
        "mass_driver_tech", "fusion_probe", "fusion_full", "fusion_power",
        "active_shielding",
        // Era 7
        "advanced_fusion", "geodesic_shielding", "swarm_power",
        // Era 8
        "am_catalyzed", "am_production", "am_power", "heavy_mass_driver",
        // Era 9
        "am_torch", "advanced_am_power", "ring_accelerator",
        "planetary_mass_driver", "photon_drive",
    ].iter().map(|s| s.to_string()).collect();

    let mut tech_line_tiers: HashMap<String, u32> = HashMap::new();
    tech_line_tiers.insert("mining".to_string(), 15);
    tech_line_tiers.insert("metallurgy".to_string(), 15);
    tech_line_tiers.insert("construction".to_string(), 15);
    tech_line_tiers.insert("agriculture".to_string(), 15);
    tech_line_tiers.insert("electronics_mfg".to_string(), 15);
    tech_line_tiers.insert("life_support".to_string(), 15);
    tech_line_tiers.insert("chemical_processing".to_string(), 15);
    tech_line_tiers.insert("sail_technology".to_string(), 15);
    tech_line_tiers.insert("atmospheric_science".to_string(), 15);
    tech_line_tiers.insert("isotope_extraction".to_string(), 15);
    tech_line_tiers.insert("precision_mfg".to_string(), 15);
    tech_line_tiers.insert("nuclear_engineering".to_string(), 15);
    tech_line_tiers.insert("swarm_power_delivery".to_string(), 15);

    // =========================================================================
    // Science & Company
    // =========================================================================
    let science = ScienceState {
        available: 5_000.0,
        cumulative_discovery: 15_000.0,
        cumulative_rd: 20_000.0,
        cumulative_lab: 5_000.0,
        ..Default::default()
    };

    let company = Company {
        money: 500_000_000.0,
        rd_budget: 10_000_000.0,
    };

    // =========================================================================
    // Vessel
    // =========================================================================
    let ss = common::make_solar_system();
    let ship = Ship::spawn_on_earth(&ss);

    let vessel = SavedVessel {
        id: 1,
        name: "Test Probe".to_string(),
        ship,
        vessel: None,
        maneuver_nodes: Vec::new(),
        is_debris: false,
    };

    // =========================================================================
    // Dyson Swarm — 50 mirrors, 2 collectors, some in transit
    // =========================================================================
    // At sail tier 15: β ≈ 1.75, η ≈ 63.6%.
    // 50 mirrors → ~6.8 TW intercepted → ~6.5 TW reflected → ~4.1 TW collected.
    // 2 collectors → 1 TW cap → limited to 1 TW → laser = 500 GW.
    // Mercury 20 receivers (985 GW cap with degradation) > 500 GW → laser-limited.
    //   Actual power = 500 × 0.9 = 450 GW ≠ old hardcoded 20 × 45 = 900 GW.
    // Moon 3 receivers (150 GW cap) < 500 GW → receiver-limited.
    //   Actual power = 150 × 0.9 = 135 GW = 3 × 45 GW (matches old code).
    let sim_time = 86400.0 * 365.0;
    let swarm = DysonSwarm {
        mirror_count: 50,
        deploying: vec![
            DeployingMirror { arrival_time: sim_time + 3600.0 },
            DeployingMirror { arrival_time: sim_time + 7200.0 },
            DeployingMirror { arrival_time: sim_time + 40000.0 },
            DeployingMirror { arrival_time: sim_time + 60000.0 },
            DeployingMirror { arrival_time: sim_time + 80000.0 },
        ],
        collector_count: 2,
        deploying_collectors: vec![
            DeployingMirror { arrival_time: sim_time + 50000.0 },
            DeployingMirror { arrival_time: sim_time + 120000.0 },
        ],
    };

    let sun_index = 1; // Index 0 = Sgr A*, index 1 = Sun
    let mut dyson_swarms = std::collections::HashMap::new();
    dyson_swarms.insert(sun_index, swarm);

    // =========================================================================
    // Assemble & Write
    // =========================================================================
    let save = SaveGame {
        version: 1,
        name: "Mass Driver Test".to_string(),
        simulation_time: sim_time,
        vessels: vec![vessel],
        next_vessel_id: 2,
        debris_counter: 0,
        blueprints: Vec::new(),
        editor_vessel_name: String::new(),
        colonies: colony_manager,
        company,
        science,
        tech_unlocked,
        tech_line_tiers,
        notifications: Vec::new(),
        contracts: ContractManager::default(),
        fleet: FleetManager::default(),
        dyson_swarm: DysonSwarm::default(),
        dyson_swarms,
        editor_blueprint: None,
    };

    save.write_to_file().expect("Failed to write test save file");
    println!("Save written to data/saves/Mass_Driver_Test/save.ron");
}
