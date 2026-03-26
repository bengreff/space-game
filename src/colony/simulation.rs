use std::collections::HashMap;

use crate::bodies::SolarSystem;
use crate::colony::notification::{Notification, NotificationKind};
use crate::colony::tech::TechTree;
use super::buildings::{BuildingType, Colony};

/// 1 AU in meters, for solar power distance scaling.
const AU: f64 = 1.496e11;

/// Habitability multiplier: score 100 = 1x, score 0 = 2x.
/// Formula: (200 - score) / 100.
pub fn habitability_multiplier(score: u32) -> f64 {
    (200.0 - score as f64) / 100.0
}

/// Storage capacity: 500,000 kg per operational Stockpile.
impl Colony {
    pub fn storage_capacity(&self) -> f64 {
        let stockpile_count = self
            .buildings
            .iter()
            .filter(|b| b.building_type == BuildingType::Stockpile && b.operational)
            .count();
        stockpile_count as f64 * 500_000.0
    }

    /// Food storage capacity in kg.
    /// Habitat: 3,000 kg per operational building.
    /// FoodStorage: 10,000 kg per operational building.
    pub fn food_capacity(&self) -> f64 {
        let mut cap = 0.0;
        for b in &self.buildings {
            if !b.operational {
                continue;
            }
            match b.building_type {
                BuildingType::Habitat => cap += 3_000.0,
                BuildingType::FoodStorage => cap += 10_000.0,
                _ => {}
            }
        }
        cap
    }

    /// Crew capacity: 20 per operational Habitat.
    pub fn crew_capacity(&self) -> u32 {
        self.operational_building_count(BuildingType::Habitat) as u32 * 20
    }

    pub fn operational_building_count(&self, bt: BuildingType) -> usize {
        self.buildings
            .iter()
            .filter(|b| b.building_type == bt && b.operational)
            .count()
    }
}

/// Get the Sun distance for a body (walk parent chain to find the body orbiting the Sun).
pub fn sun_distance(body_index: usize, solar_system: &SolarSystem) -> f64 {
    let sun_idx = solar_system.sun_index;
    let mut idx = body_index;
    // Walk up until we find a body whose parent is the Sun
    loop {
        let body = &solar_system.bodies[idx];
        if let Some(parent) = body.parent {
            if parent == sun_idx {
                // This body orbits the Sun directly — use its SMA
                return body
                    .orbit
                    .map(|o| o.semi_major_axis)
                    .unwrap_or(AU);
            }
            idx = parent;
        } else {
            // Reached root without finding Sun (shouldn't happen for colonizable bodies)
            return AU;
        }
    }
}

