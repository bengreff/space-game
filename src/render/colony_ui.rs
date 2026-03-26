use std::collections::{BTreeMap, HashMap};

use crate::colony::{BuildingType, Colony, ColonyManager, FactoryRecipe, FleetManager, ResourceType, StoredShipId};
use super::types::TradeAction;

// ============================================================
// Format helpers
// ============================================================

/// Format mass in kg: use "kg" for < 1000, "t" for >= 1000 kg.
fn format_colony_mass(kg: f64) -> String {
    if kg >= 1000.0 {
        format!("{:.2} t", kg / 1000.0)
    } else {
        format!("{:.1} kg", kg)
    }
}

/// Format power in kW with SI prefixes.
fn format_power_kw(kw: f64) -> String {
    if kw.abs() >= 1_000_000.0 {
        format!("{:.1} GW", kw / 1_000_000.0)
    } else if kw.abs() >= 1_000.0 {
        format!("{:.1} MW", kw / 1_000.0)
    } else {
        format!("{:.1} kW", kw)
    }
}

/// Format a production/consumption rate in kg/day with sign prefix.
fn format_rate(kg_per_day: f64) -> String {
    let abs = kg_per_day.abs();
    let sign = if kg_per_day >= 0.0 { "+" } else { "\u{2212}" };
    if abs >= 1000.0 {
        format!("{}{:.1} t/day", sign, abs / 1000.0)
    } else if abs >= 0.1 {
        format!("{}{:.1} kg/day", sign, abs)
    } else if abs > 0.001 {
        format!("{}{:.2} kg/day", sign, abs)
    } else {
        String::new()
    }
}

// ============================================================
// Color constants
// ============================================================

const COLOR_GREEN: egui::Color32 = egui::Color32::from_rgb(100, 255, 100);
const COLOR_RED: egui::Color32 = egui::Color32::from_rgb(255, 100, 100);
const COLOR_YELLOW: egui::Color32 = egui::Color32::from_rgb(220, 200, 80);
const COLOR_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 150, 100);
const COLOR_DEG_YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 180, 60);
const COLOR_GRAY: egui::Color32 = egui::Color32::from_rgb(160, 160, 160);
const CARD_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(30, 35, 50, 220);

// ============================================================
// UI helpers
// ============================================================

fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(CARD_BG)
        .inner_margin(egui::Margin::same(12.0))
        .rounding(egui::Rounding::same(6.0))
        .outer_margin(egui::Margin::symmetric(0.0, 4.0))
}

fn section_heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(14.0)
            .strong()
            .color(egui::Color32::WHITE),
    );
    ui.add_space(4.0);
}


// ============================================================
// Constants
// ============================================================

/// All building types available for construction via the UI.
const BUILDABLE_BUILDINGS: &[BuildingType] = &[
    BuildingType::Habitat,
    BuildingType::BasicGreenhouse,
    BuildingType::AdvancedGreenhouse,
    BuildingType::SmallSolarFarm,
    BuildingType::MediumSolarFarm,
    BuildingType::LargeSolarFarm,
    BuildingType::FissionReactor,
    BuildingType::Mine,
    BuildingType::Factory,
    BuildingType::LightConstructionRobot,
    BuildingType::ConstructionRobot,
    BuildingType::ScienceLab,
    BuildingType::Stockpile,
    BuildingType::FoodStorage,
    BuildingType::Hangar,
];

/// All factory recipes for the assignment UI.
const ALL_RECIPES: &[FactoryRecipe] = &[
    FactoryRecipe::MetalSmelting,
    FactoryRecipe::AlloyForging,
    FactoryRecipe::ElectronicsManufacturing,
    FactoryRecipe::SuperconductorFabrication,
    FactoryRecipe::PrecisionInstrumentsManufacturing,
    FactoryRecipe::Electrolysis,
    FactoryRecipe::DeuteriumExtraction,
    FactoryRecipe::SabatierReaction,
    FactoryRecipe::MethanePurification,
    FactoryRecipe::KeroseneRefining,
    FactoryRecipe::UraniumEnrichment,
    FactoryRecipe::TritiumBreeding,
    FactoryRecipe::NpuAssembly,
    FactoryRecipe::RegolithHe3Extraction,
    FactoryRecipe::GasGiantHe3Separation,
];

// ============================================================
// Actions
// ============================================================

/// Actions returned by the colony screen.
#[derive(Debug, Clone, PartialEq)]
pub enum ColonyScreenAction {
    None,
    QueueBuilding(usize, BuildingType),
    AddMineAssignment(usize, ResourceType, u32),
    RemoveMineAssignment(usize, ResourceType, u32),
    AddCollectorAssignment(usize, ResourceType, u32),
    RemoveCollectorAssignment(usize, ResourceType, u32),
    AddFactoryAssignment(usize, FactoryRecipe, u32),
    RemoveFactoryAssignment(usize, FactoryRecipe, u32),
    ReturnToFlight,
    GoToTrackingStation,
    GoToMainMenu,
    GoToColonyOverview,
    ChangeWarp(usize),
    SwitchColony(usize),
    DebugAddResource(usize, ResourceType, f64),
    DebugAddBuilding(usize, BuildingType),
    DebugAddCrew(usize, u32),
    ScrapShip(usize, StoredShipId),
    Trade(TradeAction),
}

// ============================================================
// Pre-computed resource rates
// ============================================================

struct ResourceRates {
    production: HashMap<ResourceType, f64>,
    consumption: HashMap<ResourceType, f64>,
}

