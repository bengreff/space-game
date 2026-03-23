use std::collections::{BTreeMap, HashMap};

use crate::colony::{BuildingType, ColonyManager, FactoryRecipe, ResourceType};

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

/// Actions returned by the colony screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColonyScreenAction {
    None,
    QueueBuilding(usize, BuildingType),
    AddMineAssignment(usize, ResourceType),
    RemoveMineAssignment(usize, ResourceType),
    AddCollectorAssignment(usize, ResourceType),
    RemoveCollectorAssignment(usize, ResourceType),
    AddFactoryAssignment(usize, FactoryRecipe),
    RemoveFactoryAssignment(usize, FactoryRecipe),
    ReturnToFlight,
    GoToTrackingStation,
    GoToMainMenu,
    ChangeWarp(usize),
    SwitchColony(usize),
    DebugAddResource(usize, ResourceType, f64),
    DebugAddBuilding(usize, BuildingType),
    DebugAddCrew(usize, u32),
}

/// Render the full-screen colony management screen.
/// Returns a ColonyScreenAction describing what the user wants to do.
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

    // === Central panel: colony content ===
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // ================================================================
                // === Overview ===
                // ================================================================
                ui.heading("Overview");

                let body_name = body_names
                    .get(body_index)
                    .map(|s| s.as_str())
                    .unwrap_or("Unknown");
                let location = if colony.is_orbital_station {
                    format!("Orbital station, {}", body_name)
                } else {
                    format!("Surface, {}", body_name)
                };
                ui.label(format!("{} ({})", colony.name, location));

                // Crew with capacity
                let crew_cap = colony.crew_capacity();
                if crew_cap > 0 {
                    ui.label(format!("Crew: {} / {}", colony.crew, crew_cap));
                } else {
                    ui.label(format!("Crew: {}", colony.crew));
                }

                // Crew death indicator
                if let Some(crisis_crew) = colony.crew_at_crisis_start {
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
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 60, 60),
                        format!(
                            "  Crew declining: ~{:.1}/day ({})",
                            deaths_per_day, reason
                        ),
                    );
                }

                // Food with capacity
                let food_days = colony.food_days_remaining();
                let food_cap = colony.food_capacity();
                let food_text = if food_days.is_infinite() {
                    if food_cap > 0.0 {
                        format!(
                            "Food: {} / {} (no crew)",
                            format_colony_mass(colony.food_stored),
                            format_colony_mass(food_cap)
                        )
                    } else {
                        format!("Food: {} (no crew)", format_colony_mass(colony.food_stored))
                    }
                } else if food_cap > 0.0 {
                    format!(
                        "Food: {} / {} ({:.1} days)",
                        format_colony_mass(colony.food_stored),
                        format_colony_mass(food_cap),
                        food_days
                    )
                } else {
                    format!(
                        "Food: {} ({:.1} days)",
                        format_colony_mass(colony.food_stored),
                        food_days
                    )
                };
                let food_color = if food_days < 10.0 && !food_days.is_infinite() {
                    egui::Color32::from_rgb(255, 100, 100)
                } else if food_days < 30.0 && !food_days.is_infinite() {
                    egui::Color32::from_rgb(220, 200, 80)
                } else {
                    ui.visuals().text_color()
                };
                ui.colored_label(food_color, food_text);

                // Storage
                let storage_used = colony.resources.total_mass();
                let storage_cap = colony.storage_capacity();
                ui.label(format!(
                    "Storage: {} / {}",
                    format_colony_mass(storage_used),
                    format_colony_mass(storage_cap)
                ));

                ui.add_space(8.0);
                ui.separator();

                // ================================================================
                // === Power ===
                // ================================================================
                ui.heading("Power");

                // --- Power production by building type ---
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

                if !prod_map.is_empty() {
                    ui.label("Production:");
                    for (name, (count, total_kw)) in &prod_map {
                        ui.label(format!(
                            "  {} ({}x): {}",
                            name,
                            count,
                            format_power_kw(*total_kw)
                        ));
                    }
                } else {
                    ui.label("Production: none");
                }

                // --- Power demand by building type + factory recipe ---
                // Use Vec to preserve insertion order (BTreeMap would sort alphabetically)
                let mut demand_entries: Vec<(String, u32, f64)> = Vec::new();
                let mut demand_map: BTreeMap<String, usize> = BTreeMap::new(); // key -> index

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
                            let key =
                                format!("Factory \u{2014} {}", recipe.display_name());
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

                if !demand_entries.is_empty() {
                    ui.add_space(4.0);
                    ui.label("Demand:");
                    for (name, count, total_kw) in &demand_entries {
                        ui.label(format!(
                            "  {} ({}x): {}",
                            name,
                            count,
                            format_power_kw(*total_kw)
                        ));
                    }
                } else {
                    ui.add_space(4.0);
                    ui.label("Demand: none");
                }

                // --- Allocation summary ---
                ui.add_space(4.0);
                if colony.habitat_power_fraction < 1.0 {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 80, 80),
                        format!(
                            "Habitat power: {:.0}% \u{2014} CREW AT RISK",
                            colony.habitat_power_fraction * 100.0
                        ),
                    );
                } else if colony.crew > 0 {
                    ui.label(format!(
                        "Habitat power: {:.0}%",
                        colony.habitat_power_fraction * 100.0
                    ));
                }
                if colony.other_power_fraction < 1.0 {
                    let color = if colony.other_power_fraction < 0.5 {
                        egui::Color32::from_rgb(255, 80, 80)
                    } else {
                        egui::Color32::from_rgb(220, 200, 80)
                    };
                    ui.colored_label(
                        color,
                        format!(
                            "Building power: {:.0}%",
                            colony.other_power_fraction * 100.0
                        ),
                    );
                }

                let power_surplus = colony.power_generated - colony.power_consumed;
                let power_color = if power_surplus < 0.0 {
                    egui::Color32::from_rgb(255, 100, 100)
                } else {
                    egui::Color32::from_rgb(100, 255, 100)
                };
                ui.colored_label(
                    power_color,
                    format!(
                        "Net: {}{}",
                        if power_surplus >= 0.0 { "+" } else { "" },
                        format_power_kw(power_surplus),
                    ),
                );

                ui.add_space(8.0);
                ui.separator();

                // ================================================================
                // === Buildings ===
                // ================================================================
                ui.heading("Buildings");

                if colony.buildings.is_empty() {
                    ui.label("No buildings.");
                } else {
                    // §2: Show building counts without degradation
                    let mut counts: BTreeMap<&str, (BuildingType, u32)> = BTreeMap::new();
                    for b in &colony.buildings {
                        let entry = counts
                            .entry(b.building_type.display_name())
                            .or_insert((b.building_type, 0));
                        entry.1 += 1;
                    }

                    for (_name, (bt, count)) in &counts {
                        if *bt == BuildingType::Mine || *bt == BuildingType::Factory {
                            continue;
                        }
                        let name_part = if *count > 1 {
                            format!("{} ({}x)", bt.display_name(), count)
                        } else {
                            bt.display_name().to_string()
                        };
                        ui.label(name_part);
                    }

                    // §10: Mine sub-section with +/- buttons
                    let mine_count = colony
                        .buildings
                        .iter()
                        .filter(|b| b.building_type == BuildingType::Mine)
                        .count();

                    if mine_count > 0 {
                        ui.add_space(4.0);
                        ui.label(format!("Mines ({}x):", mine_count));

                        let mineable = body_mineable
                            .get(body_index)
                            .cloned()
                            .unwrap_or_default();

                        // Count assignments per resource
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

                        // Show each mineable resource with count and buttons
                        for &res in &mineable {
                            let count = assigned_counts.get(&res).copied().unwrap_or(0);
                            if count == 0 && unassigned == 0 {
                                continue;
                            }
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.label(format!("{}: {}", res.display_name(), count));
                                if count > 0 {
                                    if ui.small_button("\u{2212}").clicked() {
                                        action = ColonyScreenAction::RemoveMineAssignment(
                                            body_index, res,
                                        );
                                    }
                                }
                                if unassigned > 0 {
                                    let resp = ui.small_button("+");
                                    if resp.clicked() {
                                        action = ColonyScreenAction::AddMineAssignment(
                                            body_index, res,
                                        );
                                    }
                                    resp.on_hover_text("Produces 2,000 kg/day");
                                }
                            });
                        }

                        // Also show non-mineable resources that have assignments (edge case)
                        for (&res, &count) in &assigned_counts {
                            if !mineable.contains(&res) && count > 0 {
                                ui.horizontal(|ui| {
                                    ui.add_space(16.0);
                                    ui.label(format!("{}: {}", res.display_name(), count));
                                    if ui.small_button("\u{2212}").clicked() {
                                        action = ColonyScreenAction::RemoveMineAssignment(
                                            body_index, res,
                                        );
                                    }
                                });
                            }
                        }

                        if unassigned > 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.colored_label(
                                    egui::Color32::from_rgb(180, 180, 180),
                                    format!("Unassigned: {}", unassigned),
                                );
                            });
                        }
                    }

                    // Atmospheric Collector sub-section with +/- buttons
                    let collector_count = colony
                        .buildings
                        .iter()
                        .filter(|b| b.building_type == BuildingType::AtmosphericCollector)
                        .count();

                    if collector_count > 0 {
                        ui.add_space(4.0);
                        ui.label(format!("Atmospheric Collectors ({}x):", collector_count));

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
                                ui.label(format!("{}: {}", res.display_name(), count));
                                if count > 0 {
                                    if ui.small_button("\u{2212}").clicked() {
                                        action = ColonyScreenAction::RemoveCollectorAssignment(
                                            body_index, res,
                                        );
                                    }
                                }
                                if unassigned > 0 {
                                    let resp = ui.small_button("+");
                                    if resp.clicked() {
                                        action = ColonyScreenAction::AddCollectorAssignment(
                                            body_index, res,
                                        );
                                    }
                                    resp.on_hover_text("Produces 10,000 kg/day");
                                }
                            });
                        }

                        // Show non-atmospheric resources that have assignments (edge case)
                        for (&res, &count) in &assigned_counts {
                            if !atmospheric.contains(&res) && count > 0 {
                                ui.horizontal(|ui| {
                                    ui.add_space(16.0);
                                    ui.label(format!("{}: {}", res.display_name(), count));
                                    if ui.small_button("\u{2212}").clicked() {
                                        action = ColonyScreenAction::RemoveCollectorAssignment(
                                            body_index, res,
                                        );
                                    }
                                });
                            }
                        }

                        if unassigned > 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.colored_label(
                                    egui::Color32::from_rgb(180, 180, 180),
                                    format!("Unassigned: {}", unassigned),
                                );
                            });
                        }
                    }

                    // §10: Factory sub-section with +/- buttons
                    let factory_count = colony
                        .buildings
                        .iter()
                        .filter(|b| b.building_type == BuildingType::Factory)
                        .count();

                    if factory_count > 0 {
                        ui.add_space(4.0);
                        ui.label(format!("Factories ({}x):", factory_count));

                        // Count assignments per recipe
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

                        // Show assigned recipes, then unassigned-only recipes
                        for &recipe in ALL_RECIPES {
                            let count = recipe_counts.get(&recipe).copied().unwrap_or(0);
                            if count == 0 && unassigned_factories == 0 {
                                continue;
                            }
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.label(format!("{}: {}", recipe.display_name(), count));
                                if count > 0 {
                                    if ui.small_button("\u{2212}").clicked() {
                                        action = ColonyScreenAction::RemoveFactoryAssignment(
                                            body_index, recipe,
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
                                        action = ColonyScreenAction::AddFactoryAssignment(
                                            body_index, recipe,
                                        );
                                    }
                                    resp.on_hover_text(tooltip);
                                }
                            });
                        }

                        if unassigned_factories > 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(16.0);
                                ui.colored_label(
                                    egui::Color32::from_rgb(180, 180, 180),
                                    format!("Unassigned: {}", unassigned_factories),
                                );
                            });
                        }
                    }
                }

                ui.add_space(8.0);
                ui.separator();

                // ================================================================
                // === Maintenance ===
                // ================================================================
                ui.heading("Maintenance");

                // Aggregate maintenance costs per 30 days across all buildings
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
                    ui.label("No maintenance required.");
                } else {
                    // Resource consumption table with days remaining
                    egui::Grid::new("cs_maintenance_grid")
                        .striped(true)
                        .min_col_width(120.0)
                        .show(ui, |ui| {
                            ui.strong("Resource");
                            ui.strong("Per 30 days");
                            ui.strong("In stock");
                            ui.strong("Days left");
                            ui.end_row();

                            for (_name, (res, cost_30d)) in &total_costs {
                                let in_stock = colony.resources.get(*res);
                                let daily_rate = cost_30d / 30.0;
                                let days_left = if daily_rate > 0.0 {
                                    in_stock / daily_rate
                                } else {
                                    f64::INFINITY
                                };

                                ui.label(res.display_name());
                                ui.label(format_colony_mass(*cost_30d));
                                ui.label(format_colony_mass(in_stock));

                                let days_color = if days_left < 10.0 {
                                    egui::Color32::from_rgb(255, 100, 100)
                                } else if days_left < 30.0 {
                                    egui::Color32::from_rgb(220, 200, 80)
                                } else {
                                    ui.visuals().text_color()
                                };
                                ui.colored_label(
                                    days_color,
                                    if days_left.is_infinite() {
                                        "\u{221e}".to_string()
                                    } else {
                                        format!("{:.0}", days_left)
                                    },
                                );
                                ui.end_row();
                            }
                        });

                    ui.add_space(4.0);
                    ui.label(format!(
                        "Total: {} / 30 days",
                        format_colony_mass(total_mass_per_30d)
                    ));

                    // Robot capacity
                    let mut robot_capacity_per_day = 0.0_f64;
                    for b in &colony.buildings {
                        if !b.operational {
                            continue;
                        }
                        match b.building_type {
                            BuildingType::ConstructionRobot => {
                                robot_capacity_per_day += 20_000.0
                            }
                            BuildingType::LightConstructionRobot => {
                                robot_capacity_per_day += 5_000.0
                            }
                            _ => {}
                        }
                    }
                    let daily_demand = total_mass_per_30d / 30.0;
                    let robot_color = if robot_capacity_per_day < daily_demand {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(
                        robot_color,
                        format!(
                            "Robot capacity: {} / day (demand: {} / day)",
                            format_colony_mass(robot_capacity_per_day),
                            format_colony_mass(daily_demand),
                        ),
                    );

                    // §3: Degraded buildings — yellow "Name — X% degraded"
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
                        for (name, (count, worst)) in &deg_counts {
                            let label = if *count > 1 {
                                format!(
                                    "{} ({}x) \u{2014} {:.0}% degraded",
                                    name,
                                    count,
                                    worst * 100.0
                                )
                            } else {
                                format!(
                                    "{} \u{2014} {:.0}% degraded",
                                    name,
                                    worst * 100.0
                                )
                            };
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 180, 60),
                                label,
                            );
                        }
                    }
                }

                ui.add_space(8.0);
                ui.separator();

                // ================================================================
                // === Construction ===
                // ================================================================
                ui.heading("Construction");

                if colony.construction_queue.is_empty() {
                    ui.label("No active construction.");
                } else {
                    for item in &colony.construction_queue {
                        let progress = if item.total_mass > 0.0 {
                            (item.mass_assembled / item.total_mass) as f32
                        } else {
                            0.0
                        };
                        let text = format!(
                            "{} ({} / {})",
                            item.building_type.display_name(),
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

                ui.horizontal(|ui| {
                    ui.label("Add building:");
                    egui::ComboBox::from_id_source("cs_add_building_combo")
                        .selected_text("Select...")
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for &bt in BUILDABLE_BUILDINGS {
                                let can_build = colony.can_queue_building(bt, hab_score);
                                let costs = bt.build_cost();
                                let cost_str: String = costs
                                    .iter()
                                    .map(|(r, amt)| {
                                        format!(
                                            "{} {}",
                                            format_colony_mass(*amt),
                                            r.display_name()
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let label =
                                    format!("{} [{}]", bt.display_name(), cost_str);

                                if ui
                                    .add_enabled(
                                        can_build,
                                        egui::SelectableLabel::new(false, &label),
                                    )
                                    .clicked()
                                {
                                    action =
                                        ColonyScreenAction::QueueBuilding(body_index, bt);
                                }
                            }
                        });
                });

                ui.add_space(8.0);
                ui.separator();

                // ================================================================
                // === Resources ===
                // ================================================================
                ui.heading("Resources");

                // §8: Compute production/consumption rates per resource
                let mut production: HashMap<ResourceType, f64> = HashMap::new();
                let mut consumption: HashMap<ResourceType, f64> = HashMap::new();

                for b in &colony.buildings {
                    if !b.operational {
                        continue;
                    }

                    // Mine production
                    if b.building_type == BuildingType::Mine {
                        if let Some(res) = b.assigned_resource {
                            let rate =
                                2000.0 * (1.0 - b.degradation) * colony.other_power_fraction;
                            *production.entry(res).or_insert(0.0) += rate;
                        }
                    }

                    // Atmospheric Collector production
                    if b.building_type == BuildingType::AtmosphericCollector {
                        if let Some(res) = b.assigned_resource {
                            let rate =
                                10_000.0 * (1.0 - b.degradation) * colony.other_power_fraction;
                            *production.entry(res).or_insert(0.0) += rate;
                        }
                    }

                    // Factory production/consumption
                    if b.building_type == BuildingType::Factory {
                        if let Some(recipe) = b.assigned_recipe {
                            let batches_per_day = 24.0 / recipe.batch_time_hours();
                            let factor = batches_per_day
                                * (1.0 - b.degradation)
                                * colony.other_power_fraction;
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
                            *consumption.entry(ResourceType::EnrichedUranium).or_insert(0.0) += 0.5;
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
                    *consumption.entry(ResourceType::Food).or_insert(0.0) +=
                        0.5 * colony.crew as f64;
                }

                // Build resource list: regular resources + food
                let mut resources: Vec<(ResourceType, f64)> = colony
                    .resources
                    .iter()
                    .filter(|(_, &amt)| amt > 0.001)
                    .map(|(&rt, &amt)| (rt, amt))
                    .collect();
                resources.sort_by(|a, b| a.0.display_name().cmp(b.0.display_name()));

                // §7: Add Food row
                let has_food_in_list = resources.iter().any(|(rt, _)| *rt == ResourceType::Food);
                if !has_food_in_list && (colony.food_stored > 0.001 || colony.crew > 0) {
                    resources.push((ResourceType::Food, colony.food_stored));
                    resources.sort_by(|a, b| a.0.display_name().cmp(b.0.display_name()));
                }

                // Also add resources that have production/consumption but zero stock
                let rate_resources: Vec<ResourceType> = production
                    .keys()
                    .chain(consumption.keys())
                    .copied()
                    .collect();
                for rt in rate_resources {
                    if rt == ResourceType::Food {
                        continue; // handled above
                    }
                    if !resources.iter().any(|(r, _)| *r == rt) {
                        resources.push((rt, 0.0));
                    }
                }
                resources.sort_by(|a, b| a.0.display_name().cmp(b.0.display_name()));

                if resources.is_empty() {
                    ui.label("No resources in storage.");
                } else {
                    egui::Grid::new("cs_resources_grid")
                        .striped(true)
                        .min_col_width(120.0)
                        .show(ui, |ui| {
                            ui.strong("Resource");
                            ui.strong("Amount");
                            ui.strong("Production");
                            ui.strong("Consumption");
                            ui.end_row();

                            for (rt, amt) in &resources {
                                let amount = if *rt == ResourceType::Food {
                                    colony.food_stored
                                } else {
                                    *amt
                                };
                                let prod = production.get(rt).copied().unwrap_or(0.0);
                                let cons = consumption.get(rt).copied().unwrap_or(0.0);

                                ui.label(rt.display_name());
                                ui.label(format_colony_mass(amount));

                                if prod > 0.001 {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(100, 255, 100),
                                        format_rate(prod),
                                    );
                                } else {
                                    ui.label("");
                                }

                                if cons > 0.001 {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 150, 100),
                                        format_rate(-cons),
                                    );
                                } else {
                                    ui.label("");
                                }

                                ui.end_row();
                            }
                        });
                }

                ui.add_space(8.0);
                ui.separator();

                // ================================================================
                // === Debug ===
                // ================================================================
                egui::CollapsingHeader::new("Debug")
                    .default_open(false)
                    .show(ui, |ui| {
                        // Add Resource (also handles Food via routing in main.rs)
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