/// Run one simulation tick for a colony.
pub fn simulate_colony_tick(
    colony: &mut Colony,
    days: f64,
    solar_system: &SolarSystem,
    notifications: &mut Vec<Notification>,
    sim_time: f64,
    _tech_tree: &TechTree,
) {
    let hab_score = solar_system.bodies[colony.body_index].habitability_score;
    let hab_mult = habitability_multiplier(hab_score);
    let dist = sun_distance(colony.body_index, solar_system);

    // === 0. Reactor fuel consumption ===
    // Consume fuel before power calculation. Reactors without fuel don't generate.
    // Track which building indices have fuel available.
    let mut reactor_has_fuel = vec![true; colony.buildings.len()];
    for (i, b) in colony.buildings.iter().enumerate() {
        if !b.operational {
            continue;
        }
        match b.building_type {
            BuildingType::FissionReactor => {
                // 0.5 kg Enriched Uranium per day
                let needed = 0.5 * days;
                let available = colony.resources.get(super::resources::ResourceType::EnrichedUranium);
                if available >= needed {
                    reactor_has_fuel[i] = true;
                } else {
                    reactor_has_fuel[i] = false;
                }
            }
            BuildingType::FusionReactor => {
                // 3 kg He-3 + 2 kg Deuterium per day
                let he3_needed = 3.0 * days;
                let d_needed = 2.0 * days;
                let he3_avail = colony.resources.get(super::resources::ResourceType::Helium3);
                let d_avail = colony.resources.get(super::resources::ResourceType::Deuterium);
                if he3_avail >= he3_needed && d_avail >= d_needed {
                    reactor_has_fuel[i] = true;
                } else {
                    reactor_has_fuel[i] = false;
                }
            }
            _ => {}
        }
    }
    // Actually consume fuel for fueled reactors
    for (i, b) in colony.buildings.iter().enumerate() {
        if !b.operational || !reactor_has_fuel[i] {
            continue;
        }
        match b.building_type {
            BuildingType::FissionReactor => {
                colony.resources.remove(super::resources::ResourceType::EnrichedUranium, 0.5 * days);
            }
            BuildingType::FusionReactor => {
                colony.resources.remove(super::resources::ResourceType::Helium3, 3.0 * days);
                colony.resources.remove(super::resources::ResourceType::Deuterium, 2.0 * days);
            }
            _ => {}
        }
    }

    // === 1. Power balance (habitat-priority allocation) ===
    let mut total_generation = 0.0_f64;
    let mut habitat_demand = 0.0_f64;
    let mut other_demand = 0.0_f64;

    for (i, b) in colony.buildings.iter().enumerate() {
        if !b.operational {
            continue;
        }
        // Generation
        let output = b.building_type.power_output_kw();
        if output > 0.0 {
            let is_solar = matches!(
                b.building_type,
                BuildingType::SmallSolarFarm
                    | BuildingType::MediumSolarFarm
                    | BuildingType::LargeSolarFarm
            );
            if is_solar {
                total_generation += output * (AU / dist).powi(2) * (1.0 - b.degradation);
            } else if reactor_has_fuel[i] {
                total_generation += output * (1.0 - b.degradation);
            }
            // If reactor has no fuel, it contributes 0 power
        }
        // Demand — split habitat vs other
        let draw = b.building_type.power_draw_kw();
        if b.building_type == BuildingType::Habitat {
            habitat_demand += draw;
        } else {
            other_demand += draw;
        }
        // Factory recipe power goes to "other"
        if b.building_type == BuildingType::Factory {
            if let Some(recipe) = b.assigned_recipe {
                other_demand += recipe.power_draw_kw();
            }
        }
    }

    // Habitats get power first, then everything else gets the remainder
    colony.habitat_power_fraction = if habitat_demand == 0.0 {
        1.0
    } else {
        (total_generation / habitat_demand).min(1.0)
    };
    let power_after_habitats = (total_generation - habitat_demand).max(0.0);
    colony.other_power_fraction = if other_demand == 0.0 {
        1.0
    } else {
        (power_after_habitats / other_demand).min(1.0)
    };

    colony.power_generated = total_generation;
    colony.power_consumed = habitat_demand + other_demand;

    // Habitat power loss notification
    if colony.habitat_power_fraction < 1.0 && colony.crew > 0 && !colony.habitat_unpowered_notified
    {
        colony.habitat_unpowered_notified = true;
        notifications.push(Notification {
            kind: NotificationKind::ColonyPowerLoss {
                colony_name: colony.name.clone(),
            },
            time: sim_time,
            read: false,
        });
    }
    if colony.habitat_power_fraction >= 1.0 {
        colony.habitat_unpowered_notified = false;
    }

    // === 2. Maintenance ===
    process_maintenance(colony, days, hab_mult);

    // === 3. Construction ===
    process_construction(colony, days, hab_mult, notifications, sim_time);

    // === 4. Mines ===
    let storage_cap = colony.storage_capacity();
    let current_mass = colony.resources.total_mass();
    let mut available_storage = (storage_cap - current_mass).max(0.0);

    for b in &mut colony.buildings {
        if !b.operational || b.building_type != BuildingType::Mine {
            continue;
        }
        if let Some(resource) = b.assigned_resource {
            let production = 2000.0 * days * (1.0 - b.degradation) * colony.other_power_fraction;
            let capped = production.min(available_storage);
            if capped > 0.0 {
                colony.resources.add(resource, capped);
                available_storage -= capped;
            }
        }
    }

    // === 4b. Atmospheric Collectors ===
    for b in &mut colony.buildings {
        if !b.operational || b.building_type != BuildingType::AtmosphericCollector {
            continue;
        }
        if let Some(resource) = b.assigned_resource {
            let production =
                10_000.0 * days * (1.0 - b.degradation) * colony.other_power_fraction;
            let capped = production.min(available_storage);
            if capped > 0.0 {
                colony.resources.add(resource, capped);
                available_storage -= capped;
            }
        }
    }

    // === 5. Factories ===
    for i in 0..colony.buildings.len() {
        let b = &colony.buildings[i];
        if !b.operational || b.building_type != BuildingType::Factory {
            continue;
        }
        let recipe = match b.assigned_recipe {
            Some(r) => r,
            None => continue,
        };
        let degradation = b.degradation;

        // Check colocation requirement
        if let Some(required) = recipe.requires_colocation() {
            if colony.operational_building_count(required) == 0 {
                continue;
            }
        }

        let outputs = recipe.outputs();
        let inputs = recipe.inputs();
        let batch_hours = recipe.batch_time_hours();

        // Throughput: how many batches worth of output per day
        let batches_per_day = 24.0 / batch_hours;
        let throughput_factor = batches_per_day * days * (1.0 - degradation) * colony.other_power_fraction;

        // Check if we have enough inputs
        let mut can_produce = true;
        for &(ref res, amount) in &inputs {
            let needed = amount * throughput_factor;
            if colony.resources.get(*res) < needed {
                can_produce = false;
                break;
            }
        }

        if !can_produce {
            continue;
        }

        // Check storage for outputs
        let output_mass: f64 = outputs.iter().map(|(_, amt)| amt * throughput_factor).sum();
        let current_total = colony.resources.total_mass();
        if current_total + output_mass > storage_cap {
            continue;
        }

        // Consume inputs
        for &(ref res, amount) in &inputs {
            colony.resources.remove(*res, amount * throughput_factor);
        }

        // Produce outputs
        for &(ref res, amount) in &outputs {
            colony.resources.add(*res, amount * throughput_factor);
        }
    }

    // === 6. Greenhouses ===
    for b in &mut colony.buildings {
        if !b.operational {
            continue;
        }
        match b.building_type {
            BuildingType::BasicGreenhouse => {
                let max_water = 2_000.0; // kg
                let rate = 0.5 * days * (b.water_fill / max_water).min(1.0) * (1.0 - b.degradation) * colony.other_power_fraction;
                colony.food_stored += rate;
            }
            BuildingType::AdvancedGreenhouse => {
                let max_water = 5_000.0; // kg
                let rate = 2.5 * days * (b.water_fill / max_water).min(1.0) * (1.0 - b.degradation) * colony.other_power_fraction;
                colony.food_stored += rate;
            }
            _ => {}
        }
    }

    // === 6b. Cap food at capacity ===
    let food_cap = colony.food_capacity();
    if food_cap > 0.0 && colony.food_stored > food_cap {
        colony.food_stored = food_cap;
    }

    // === 7. Food consumption ===
    let food_consumed = 0.5 * colony.crew as f64 * days;
    colony.food_stored -= food_consumed;
    if colony.food_stored <= 0.0 {
        colony.food_stored = 0.0;
        if colony.crew > 0 && !colony.food_depleted_notified {
            colony.food_depleted_notified = true;
            notifications.push(Notification {
                kind: NotificationKind::ColonyFoodDepleted {
                    colony_name: colony.name.clone(),
                },
                time: sim_time,
                read: false,
            });
        }
    } else {
        colony.food_depleted_notified = false;
    }

    // === 7b. Crew death from food/power crisis ===
    let crisis = (colony.food_stored <= 0.0 && colony.crew > 0)
        || (colony.habitat_power_fraction < 1.0 && colony.crew > 0);

    if crisis {
        if colony.crew_at_crisis_start.is_none() {
            colony.crew_at_crisis_start = Some(colony.crew);
        }
        let base_crew = colony.crew_at_crisis_start.unwrap_or(colony.crew) as f64;
        let deaths_per_day = base_crew * 0.005; // 1% per 2 days
        colony.crew_death_accumulator += deaths_per_day * days;
        let whole_deaths = colony.crew_death_accumulator.floor() as u32;
        if whole_deaths > 0 {
            colony.crew_death_accumulator -= whole_deaths as f64;
            colony.crew = colony.crew.saturating_sub(whole_deaths);
        }
    } else {
        colony.crew_at_crisis_start = None;
        colony.crew_death_accumulator = 0.0;
    }

    // === 8. Science labs ===
    process_science_labs(colony, days, solar_system);
}