fn compute_resource_rates(colony: &Colony, hab_mult: f64) -> ResourceRates {
    let mut production: HashMap<ResourceType, f64> = HashMap::new();
    let mut consumption: HashMap<ResourceType, f64> = HashMap::new();

    for b in &colony.buildings {
        if !b.operational {
            continue;
        }

        // Mine production
        if b.building_type == BuildingType::Mine {
            if let Some(res) = b.assigned_resource {
                let rate = 2000.0 * (1.0 - b.degradation) * colony.other_power_fraction;
                *production.entry(res).or_insert(0.0) += rate;
            }
        }

        // Atmospheric Collector production
        if b.building_type == BuildingType::AtmosphericCollector {
            if let Some(res) = b.assigned_resource {
                let rate = 10_000.0 * (1.0 - b.degradation) * colony.other_power_fraction;
                *production.entry(res).or_insert(0.0) += rate;
            }
        }

        // Factory production/consumption
        if b.building_type == BuildingType::Factory {
            if let Some(recipe) = b.assigned_recipe {
                let batches_per_day = 24.0 / recipe.batch_time_hours();
                let factor =
                    batches_per_day * (1.0 - b.degradation) * colony.other_power_fraction;
                for &(res, amt) in &recipe.outputs() {
                    *production.entry(res).or_insert(0.0) += amt * factor;
                }
                for &(res, amt) in &recipe.inputs() {
                    *consumption.entry(res).or_insert(0.0) += amt * factor;
                }
            }
        }

        // Greenhouse food production
        match b.building_type {
            BuildingType::BasicGreenhouse => {
                let max_water = 2_000.0;
                let rate = 0.5
                    * (b.water_fill / max_water).min(1.0)
                    * (1.0 - b.degradation)
                    * colony.other_power_fraction;
                *production.entry(ResourceType::Food).or_insert(0.0) += rate;
            }
            BuildingType::AdvancedGreenhouse => {
                let max_water = 5_000.0;
                let rate = 2.5
                    * (b.water_fill / max_water).min(1.0)
                    * (1.0 - b.degradation)
                    * colony.other_power_fraction;
                *production.entry(ResourceType::Food).or_insert(0.0) += rate;
            }
            _ => {}
        }
    }

    // Maintenance consumption
    for b in &colony.buildings {
        let costs = b.building_type.maintenance_cost_per_30d();
        let mult = if b.building_type.affected_by_habitability() {
            hab_mult
        } else {
            1.0
        };
        for &(res, amt) in &costs {
            *consumption.entry(res).or_insert(0.0) += amt * mult / 30.0;
        }
    }

    // Reactor fuel consumption
    for b in &colony.buildings {
        if !b.operational {
            continue;
        }
        match b.building_type {
            BuildingType::FissionReactor => {
                *consumption
                    .entry(ResourceType::EnrichedUranium)
                    .or_insert(0.0) += 0.5;
            }
            BuildingType::FusionReactor => {
                *consumption.entry(ResourceType::Helium3).or_insert(0.0) += 3.0;
                *consumption.entry(ResourceType::Deuterium).or_insert(0.0) += 2.0;
            }
            _ => {}
        }
    }

    // Food consumption by crew
    if colony.crew > 0 {
        *consumption.entry(ResourceType::Food).or_insert(0.0) += 0.5 * colony.crew as f64;
    }

    ResourceRates {
        production,
        consumption,
    }
}

// ============================================================
// Section renderers
// ============================================================

fn render_overview_card(ui: &mut egui::Ui, colony: &Colony, body_name: &str) {
    card_frame().show(ui, |ui| {
        section_heading(ui, &colony.name);
        let location = if colony.is_orbital_station {
            format!("Orbital station, {}", body_name)
        } else {
            format!("Surface, {}", body_name)
        };
        ui.label(egui::RichText::new(location).size(12.0).color(COLOR_GRAY));

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        egui::Grid::new("overview_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .show(ui, |ui| {
                // Crew
                let crew_cap = colony.crew_capacity();
                ui.label(egui::RichText::new("Crew").size(12.0));
                if crew_cap > 0 {
                    ui.label(
                        egui::RichText::new(format!("{} / {}", colony.crew, crew_cap)).size(12.0),
                    );
                } else {
                    ui.label(egui::RichText::new(format!("{}", colony.crew)).size(12.0));
                }
                ui.end_row();

                // Food
                let food_days = colony.food_days_remaining();
                let food_cap = colony.food_capacity();
                let food_value = if food_days.is_infinite() {
                    if food_cap > 0.0 {
                        format!(
                            "{} / {} (no crew)",
                            format_colony_mass(colony.food_stored),
                            format_colony_mass(food_cap)
                        )
                    } else {
                        format!("{} (no crew)", format_colony_mass(colony.food_stored))
                    }
                } else if food_cap > 0.0 {
                    format!(
                        "{} / {} ({:.1} days)",
                        format_colony_mass(colony.food_stored),
                        format_colony_mass(food_cap),
                        food_days
                    )
                } else {
                    format!(
                        "{} ({:.1} days)",
                        format_colony_mass(colony.food_stored),
                        food_days
                    )
                };
                let food_color = if food_days < 10.0 && !food_days.is_infinite() {
                    COLOR_RED
                } else if food_days < 30.0 && !food_days.is_infinite() {
                    COLOR_YELLOW
                } else {
                    egui::Color32::WHITE
                };
                ui.label(egui::RichText::new("Food").size(12.0));
                ui.label(egui::RichText::new(food_value).size(12.0).color(food_color));
                ui.end_row();

                // Storage
                let storage_used = colony.resources.total_mass();
                let storage_cap = colony.storage_capacity();
                ui.label(egui::RichText::new("Storage").size(12.0));
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {}",
                        format_colony_mass(storage_used),
                        format_colony_mass(storage_cap)
                    ))
                    .size(12.0),
                );
                ui.end_row();
            });

        // Crisis alert
        if let Some(crisis_crew) = colony.crew_at_crisis_start {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            let deaths_per_day = crisis_crew as f64 * 0.005;
            let reason = match (
                colony.food_stored <= 0.0,
                colony.habitat_power_fraction < 1.0,
            ) {
                (true, true) => "food + power shortage",
                (true, false) => "food shortage",
                (false, true) => "power shortage",
                (false, false) => "crisis",
            };
            ui.label(
                egui::RichText::new(format!(
                    "CRISIS: ~{:.1} crew/day ({})",
                    deaths_per_day, reason
                ))
                .size(12.0)
                .strong()
                .color(COLOR_RED),
            );
        }
    });
}

