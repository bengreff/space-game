use crate::colony::{ColonyManager, FleetManager};
use crate::parts::{BlueprintRegistry, PartDefinitions};
use super::types::ColonyOverviewAction;

/// Render the colony overview screen showing all colonies.
pub fn render_colony_overview_screen(
    ctx: &egui::Context,
    colony_manager: &ColonyManager,
    body_names: &[String],
    warp_levels: &[f64],
    current_warp_index: usize,
    date_str: &str,
    paused: bool,
    company_money: f64,
    science_available: f64,
    active_toasts: &[(String, std::time::Instant)],
    fleet: &FleetManager,
    earth_index: usize,
    route_creation: &mut super::trade_ui::RouteCreationState,
    blueprints: &BlueprintRegistry,
    part_defs: &PartDefinitions,
    solar_system: &crate::bodies::SolarSystem,
    sim_time: f64,
) -> ColonyOverviewAction {
    let mut action = ColonyOverviewAction::None;

    // === Top panel: time warp bar + date ===
    egui::TopBottomPanel::top("colony_overview_top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("Colony Overview")
                    .size(18.0)
                    .color(egui::Color32::WHITE),
            );
            ui.separator();

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
                    action = ColonyOverviewAction::ChangeWarp(i);
                }
            }
            ui.separator();
            let current_warp = warp_levels[current_warp_index];
            ui.label(format!("Current: {}x", current_warp as i64));

            ui.separator();
            ui.label(date_str);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("Science: {:.0}", science_available))
                        .color(egui::Color32::from_rgb(100, 180, 255)),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new(crate::colony::format_money(company_money))
                        .color(egui::Color32::from_rgb(100, 200, 100)),
                );
            });
        });
    });

    // === Pause overlay ===
    if paused {
        egui::Area::new(egui::Id::new("colony_overview_pause_overlay"))
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
                            if ui
                                .button(egui::RichText::new("Main Menu").size(18.0))
                                .clicked()
                            {
                                action = ColonyOverviewAction::GoToMainMenu;
                            }
                        });
                    });
            });

        super::flight::render_toasts(ctx, active_toasts);
        return action;
    }

    // === Central panel: colony list ===
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.heading(
                        egui::RichText::new("Colonies")
                            .size(24.0)
                            .color(egui::Color32::WHITE),
                    );
                });
                ui.add_space(16.0);

                if colony_manager.colonies.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            egui::RichText::new("No colonies established yet.")
                                .size(16.0)
                                .color(egui::Color32::from_rgb(160, 160, 160)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                "Land a vessel with a Habitat and Solar Farm to start a colony.",
                            )
                            .size(13.0)
                            .color(egui::Color32::from_rgb(120, 120, 120)),
                        );
                    });
                } else {
                    // Colony cards
                    for colony in &colony_manager.colonies {
                        let body_name = body_names
                            .get(colony.body_index)
                            .map(|s| s.as_str())
                            .unwrap_or("Unknown");

                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_unmultiplied(30, 35, 50, 220))
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .outer_margin(egui::Margin::symmetric(40.0, 4.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(&colony.name)
                                                .size(16.0)
                                                .strong()
                                                .color(egui::Color32::WHITE),
                                        );
                                        ui.label(
                                            egui::RichText::new(body_name)
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(160, 160, 160)),
                                        );
                                    });

                                    ui.add_space(20.0);

                                    // Crew
                                    let crew_cap = colony.crew_capacity();
                                    ui.label(format!("Crew: {} / {}", colony.crew, crew_cap));

                                    ui.add_space(12.0);

                                    // Power net
                                    let power_net =
                                        colony.power_generated - colony.power_consumed;
                                    let power_color = if power_net < 0.0 {
                                        egui::Color32::from_rgb(255, 100, 100)
                                    } else {
                                        egui::Color32::from_rgb(100, 255, 100)
                                    };
                                    let power_text = if power_net.abs() >= 1000.0 {
                                        format!("{:+.1} MW", power_net / 1000.0)
                                    } else {
                                        format!("{:+.1} kW", power_net)
                                    };
                                    ui.colored_label(power_color, power_text);

                                    ui.add_space(12.0);

                                    // Food days
                                    let food_days = colony.food_days_remaining();
                                    let food_color = if food_days < 10.0 && !food_days.is_infinite()
                                    {
                                        egui::Color32::from_rgb(255, 100, 100)
                                    } else if food_days < 30.0 && !food_days.is_infinite() {
                                        egui::Color32::from_rgb(220, 200, 80)
                                    } else {
                                        egui::Color32::from_rgb(160, 160, 160)
                                    };
                                    let food_text = if food_days.is_infinite() {
                                        "Food: stable".to_string()
                                    } else {
                                        format!("Food: {:.0}d", food_days)
                                    };
                                    ui.colored_label(food_color, food_text);

                                    ui.add_space(12.0);

                                    // Building count
                                    ui.label(format!(
                                        "{} buildings",
                                        colony.buildings.len()
                                    ));

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("Open").clicked() {
                                                action = ColonyOverviewAction::OpenColony(
                                                    colony.body_index,
                                                );
                                            }
                                        },
                                    );
                                });
                            });
                    }
                }

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(4.0);

                // Trade Routes heading with Create button
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Trade Routes")
                            .size(14.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(8.0);
                    if ui.button("Create Trade Route").clicked() {
                        *route_creation = super::trade_ui::RouteCreationState::start();
                    }
                });
                ui.add_space(4.0);

                let trade_action = super::trade_ui::render_fleet_overview_panel(
                    ui, fleet, body_names, earth_index,
                );
                if trade_action != super::types::TradeAction::None {
                    action = ColonyOverviewAction::Trade(trade_action);
                }
            });
    });

    // Route creation panel (modal, renders on top of everything)
    if route_creation.active {
        let panel_action = super::trade_ui::render_route_creation_panel(
            ctx,
            route_creation,
            fleet,
            colony_manager,
            body_names,
            earth_index,
            blueprints,
            part_defs,
            solar_system,
            sim_time,
        );
        if panel_action != super::types::TradeAction::None {
            action = ColonyOverviewAction::Trade(panel_action);
        }
    }

    // Toast notifications
    super::flight::render_toasts(ctx, active_toasts);

    action
}