/// Process maintenance for all buildings.
fn process_maintenance(colony: &mut Colony, days: f64, hab_mult: f64) {
    // Calculate robot maintenance capacity
    let mut robot_maintenance_capacity = 0.0_f64;
    for b in &colony.buildings {
        if !b.operational {
            continue;
        }
        match b.building_type {
            BuildingType::ConstructionRobot => robot_maintenance_capacity += 60_000.0 * days,
            BuildingType::LightConstructionRobot => robot_maintenance_capacity += 15_000.0 * days,
            _ => {}
        }
    }

    // Calculate total maintenance demand
    let mut total_maintenance_mass = 0.0_f64;
    for b in &colony.buildings {
        let costs = b.building_type.maintenance_cost_per_30d();
        let mult = if b.building_type.affected_by_habitability() {
            hab_mult
        } else {
            1.0
        };
        let mass: f64 = costs.iter().map(|(_, amt)| amt * mult).sum();
        total_maintenance_mass += mass / 30.0 * days;
    }

    let robot_shortfall = if robot_maintenance_capacity > 0.0 && total_maintenance_mass > robot_maintenance_capacity {
        1.0 - robot_maintenance_capacity / total_maintenance_mass
    } else {
        0.0
    };

    // Process per-building maintenance
    for b in &mut colony.buildings {
        let costs = b.building_type.maintenance_cost_per_30d();
        if costs.is_empty() {
            continue;
        }

        let mult = if b.building_type.affected_by_habitability() {
            hab_mult
        } else {
            1.0
        };

        let mut resource_shortfall = 0.0_f64;
        for &(ref res, amount) in &costs {
            let needed = amount * mult / 30.0 * days;
            let available = colony.resources.get(*res);
            if available >= needed {
                colony.resources.remove(*res, needed);
            } else {
                colony.resources.remove(*res, available);
                resource_shortfall = resource_shortfall.max(1.0 - available / needed);
            }
        }

        let shortfall = resource_shortfall.max(robot_shortfall);
        if shortfall > 0.0 {
            b.degradation = (b.degradation + shortfall * days / 30.0).min(1.0);
        }
    }

    // Sync degradation within each building type to their average
    let mut type_deg: HashMap<BuildingType, (f64, u32)> = HashMap::new();
    for b in colony.buildings.iter() {
        let entry = type_deg.entry(b.building_type).or_insert((0.0, 0));
        entry.0 += b.degradation;
        entry.1 += 1;
    }
    for b in colony.buildings.iter_mut() {
        if let Some(&(sum, count)) = type_deg.get(&b.building_type) {
            if count > 1 {
                b.degradation = sum / count as f64;
            }
        }
    }
}