fn render_power_card(ui: &mut egui::Ui, colony: &Colony, solar_power_factor: f64) {
    card_frame().show(ui, |ui| {
        section_heading(ui, "Power");

        // Net power summary (most important)
        let power_surplus = colony.power_generated - colony.power_consumed;
        let net_color = if power_surplus < 0.0 {
            COLOR_RED
        } else {
            COLOR_GREEN
        };
        ui.label(
            egui::RichText::new(format!(
                "Net: {}{}",
                if power_surplus >= 0.0 { "+" } else { "" },
                format_power_kw(power_surplus),
            ))
            .size(12.0)
            .strong()
            .color(net_color),
        );

        // Allocation warnings inline
        ui.horizontal(|ui| {
            if colony.habitat_power_fraction < 1.0 {
                ui.label(
                    egui::RichText::new(format!(
                        "Habitat: {:.0}%",
                        colony.habitat_power_fraction * 100.0
                    ))
                    .size(12.0)
                    .color(COLOR_RED),
                );
                ui.label(
                    egui::RichText::new("\u{2014} CREW AT RISK")
                        .size(12.0)
                        .strong()
                        .color(COLOR_RED),
                );
            } else if colony.crew > 0 {
                ui.label(
                    egui::RichText::new(format!(
                        "Habitat: {:.0}%",
                        colony.habitat_power_fraction * 100.0
                    ))
                    .size(12.0),
                );
            }

            if colony.other_power_fraction < 1.0 {
                if colony.crew > 0 {
                    ui.label(egui::RichText::new("  |  ").size(12.0).color(COLOR_GRAY));
                }
                let color = if colony.other_power_fraction < 0.5 {
                    COLOR_RED
                } else {
                    COLOR_YELLOW
                };
                ui.label(
                    egui::RichText::new(format!(
                        "Buildings: {:.0}%",
                        colony.other_power_fraction * 100.0
                    ))
                    .size(12.0)
                    .color(color),
                );
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Combined production + demand grid
        // Power production by building type
        let mut prod_map: BTreeMap<&str, (u32, f64)> = BTreeMap::new();
        for b in &colony.buildings {
            if !b.operational {
                continue;
            }
            let output = b.building_type.power_output_kw();
            if output <= 0.0 {
                continue;
            }
            let is_solar = matches!(
                b.building_type,
                BuildingType::SmallSolarFarm
                    | BuildingType::MediumSolarFarm
                    | BuildingType::LargeSolarFarm
            );
            let actual = if is_solar {
                output * solar_power_factor * (1.0 - b.degradation)
            } else {
                output * (1.0 - b.degradation)
            };
            let entry = prod_map
                .entry(b.building_type.display_name())
                .or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += actual;
        }

        // Power demand by building type + factory recipe
        let mut demand_entries: Vec<(String, u32, f64)> = Vec::new();
        let mut demand_map: BTreeMap<String, usize> = BTreeMap::new();

        for b in &colony.buildings {
            if !b.operational {
                continue;
            }
            let draw = b.building_type.power_draw_kw();
            if draw > 0.0 {
                let key = b.building_type.display_name().to_string();
                if let Some(&idx) = demand_map.get(&key) {
                    demand_entries[idx].1 += 1;
                    demand_entries[idx].2 += draw;
                } else {
                    let idx = demand_entries.len();
                    demand_map.insert(key.clone(), idx);
                    demand_entries.push((key, 1, draw));
                }
            }
            if b.building_type == BuildingType::Factory {
                if let Some(recipe) = b.assigned_recipe {
                    let key = format!("Factory \u{2014} {}", recipe.display_name());
                    if let Some(&idx) = demand_map.get(&key) {
                        demand_entries[idx].1 += 1;
                        demand_entries[idx].2 += recipe.power_draw_kw();
                    } else {
                        let idx = demand_entries.len();
                        demand_map.insert(key.clone(), idx);
                        demand_entries.push((key, 1, recipe.power_draw_kw()));
                    }
                }
            }
        }

        let has_entries = !prod_map.is_empty() || !demand_entries.is_empty();
        if has_entries {
            egui::Grid::new("power_grid")
                .striped(true)
                .num_columns(3)
                .min_col_width(100.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Source").size(11.0).strong());
                    ui.label(egui::RichText::new("Count").size(11.0).strong());
                    ui.label(egui::RichText::new("Output").size(11.0).strong());
                    ui.end_row();

                    for (name, (count, total_kw)) in &prod_map {
                        ui.label(egui::RichText::new(*name).size(11.0));
                        ui.label(egui::RichText::new(format!("{}x", count)).size(11.0));
                        ui.label(
                            egui::RichText::new(format!("+{}", format_power_kw(*total_kw)))
                                .size(11.0)
                                .color(COLOR_GREEN),
                        );
                        ui.end_row();
                    }

                    if !prod_map.is_empty() && !demand_entries.is_empty() {
                        ui.separator();
                        ui.separator();
                        ui.separator();
                        ui.end_row();
                    }

                    for (name, count, total_kw) in &demand_entries {
                        ui.label(egui::RichText::new(name).size(11.0));
                        ui.label(egui::RichText::new(format!("{}x", count)).size(11.0));
                        ui.label(
                            egui::RichText::new(format!(
                                "\u{2212}{}",
                                format_power_kw(*total_kw)
                            ))
                            .size(11.0)
                            .color(COLOR_ORANGE),
                        );
                        ui.end_row();
                    }
                });
        } else {
            ui.label(egui::RichText::new("No power infrastructure.").size(11.0).color(COLOR_GRAY));
        }
    });
}

fn render_buildings_card(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    colony: &Colony,
    body_index: usize,
    body_mineable: &[Vec<ResourceType>],
    body_atmospheric: &[Vec<ResourceType>],
    action: &mut ColonyScreenAction,
) {
    card_frame().show(ui, |ui| {
        section_heading(ui, "Buildings");

        // Batch size selector
        let batch_id = egui::Id::new("colony_batch_size");
        let mut batch_size: u32 = ctx.data_mut(|d| *d.get_temp_mut_or(batch_id, 10u32));
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Assign:").size(11.0));
            for &(label, val) in
                &[("10", 10u32), ("100", 100), ("1000", 1000), ("All", 0)]
            {
                if ui
                    .selectable_label(batch_size == val, egui::RichText::new(label).size(11.0))
                    .clicked()
                {
                    batch_size = val;
                }
            }
        });
        ctx.data_mut(|d| d.insert_temp(batch_id, batch_size));

        if colony.buildings.is_empty() {
            ui.label(
                egui::RichText::new("No buildings.")
                    .size(12.0)
                    .color(COLOR_GRAY),
            );
            return;
        }

        ui.add_space(4.0);

        // Building counts
        let mut counts: BTreeMap<&str, (BuildingType, u32)> = BTreeMap::new();
        for b in &colony.buildings {
            let entry = counts
                .entry(b.building_type.display_name())
                .or_insert((b.building_type, 0));
            entry.1 += 1;
        }

        egui::Grid::new("building_counts_grid")
            .num_columns(2)
            .spacing([40.0, 2.0])
            .show(ui, |ui| {
                for (_name, (bt, count)) in &counts {
                    if *bt == BuildingType::Mine
                        || *bt == BuildingType::Factory
                        || *bt == BuildingType::AtmosphericCollector
                    {
                        continue;
                    }
                    ui.label(egui::RichText::new(bt.display_name()).size(11.0));
                    ui.label(egui::RichText::new(format!("{}x", count)).size(11.0));
                    ui.end_row();
                }
            });

        // Mines sub-section
        let mine_count = colony
            .buildings
            .iter()
            .filter(|b| b.building_type == BuildingType::Mine)
            .count();

        if mine_count > 0 {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("Mines ({}x)", mine_count))
                    .size(13.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(2.0);

            let mineable = body_mineable
                .get(body_index)
                .cloned()
                .unwrap_or_default();

            let mut assigned_counts: HashMap<ResourceType, u32> = HashMap::new();
            let mut unassigned = 0u32;
            for b in &colony.buildings {
                if b.building_type != BuildingType::Mine {
                    continue;
                }
                match b.assigned_resource {
                    Some(res) => {
                        *assigned_counts.entry(res).or_insert(0) += 1;
                    }
                    None => unassigned += 1,
                }
            }

            for &res in &mineable {
                let count = assigned_counts.get(&res).copied().unwrap_or(0);
                if count == 0 && unassigned == 0 {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!("{}: {}", res.display_name(), count))
                            .size(11.0),
                    );
                    if count > 0 {
                        if ui.small_button("\u{2212}").clicked() {
                            let n = if batch_size == 0 {
                                count
                            } else {
                                batch_size.min(count)
                            };
                            *action =
                                ColonyScreenAction::RemoveMineAssignment(body_index, res, n);
                        }
                    }
                    if unassigned > 0 {
                        let resp = ui.small_button("+");
                        if resp.clicked() {
                            let n = if batch_size == 0 {
                                unassigned
                            } else {
                                batch_size.min(unassigned)
                            };
                            *action =
                                ColonyScreenAction::AddMineAssignment(body_index, res, n);
                        }
                        resp.on_hover_text("Produces 2,000 kg/day");
                    }
                });
            }

            // Non-mineable resources with assignments (edge case)
            for (&res, &count) in &assigned_counts {
                if !mineable.contains(&res) && count > 0 {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new(format!("{}: {}", res.display_name(), count))
                                .size(11.0),
                        );
                        if ui.small_button("\u{2212}").clicked() {
                            let n = if batch_size == 0 {
                                count
                            } else {
                                batch_size.min(count)
                            };
                            *action =
                                ColonyScreenAction::RemoveMineAssignment(body_index, res, n);
                        }
                    });
                }
            }

            if unassigned > 0 {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!("Unassigned: {}", unassigned))
                            .size(11.0)
                            .color(COLOR_GRAY),
                    );
                });
            }
        }

        // Atmospheric Collector sub-section
        let collector_count = colony
            .buildings
            .iter()
            .filter(|b| b.building_type == BuildingType::AtmosphericCollector)
            .count();

        if collector_count > 0 {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("Atmospheric Collectors ({}x)", collector_count))
                    .size(13.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(2.0);

            let atmospheric = body_atmospheric
                .get(body_index)
                .cloned()
                .unwrap_or_default();

            let mut assigned_counts: HashMap<ResourceType, u32> = HashMap::new();
            let mut unassigned = 0u32;
            for b in &colony.buildings {
                if b.building_type != BuildingType::AtmosphericCollector {
                    continue;
                }
                match b.assigned_resource {
                    Some(res) => {
                        *assigned_counts.entry(res).or_insert(0) += 1;
                    }
                    None => unassigned += 1,
                }
            }

            for &res in &atmospheric {
                let count = assigned_counts.get(&res).copied().unwrap_or(0);
                if count == 0 && unassigned == 0 {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!("{}: {}", res.display_name(), count))
                            .size(11.0),
                    );
                    if count > 0 {
                        if ui.small_button("\u{2212}").clicked() {
                            let n = if batch_size == 0 {
                                count
                            } else {
                                batch_size.min(count)
                            };
                            *action = ColonyScreenAction::RemoveCollectorAssignment(
                                body_index, res, n,
                            );
                        }
                    }
                    if unassigned > 0 {
                        let resp = ui.small_button("+");
                        if resp.clicked() {
                            let n = if batch_size == 0 {
                                unassigned
                            } else {
                                batch_size.min(unassigned)
                            };
                            *action = ColonyScreenAction::AddCollectorAssignment(
                                body_index, res, n,
                            );
                        }
                        resp.on_hover_text("Produces 10,000 kg/day");
                    }
                });
            }

            // Non-atmospheric resources with assignments (edge case)
            for (&res, &count) in &assigned_counts {
                if !atmospheric.contains(&res) && count > 0 {
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new(format!("{}: {}", res.display_name(), count))
                                .size(11.0),
                        );
                        if ui.small_button("\u{2212}").clicked() {
                            let n = if batch_size == 0 {
                                count
                            } else {
                                batch_size.min(count)
                            };
                            *action = ColonyScreenAction::RemoveCollectorAssignment(
                                body_index, res, n,
                            );
                        }
                    });
                }
            }

            if unassigned > 0 {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!("Unassigned: {}", unassigned))
                            .size(11.0)
                            .color(COLOR_GRAY),
                    );
                });
            }
        }

        // Factory sub-section
        let factory_count = colony
            .buildings
            .iter()
            .filter(|b| b.building_type == BuildingType::Factory)
            .count();

        if factory_count > 0 {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("Factories ({}x)", factory_count))
                    .size(13.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(2.0);

            let mut recipe_counts: HashMap<FactoryRecipe, u32> = HashMap::new();
            let mut unassigned_factories = 0u32;
            for b in &colony.buildings {
                if b.building_type != BuildingType::Factory {
                    continue;
                }
                match b.assigned_recipe {
                    Some(recipe) => {
                        *recipe_counts.entry(recipe).or_insert(0) += 1;
                    }
                    None => unassigned_factories += 1,
                }
            }

            for &recipe in ALL_RECIPES {
                let count = recipe_counts.get(&recipe).copied().unwrap_or(0);
                if count == 0 && unassigned_factories == 0 {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!("{}: {}", recipe.display_name(), count))
                            .size(11.0),
                    );
                    if count > 0 {
                        if ui.small_button("\u{2212}").clicked() {
                            let n = if batch_size == 0 {
                                count
                            } else {
                                batch_size.min(count)
                            };
                            *action = ColonyScreenAction::RemoveFactoryAssignment(
                                body_index, recipe, n,
                            );
                        }
                    }
                    if unassigned_factories > 0 {
                        // Build tooltip
                        let inputs = recipe.inputs();
                        let outputs = recipe.outputs();
                        let input_str: String = inputs
                            .iter()
                            .map(|(r, a)| {
                                format!("{} {}", format_colony_mass(*a), r.display_name())
                            })
                            .collect::<Vec<_>>()
                            .join(" + ");
                        let output_str: String = outputs
                            .iter()
                            .map(|(r, a)| {
                                format!("{} {}", format_colony_mass(*a), r.display_name())
                            })
                            .collect::<Vec<_>>()
                            .join(" + ");
                        let tooltip = format!(
                            "{} \u{2192} {} ({}h, {})",
                            input_str,
                            output_str,
                            recipe.batch_time_hours() as u32,
                            format_power_kw(recipe.power_draw_kw()),
                        );
                        let resp = ui.small_button("+");
                        if resp.clicked() {
                            let n = if batch_size == 0 {
                                unassigned_factories
                            } else {
                                batch_size.min(unassigned_factories)
                            };
                            *action = ColonyScreenAction::AddFactoryAssignment(
                                body_index, recipe, n,
                            );
                        }
                        resp.on_hover_text(tooltip);
                    }
                });
            }

            if unassigned_factories > 0 {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(format!("Unassigned: {}", unassigned_factories))
                            .size(11.0)
                            .color(COLOR_GRAY),
                    );
                });
            }
        }
    });
}