/// Process construction queue.
fn process_construction(
    colony: &mut Colony,
    days: f64,
    hab_mult: f64,
    notifications: &mut Vec<Notification>,
    sim_time: f64,
) {
    // Calculate robot construction capacity (after maintenance)
    let mut robot_construction_capacity = 0.0_f64;
    for b in &colony.buildings {
        if !b.operational {
            continue;
        }
        match b.building_type {
            BuildingType::ConstructionRobot => robot_construction_capacity += 20_000.0 * days,
            BuildingType::LightConstructionRobot => robot_construction_capacity += 5_000.0 * days,
            _ => {}
        }
    }

    // Subtract maintenance demand from robot capacity
    let mut maintenance_demand = 0.0_f64;
    for b in &colony.buildings {
        let costs = b.building_type.maintenance_cost_per_30d();
        let mult = if b.building_type.affected_by_habitability() {
            hab_mult
        } else {
            1.0
        };
        let mass: f64 = costs.iter().map(|(_, amt)| amt * mult).sum();
        maintenance_demand += mass / 30.0 * days;
    }

    let remaining_capacity = (robot_construction_capacity - maintenance_demand).max(0.0);
    if remaining_capacity <= 0.0 || colony.construction_queue.is_empty() {
        return;
    }

    // Apply remaining capacity to first queue item
    let item = &mut colony.construction_queue[0];
    item.mass_assembled += remaining_capacity;

    if item.mass_assembled >= item.total_mass {
        let target = item.effective_target();
        colony.construction_queue.remove(0);

        match target {
            super::buildings::ConstructionTarget::Building(building_type) => {
                // Add the new building
                colony.buildings.push(super::buildings::BuildingInstance::new(building_type));

                // Pre-stock food when a Habitat completes
                if building_type == BuildingType::Habitat {
                    colony.food_stored += 1_000.0;
                }

                notifications.push(Notification {
                    kind: NotificationKind::ConstructionComplete {
                        colony_name: colony.name.clone(),
                        building: building_type.display_name().to_string(),
                    },
                    time: sim_time,
                    read: false,
                });
            }
            super::buildings::ConstructionTarget::Ship {
                name,
                blueprint_name,
                blueprint,
                dry_mass_kg,
                cached_delta_v,
            } => {
                // Create a StoredShip and add to colony hangar
                let stored = super::trade::StoredShip {
                    id: colony.next_stored_ship_id,
                    name: name.clone(),
                    blueprint_name: Some(blueprint_name),
                    blueprint,
                    dry_mass_kg,
                    cached_delta_v,
                };
                colony.next_stored_ship_id += 1;
                // Best-effort store: if hangar is full, just push it anyway
                // (the resources were already consumed)
                colony.stored_ships.push(stored);

                notifications.push(Notification {
                    kind: NotificationKind::ShipConstructionComplete {
                        ship_name: name,
                        location: colony.name.clone(),
                    },
                    time: sim_time,
                    read: false,
                });
            }
        }
    }
}

/// Process science labs.
fn process_science_labs(colony: &mut Colony, days: f64, solar_system: &SolarSystem) {
    let lab_count = colony.operational_building_count(BuildingType::ScienceLab);
    if lab_count == 0 {
        return;
    }

    let body = &solar_system.bodies[colony.body_index];
    let lv = body.landing_science_value();
    let n = lab_count as f64 * colony.other_power_fraction;

    // Advance lab elapsed time
    colony.lab_elapsed_years += days / 365.0;

    // extracted(t) = 10 * lv * (1 - e^(-N * t / 15))
    let new_extracted = 10.0 * lv * (1.0 - (-n * colony.lab_elapsed_years / 15.0).exp());
    colony.lab_science_extracted = new_extracted;
}