fn render_construction_card(
    ui: &mut egui::Ui,
    colony: &Colony,
    body_index: usize,
    hab_score: u32,
    tech_tree: &crate::colony::TechTree,
    action: &mut ColonyScreenAction,
) {
    card_frame().show(ui, |ui| {
        section_heading(ui, "Construction");

        if colony.construction_queue.is_empty() {
            ui.label(
                egui::RichText::new("No active construction.")
                    .size(12.0)
                    .color(COLOR_GRAY),
            );
        } else {
            for item in &colony.construction_queue {
                let progress = if item.total_mass > 0.0 {
                    (item.mass_assembled / item.total_mass) as f32
                } else {
                    0.0
                };
                let display_name = match item.effective_target() {
                    crate::colony::ConstructionTarget::Building(bt) => {
                        bt.display_name().to_string()
                    }
                    crate::colony::ConstructionTarget::Ship { name, .. } => {
                        format!("Ship: {}", name)
                    }
                };
                let text = format!(
                    "{} ({} / {})",
                    display_name,
                    format_colony_mass(item.mass_assembled),
                    format_colony_mass(item.total_mass),
                );
                ui.add(
                    egui::ProgressBar::new(progress.clamp(0.0, 1.0))
                        .text(text)
                        .fill(egui::Color32::from_rgb(80, 160, 80)),
                );
            }
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Queue:").size(12.0));
            egui::ComboBox::from_id_source("cs_add_building_combo")
                .selected_text("Select building...")
                .width(200.0)
                .show_ui(ui, |ui| {
                    for &bt in BUILDABLE_BUILDINGS {
                        let tech_available = tech_tree.is_building_available(bt);
                        if !tech_available {
                            ui.add_enabled(
                                false,
                                egui::SelectableLabel::new(
                                    false,
                                    format!("{} [Locked]", bt.display_name()),
                                ),
                            );
                            continue;
                        }
                        let can_build = colony.can_queue_building(bt, hab_score);
                        let costs = bt.build_cost();
                        let cost_str: String = costs
                            .iter()
                            .map(|(r, amt)| {
                                format!("{} {}", format_colony_mass(*amt), r.display_name())
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        let label = format!("{} [{}]", bt.display_name(), cost_str);

                        if ui
                            .add_enabled(
                                can_build,
                                egui::SelectableLabel::new(false, &label),
                            )
                            .clicked()
                        {
                            *action = ColonyScreenAction::QueueBuilding(body_index, bt);
                        }
                    }
                });
        });
    });
}

fn render_maintenance_card(ui: &mut egui::Ui, colony: &Colony, hab_mult: f64) {
    card_frame().show(ui, |ui| {
        section_heading(ui, "Maintenance");

        // Aggregate maintenance costs per 30 days
        let mut total_costs: BTreeMap<&str, (ResourceType, f64)> = BTreeMap::new();
        let mut total_mass_per_30d = 0.0_f64;
        for b in &colony.buildings {
            let costs = b.building_type.maintenance_cost_per_30d();
            let mult = if b.building_type.affected_by_habitability() {
                hab_mult
            } else {
                1.0
            };
            for &(res, amt) in &costs {
                let scaled = amt * mult;
                let entry = total_costs
                    .entry(res.display_name())
                    .or_insert((res, 0.0));
                entry.1 += scaled;
                total_mass_per_30d += scaled;
            }
        }

        if total_costs.is_empty() {
            ui.label(
                egui::RichText::new("No maintenance required.")
                    .size(12.0)
                    .color(COLOR_GRAY),
            );
            return;
        }

        egui::Grid::new("cs_maintenance_grid")
            .striped(true)
            .min_col_width(100.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Resource").size(11.0).strong());
                ui.label(egui::RichText::new("Per 30d").size(11.0).strong());
                ui.label(egui::RichText::new("In stock").size(11.0).strong());
                ui.label(egui::RichText::new("Days left").size(11.0).strong());
                ui.end_row();

                for (_name, (res, cost_30d)) in &total_costs {
                    let in_stock = colony.resources.get(*res);
                    let daily_rate = cost_30d / 30.0;
                    let days_left = if daily_rate > 0.0 {
                        in_stock / daily_rate
                    } else {
                        f64::INFINITY
                    };

                    ui.label(egui::RichText::new(res.display_name()).size(11.0));
                    ui.label(egui::RichText::new(format_colony_mass(*cost_30d)).size(11.0));
                    ui.label(egui::RichText::new(format_colony_mass(in_stock)).size(11.0));

                    let days_color = if days_left < 10.0 {
                        COLOR_RED
                    } else if days_left < 30.0 {
                        COLOR_YELLOW
                    } else {
                        egui::Color32::WHITE
                    };
                    ui.label(
                        egui::RichText::new(if days_left.is_infinite() {
                            "\u{221e}".to_string()
                        } else {
                            format!("{:.0}", days_left)
                        })
                        .size(11.0)
                        .color(days_color),
                    );
                    ui.end_row();
                }
            });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        ui.label(
            egui::RichText::new(format!(
                "Total: {} / 30 days",
                format_colony_mass(total_mass_per_30d)
            ))
            .size(11.0),
        );

        // Robot capacity
        let mut robot_capacity_per_day = 0.0_f64;
        for b in &colony.buildings {
            if !b.operational {
                continue;
            }
            match b.building_type {
                BuildingType::ConstructionRobot => robot_capacity_per_day += 20_000.0,
                BuildingType::LightConstructionRobot => robot_capacity_per_day += 5_000.0,
                _ => {}
            }
        }
        let daily_demand = total_mass_per_30d / 30.0;
        let robot_color = if robot_capacity_per_day < daily_demand {
            COLOR_RED
        } else {
            egui::Color32::WHITE
        };
        ui.label(
            egui::RichText::new(format!(
                "Robot capacity: {} / day (demand: {} / day)",
                format_colony_mass(robot_capacity_per_day),
                format_colony_mass(daily_demand),
            ))
            .size(11.0)
            .color(robot_color),
        );

        // Degraded buildings
        let mut deg_counts: BTreeMap<&str, (u32, f64)> = BTreeMap::new();
        for b in &colony.buildings {
            if b.degradation > 0.001 {
                let entry = deg_counts
                    .entry(b.building_type.display_name())
                    .or_insert((0, 0.0));
                entry.0 += 1;
                entry.1 = entry.1.max(b.degradation);
            }
        }
        if !deg_counts.is_empty() {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            for (name, (count, worst)) in &deg_counts {
                let label = if *count > 1 {
                    format!(
                        "{} ({}x) \u{2014} {:.0}% degraded",
                        name,
                        count,
                        worst * 100.0
                    )
                } else {
                    format!("{} \u{2014} {:.0}% degraded", name, worst * 100.0)
                };
                ui.label(
                    egui::RichText::new(label)
                        .size(11.0)
                        .color(COLOR_DEG_YELLOW),
                );
            }
        }
    });
}

fn render_resources_card(
    ui: &mut egui::Ui,
    colony: &Colony,
    rates: &ResourceRates,
) {
    card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        section_heading(ui, "Resources");

        // Build resource list: regular resources + food
        let mut resources: Vec<(ResourceType, f64)> = colony
            .resources
            .iter()
            .filter(|(_, &amt)| amt > 0.001)
            .map(|(&rt, &amt)| (rt, amt))
            .collect();
        resources.sort_by(|a, b| a.0.display_name().cmp(b.0.display_name()));

        // Add Food row
        let has_food_in_list = resources.iter().any(|(rt, _)| *rt == ResourceType::Food);
        if !has_food_in_list && (colony.food_stored > 0.001 || colony.crew > 0) {
            resources.push((ResourceType::Food, colony.food_stored));
            resources.sort_by(|a, b| a.0.display_name().cmp(b.0.display_name()));
        }

        // Add resources that have production/consumption but zero stock
        let rate_resources: Vec<ResourceType> = rates
            .production
            .keys()
            .chain(rates.consumption.keys())
            .copied()
            .collect();
        for rt in rate_resources {
            if rt == ResourceType::Food {
                continue;
            }
            if !resources.iter().any(|(r, _)| *r == rt) {
                resources.push((rt, 0.0));
            }
        }
        resources.sort_by(|a, b| a.0.display_name().cmp(b.0.display_name()));

        if resources.is_empty() {
            ui.label(
                egui::RichText::new("No resources in storage.")
                    .size(12.0)
                    .color(COLOR_GRAY),
            );
        } else {
            egui::Grid::new("cs_resources_grid")
                .striped(true)
                .num_columns(5)
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Resource").size(11.0).strong());
                    ui.label(egui::RichText::new("Amount").size(11.0).strong());
                    ui.label(egui::RichText::new("Production").size(11.0).strong());
                    ui.label(egui::RichText::new("Consumption").size(11.0).strong());
                    ui.label(egui::RichText::new("Days left").size(11.0).strong());
                    ui.end_row();

                    for (rt, amt) in &resources {
                        let amount = if *rt == ResourceType::Food {
                            colony.food_stored
                        } else {
                            *amt
                        };
                        let prod = rates.production.get(rt).copied().unwrap_or(0.0);
                        let cons = rates.consumption.get(rt).copied().unwrap_or(0.0);
                        let net = prod - cons;

                        ui.label(egui::RichText::new(rt.display_name()).size(11.0));
                        ui.label(egui::RichText::new(format_colony_mass(amount)).size(11.0));

                        if prod > 0.001 {
                            ui.label(
                                egui::RichText::new(format_rate(prod))
                                    .size(11.0)
                                    .color(COLOR_GREEN),
                            );
                        } else {
                            ui.label("");
                        }

                        if cons > 0.001 {
                            ui.label(
                                egui::RichText::new(format_rate(-cons))
                                    .size(11.0)
                                    .color(COLOR_ORANGE),
                            );
                        } else {
                            ui.label("");
                        }

                        // Days left: only meaningful when net consumption > 0
                        if net < -0.001 && amount > 0.001 {
                            let days_left = amount / (-net);
                            let days_color = if days_left < 10.0 {
                                COLOR_RED
                            } else if days_left < 30.0 {
                                COLOR_YELLOW
                            } else {
                                egui::Color32::WHITE
                            };
                            ui.label(
                                egui::RichText::new(format!("{:.0}", days_left))
                                    .size(11.0)
                                    .color(days_color),
                            );
                        } else if net >= 0.001 {
                            // Net positive — show infinity/stable
                            ui.label(
                                egui::RichText::new("\u{221e}")
                                    .size(11.0)
                                    .color(COLOR_GREEN),
                            );
                        } else {
                            ui.label("");
                        }

                        ui.end_row();
                    }
                });
        }
    });
}

// ============================================================
// Main render function
// ============================================================

/// Render the full-screen colony management screen.
/// Returns a ColonyScreenAction describing what the user wants to do.
fn render_hangar_card(
    ui: &mut egui::Ui,
    colony: &Colony,
    body_index: usize,
    action: &mut ColonyScreenAction,
) {
    if !colony.has_hangar() {
        return;
    }

    card_frame().show(ui, |ui| {
        section_heading(ui, "Ship Hangar");

        // Capacity bar
        let used = colony.hangar_used();
        let capacity = colony.hangar_capacity();
        let frac = if capacity > 0.0 {
            (used / capacity) as f32
        } else {
            0.0
        };
        let bar_color = if frac > 0.9 {
            egui::Color32::from_rgb(200, 80, 80)
        } else {
            egui::Color32::from_rgb(80, 140, 200)
        };
        ui.add(
            egui::ProgressBar::new(frac.clamp(0.0, 1.0))
                .text(format!(
                    "{} / {}",
                    format_colony_mass(used),
                    format_colony_mass(capacity),
                ))
                .fill(bar_color),
        );

        if colony.stored_ships.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("No ships stored.")
                    .size(12.0)
                    .color(COLOR_GRAY),
            );
        } else {
            ui.add_space(4.0);
            egui::Grid::new("cs_hangar_ships_grid")
                .striped(true)
                .min_col_width(60.0)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Ship").size(11.0).strong());
                    ui.label(egui::RichText::new("Blueprint").size(11.0).strong());
                    ui.label(egui::RichText::new("Dry Mass").size(11.0).strong());
                    ui.label(egui::RichText::new("\u{0394}v").size(11.0).strong());
                    ui.label(""); // Scrap button column
                    ui.end_row();

                    for ship in &colony.stored_ships {
                        ui.label(egui::RichText::new(&ship.name).size(11.0));
                        ui.label(
                            egui::RichText::new(
                                ship.blueprint_name.as_deref().unwrap_or("\u{2014}"),
                            )
                            .size(11.0)
                            .color(COLOR_GRAY),
                        );
                        ui.label(
                            egui::RichText::new(format_colony_mass(ship.dry_mass_kg)).size(11.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0} m/s", ship.cached_delta_v))
                                .size(11.0),
                        );
                        if ui.small_button("Scrap").clicked() {
                            *action = ColonyScreenAction::ScrapShip(body_index, ship.id);
                        }
                        ui.end_row();
                    }
                });
        }
    });
}

pub fn render_colony_screen(
    ctx: &egui::Context,
    body_index: usize,
    colony_manager: &ColonyManager,
    body_names: &[String],
    body_habitability: &[u32],
    body_mineable: &[Vec<ResourceType>],
    body_atmospheric: &[Vec<ResourceType>],
    warp_levels: &[f64],
    current_warp_index: usize,
    date_str: &str,
    paused: bool,
    can_return_to_flight: bool,
    active_toasts: &[(String, std::time::Instant)],
    solar_power_factor: f64,
    tech_tree: &crate::colony::TechTree,
    fleet: &FleetManager,
    earth_index: usize,
) -> ColonyScreenAction {
    let mut action = ColonyScreenAction::None;

    let colony = match colony_manager.get_by_body(body_index) {
        Some(c) => c,
        None => {
            return ColonyScreenAction::GoToTrackingStation;
        }
    };

    let hab_score = body_habitability.get(body_index).copied().unwrap_or(0);
    let hab_mult = (200.0 - hab_score as f64) / 100.0;

    // Pre-compute resource rates
    let rates = compute_resource_rates(colony, hab_mult);

    // === Top panel: colony name, selector, date, time warp ===
    egui::TopBottomPanel::top("colony_top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new(&colony.name)
                    .size(18.0)
                    .color(egui::Color32::WHITE),
            );

            ui.separator();

            // Colony selector ComboBox
            if colony_manager.colonies.len() > 1 {
                let selected_name = &colony.name;
                egui::ComboBox::from_id_source("colony_screen_selector")
                    .selected_text(selected_name.as_str())
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for c in &colony_manager.colonies {
                            let name = &c.name;
                            if ui
                                .selectable_label(c.body_index == body_index, name.as_str())
                                .clicked()
                            {
                                action = ColonyScreenAction::SwitchColony(c.body_index);
                            }
                        }
                    });

                ui.separator();
            }

            // Time warp buttons
            ui.label("Time Warp:");
            for (i, &warp) in warp_levels.iter().enumerate() {
                let label = if warp >= 1_000_000_000.0 {
                    format!("{}B", (warp / 1_000_000_000.0) as i32)
                } else if warp >= 1_000_000.0 {
                    format!("{}M", (warp / 1_000_000.0) as i32)
                } else if warp >= 1000.0 {
                    format!("{}K", (warp / 1000.0) as i32)
                } else {
                    format!("{}x", warp as i32)
                };
                let is_selected = i == current_warp_index;
                if ui.selectable_label(is_selected, &label).clicked() {
                    action = ColonyScreenAction::ChangeWarp(i);
                }
            }
            ui.separator();
            let current_warp = warp_levels[current_warp_index];
            ui.label(format!("Current: {}x", current_warp as i64));

            ui.separator();
            ui.label(date_str);
        });
    });

    // === Pause overlay ===
    if paused {
        egui::Area::new(egui::Id::new("colony_pause_overlay"))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                    .inner_margin(egui::Margin::same(40.0))
                    .rounding(egui::Rounding::same(8.0))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading(
                                egui::RichText::new("Paused")
                                    .size(32.0)
                                    .color(egui::Color32::WHITE),
                            );
                            ui.add_space(20.0);
                            if can_return_to_flight {
                                if ui
                                    .button(egui::RichText::new("Return to Flight").size(18.0))
                                    .clicked()
                                {
                                    action = ColonyScreenAction::ReturnToFlight;
                                }
                                ui.add_space(8.0);
                            }
                            if ui
                                .button(egui::RichText::new("Tracking Station").size(18.0))
                                .clicked()
                            {
                                action = ColonyScreenAction::GoToTrackingStation;
                            }
                            ui.add_space(8.0);
                            if ui
                                .button(egui::RichText::new("Colony Overview").size(18.0))
                                .clicked()
                            {
                                action = ColonyScreenAction::GoToColonyOverview;
                            }
                            ui.add_space(8.0);
                            if ui
                                .button(egui::RichText::new("Main Menu").size(18.0))
                                .clicked()
                            {
                                action = ColonyScreenAction::GoToMainMenu;
                            }
                        });
                    });
            });

        super::flight::render_toasts(ctx, active_toasts);

        return action;
    }

    // === Central panel: card-based colony content ===
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let body_name = body_names
                    .get(body_index)
                    .map(|s| s.as_str())
                    .unwrap_or("Unknown");

                // 1. Overview card
                render_overview_card(ui, colony, body_name);

                // 2. Power card
                render_power_card(ui, colony, solar_power_factor);

                // 3. Buildings card
                render_buildings_card(
                    ui,
                    ctx,
                    colony,
                    body_index,
                    body_mineable,
                    body_atmospheric,
                    &mut action,
                );

                // 4. Construction card
                render_construction_card(
                    ui,
                    colony,
                    body_index,
                    hab_score,
                    tech_tree,
                    &mut action,
                );

                // 5. Maintenance card
                render_maintenance_card(ui, colony, hab_mult);

                // 6. Resources card
                render_resources_card(ui, colony, &rates);

                // 7. Trade routes section (read-only)
                {
                    ui.add_space(4.0);
                    let trade_action = super::trade_ui::render_colony_trade_section(
                        ui, body_index, fleet, body_names, earth_index,
                    );
                    if trade_action != TradeAction::None {
                        action = ColonyScreenAction::Trade(trade_action);
                    }
                }

                // 8. Hangar card
                render_hangar_card(ui, colony, body_index, &mut action);

                // 9. Debug section (no card frame)
                ui.add_space(4.0);
                egui::CollapsingHeader::new("Debug")
                    .default_open(false)
                    .show(ui, |ui| {
                        // Add Resource
                        ui.horizontal(|ui| {
                            ui.label("Add Resource:");
                            let res_id = egui::Id::new("cs_debug_resource_idx");
                            let mut selected_idx: usize =
                                ui.data(|d| d.get_temp(res_id).unwrap_or(0));
                            let all_resources = ResourceType::all();
                            let selected_rt =
                                all_resources[selected_idx.min(all_resources.len() - 1)];

                            egui::ComboBox::from_id_source("cs_debug_resource")
                                .selected_text(selected_rt.display_name())
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    for (i, rt) in all_resources.iter().enumerate() {
                                        if ui
                                            .selectable_label(
                                                i == selected_idx,
                                                rt.display_name(),
                                            )
                                            .clicked()
                                        {
                                            selected_idx = i;
                                        }
                                    }
                                });
                            ui.data_mut(|d| d.insert_temp(res_id, selected_idx));

                            let amt_id = egui::Id::new("cs_debug_resource_amt");
                            let mut amount: f64 =
                                ui.data(|d| d.get_temp(amt_id).unwrap_or(10_000.0));
                            ui.add(
                                egui::DragValue::new(&mut amount)
                                    .clamp_range(100.0..=1_000_000.0)
                                    .speed(100.0)
                                    .suffix(" kg"),
                            );
                            ui.data_mut(|d| d.insert_temp(amt_id, amount));

                            if ui.button("Add").clicked() {
                                action = ColonyScreenAction::DebugAddResource(
                                    body_index,
                                    selected_rt,
                                    amount,
                                );
                            }
                        });

                        // Add Building
                        ui.horizontal(|ui| {
                            ui.label("Add Building:");
                            let bld_id = egui::Id::new("cs_debug_building_idx");
                            let mut selected_idx: usize =
                                ui.data(|d| d.get_temp(bld_id).unwrap_or(0));
                            let all_buildings = BuildingType::all();
                            let selected_bt =
                                all_buildings[selected_idx.min(all_buildings.len() - 1)];

                            egui::ComboBox::from_id_source("cs_debug_building")
                                .selected_text(selected_bt.display_name())
                                .width(200.0)
                                .show_ui(ui, |ui| {
                                    for (i, bt) in all_buildings.iter().enumerate() {
                                        if ui
                                            .selectable_label(
                                                i == selected_idx,
                                                bt.display_name(),
                                            )
                                            .clicked()
                                        {
                                            selected_idx = i;
                                        }
                                    }
                                });
                            ui.data_mut(|d| d.insert_temp(bld_id, selected_idx));

                            if ui.button("Add").clicked() {
                                action = ColonyScreenAction::DebugAddBuilding(
                                    body_index,
                                    selected_bt,
                                );
                            }
                        });

                        // Add Crew
                        ui.horizontal(|ui| {
                            ui.label("Add Crew:");
                            let crew_id = egui::Id::new("cs_debug_crew_amt");
                            let mut count: u32 =
                                ui.data(|d| d.get_temp(crew_id).unwrap_or(10));
                            ui.add(
                                egui::DragValue::new(&mut count)
                                    .clamp_range(1..=1000)
                                    .speed(1.0),
                            );
                            ui.data_mut(|d| d.insert_temp(crew_id, count));

                            if ui.button("Add").clicked() {
                                action =
                                    ColonyScreenAction::DebugAddCrew(body_index, count);
                            }
                        });
                    });
            });
    });

    // Toast notifications
    super::flight::render_toasts(ctx, active_toasts);

    action
}
