//! Trade route UI components: fleet overview panel, colony trade tab (read-only),
//! route creation panel, and route detail panel.

use crate::colony::{
    trade::{CargoManifest, FleetManager, RouteLeg, TradeRoute, TradeRouteId, TradeShipState},
    transfer::RouteCategory,
    ColonyManager, ResourceType,
};
use crate::parts::{BlueprintRegistry, PartDefinitions};
use super::types::TradeAction;

// ============================================================
// Color constants (match colony_ui.rs)
// ============================================================
const COLOR_GREEN: egui::Color32 = egui::Color32::from_rgb(100, 255, 100);
const COLOR_RED: egui::Color32 = egui::Color32::from_rgb(255, 100, 100);
const COLOR_YELLOW: egui::Color32 = egui::Color32::from_rgb(220, 200, 80);
const COLOR_GRAY: egui::Color32 = egui::Color32::from_rgb(160, 160, 160);
const CARD_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(30, 35, 50, 220);

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

/// Format flight time (seconds) into a human-readable string.
fn format_flight_time(seconds: f64) -> String {
    let days = seconds / 86400.0;
    if days >= 365.0 {
        format!("{:.1} years", days / 365.0)
    } else if days >= 30.0 {
        format!("{:.0} days", days)
    } else if days >= 1.0 {
        format!("{:.1} days", days)
    } else {
        let hours = seconds / 3600.0;
        format!("{:.1} hours", hours)
    }
}

/// Format delta-v with appropriate units.
fn format_dv(dv: f64) -> String {
    if dv >= 1000.0 {
        format!("{:.2} km/s", dv / 1000.0)
    } else {
        format!("{:.0} m/s", dv)
    }
}

// ============================================================
// Route Creation State (ephemeral, not saved)
// ============================================================

/// State for the single-panel route creation/editing UI.
#[derive(Default)]
pub struct RouteCreationState {
    pub active: bool,
    pub editing_route_id: Option<TradeRouteId>,
    // User selections
    pub route_name: String,
    pub blueprint_name: String,
    pub source_body: Option<usize>,
    pub dest_body: Option<usize>,
    pub cargo_items: Vec<(ResourceType, f64)>,
    pub selected_resource: usize,
    pub cargo_amount: f64,
    pub crew: u32,
    pub dv_budget: f64,
    pub dv_budget_text: String,
    pub interval_days: f64,
    pub ships_per_window: u32,
    pub build_new_ship: bool,
    // Cache (recomputed when inputs change)
    pub cached_category: Option<RouteCategory>,
    pub cached_leg: Option<crate::colony::transfer::LegResult>,
    pub cached_min_dv: f64,
    pub cached_ship_dv_empty: f64,
    pub cached_ship_dv_with_cargo: f64,
    pub cached_cargo_capacity: f64,
    pub cached_synodic_period: Option<f64>,
    pub cached_flight_time: f64,
    pub cached_fuel_reqs: Vec<(ResourceType, f64)>,
    pub cached_container_capacity: f64,
    pub cached_crew_capacity: u32,
    pub cached_has_probe_core: bool,
    pub cache_key: u64,
}

impl RouteCreationState {
    /// Start creating a new route.
    pub fn start() -> Self {
        Self {
            active: true,
            interval_days: 30.0,
            ships_per_window: 1,
            build_new_ship: true,
            cargo_amount: 1000.0,
            ..Default::default()
        }
    }

    /// Start editing an existing route.
    pub fn start_from_route(route: &TradeRoute, fleet: &FleetManager) -> Self {
        let _ = fleet; // Reserved for future use (ship info)
        let source = FleetManager::route_source(route);
        let dest = FleetManager::route_destination(route);
        Self {
            active: true,
            editing_route_id: Some(route.id),
            route_name: route.name.clone(),
            blueprint_name: route.blueprint_name.clone(),
            source_body: source,
            dest_body: dest,
            cargo_items: route.outbound_cargo.items.clone(),
            crew: route.crew,
            dv_budget: route.total_delta_v,
            dv_budget_text: String::new(),
            interval_days: route.interval_days,
            ships_per_window: route.ships_per_window,
            build_new_ship: false, // Don't build when editing
            cached_category: Some(route.route_category),
            cargo_amount: 1000.0,
            ..Default::default()
        }
    }

    /// Compute a simple hash of the key inputs for cache invalidation.
    fn compute_cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.source_body.hash(&mut hasher);
        self.dest_body.hash(&mut hasher);
        self.blueprint_name.hash(&mut hasher);
        (self.dv_budget as u64).hash(&mut hasher);
        let cargo_mass = self.cargo_items.iter().map(|(_, kg)| *kg as u64).sum::<u64>();
        cargo_mass.hash(&mut hasher);
        hasher.finish()
    }
}

// ============================================================
// Fleet Overview Panel (for colony overview screen)
// ============================================================

/// Render the fleet overview section in the colony overview screen.
/// Returns a TradeAction if the user interacts with a route/ship.
pub fn render_fleet_overview_panel(
    ui: &mut egui::Ui,
    fleet: &FleetManager,
    body_names: &[String],
    earth_index: usize,
) -> TradeAction {
    let mut action = TradeAction::None;

    if fleet.routes.is_empty() && fleet.ships.is_empty() {
        ui.label(
            egui::RichText::new("No trade routes or ships yet.")
                .size(12.0)
                .color(COLOR_GRAY),
        );
        return action;
    }

    // Routes
    if !fleet.routes.is_empty() {
        for route in &fleet.routes {
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let status_color = if route.paused {
                        COLOR_RED
                    } else {
                        COLOR_GREEN
                    };
                    let status_text = if route.paused { "Paused" } else { "Active" };

                    ui.label(
                        egui::RichText::new(&route.name)
                            .size(13.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    ui.label(
                        egui::RichText::new(format!("[{}]", status_text))
                            .size(11.0)
                            .color(status_color),
                    );

                    ui.add_space(8.0);

                    let source = body_name_or_earth(
                        FleetManager::route_source(route),
                        body_names,
                        earth_index,
                    );
                    let dest = body_name_or_earth(
                        FleetManager::route_destination(route),
                        body_names,
                        earth_index,
                    );
                    ui.label(
                        egui::RichText::new(format!("{} \u{2192} {}", source, dest))
                            .size(12.0)
                            .color(COLOR_GRAY),
                    );

                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format_dv(route.total_delta_v))
                            .size(11.0)
                            .color(COLOR_GRAY),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("\u{2716}").on_hover_text("Delete route").clicked() {
                            action = TradeAction::DeleteRoute(route.id);
                        }
                        // Edit button
                        if ui.small_button("\u{270e}").on_hover_text("Edit route").clicked() {
                            action = TradeAction::OpenEditor(route.id);
                        }
                        if route.paused {
                            if ui.small_button("\u{25b6}").on_hover_text("Resume").clicked() {
                                action = TradeAction::ResumeRoute(route.id);
                            }
                        } else {
                            if ui.small_button("\u{23f8}").on_hover_text("Pause").clicked() {
                                action = TradeAction::PauseRoute(route.id);
                            }
                        }

                        // Manual launch button (builds and launches a new ship)
                        let has_ship_in_transit = route.assigned_ship_id
                            .and_then(|id| fleet.get_ship(id))
                            .map_or(false, |s| s.state == TradeShipState::InTransit);
                        if !route.paused && !has_ship_in_transit {
                            if ui.small_button("Launch").on_hover_text("Build and launch a ship now").clicked() {
                                action = TradeAction::ManualLaunch(route.id);
                            }
                        }
                    });
                });

                // Alert line (if any)
                if let Some(ref alert) = route.alert_reason {
                    ui.label(
                        egui::RichText::new(format!("\u{26a0} {}", alert))
                            .size(11.0)
                            .color(COLOR_RED),
                    );
                }

                // Ship status line — show in-transit ship progress
                if let Some(ship_id) = route.assigned_ship_id {
                    if let Some(ship) = fleet.get_ship(ship_id) {
                        if ship.state == TradeShipState::InTransit {
                            ui.horizontal(|ui| {
                                let dest = if let Some(r) = fleet.get_route(route.id) {
                                    r.legs.get(ship.current_leg).and_then(|l| l.to_body)
                                } else {
                                    None
                                };
                                let dest_name =
                                    body_name_or_earth(dest, body_names, earth_index);
                                let total = route.total_flight_time;
                                let elapsed = (total - ship.transit_remaining).max(0.0);
                                let elapsed_days = elapsed / 86400.0;
                                let total_days = total / 86400.0;
                                ui.label(
                                    egui::RichText::new(format!(
                                        "\u{1f680} In transit to {} \u{2014} {:.1} / {:.1} days",
                                        dest_name, elapsed_days, total_days
                                    ))
                                    .size(11.0)
                                    .color(COLOR_YELLOW),
                                );
                            });
                        }
                    }
                }
            });
        }
    }

    // Unassigned ships
    let unassigned: Vec<&crate::colony::trade::TradeShip> = fleet
        .ships
        .iter()
        .filter(|s| s.assigned_route.is_none())
        .collect();
    if !unassigned.is_empty() {
        ui.add_space(8.0);
        section_heading(ui, "Unassigned Ships");
        for ship in unassigned {
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&ship.name)
                            .size(12.0)
                            .color(egui::Color32::WHITE),
                    );
                    let loc = body_name_or_earth(ship.location, body_names, earth_index);
                    ui.label(
                        egui::RichText::new(format!("at {}", loc))
                            .size(11.0)
                            .color(COLOR_GRAY),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "({}, {:.0} m/s dv)",
                            ship.blueprint_name, ship.cached_delta_v
                        ))
                        .size(11.0)
                        .color(COLOR_GRAY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("\u{2716}")
                            .on_hover_text("Delete ship")
                            .clicked()
                        {
                            action = TradeAction::DeleteShip(ship.id);
                        }
                    });
                });
            });
        }
    }

    action
}

// ============================================================
// Route Creation Panel (single-panel, replaces old wizard)
// ============================================================

/// Render the route creation/editing panel as an egui::Window modal.
/// Returns a TradeAction when the user creates/updates a route.
pub fn render_route_creation_panel(
    ctx: &egui::Context,
    state: &mut RouteCreationState,
    _fleet: &FleetManager,
    colony_manager: &ColonyManager,
    body_names: &[String],
    earth_index: usize,
    blueprints: &BlueprintRegistry,
    part_defs: &PartDefinitions,
    solar_system: &crate::bodies::SolarSystem,
    sim_time: f64,
) -> TradeAction {
    let mut action = TradeAction::None;

    if !state.active {
        return action;
    }

    let title = if state.editing_route_id.is_some() {
        "Edit Trade Route"
    } else {
        "Create Trade Route"
    };

    egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_width(520.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(600.0)
                .show(ui, |ui| {
                    // 1. Route Name
                    section_heading(ui, "Route Name");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut state.route_name);
                    });
                    // Auto-suggest if empty
                    if state.route_name.is_empty()
                        && state.source_body.is_some() != state.dest_body.is_some()
                        || (state.source_body.is_some()
                            && state.dest_body.is_some()
                            && state.source_body != state.dest_body)
                        || (state.source_body.is_none() && state.dest_body.is_some())
                    {
                        let src =
                            body_name_or_earth(state.source_body, body_names, earth_index);
                        let dst =
                            body_name_or_earth(state.dest_body, body_names, earth_index);
                        if src != dst {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Suggested: {} \u{2192} {}",
                                        src, dst
                                    ))
                                    .size(11.0)
                                    .color(COLOR_GRAY),
                                );
                                if ui.small_button("Use").clicked() {
                                    state.route_name =
                                        format!("{} \u{2192} {}", src, dst);
                                }
                            });
                        }
                    }
                    ui.add_space(8.0);

                    // 2. Ship Blueprint
                    section_heading(ui, "Ship Blueprint");
                    let all_blueprints = blueprints.all_blueprints();
                    if all_blueprints.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No blueprints saved. Save a vessel in the editor first.",
                            )
                            .color(COLOR_RED),
                        );
                    } else {
                        let bp_display = if state.blueprint_name.is_empty() {
                            "Select blueprint...".to_string()
                        } else {
                            let dv = blueprints
                                .get(&state.blueprint_name)
                                .map(|bp| {
                                    crate::colony::transfer::blueprint_total_delta_v(
                                        bp, part_defs,
                                    )
                                })
                                .unwrap_or(0.0);
                            format!("{} ({} dv)", state.blueprint_name, format_dv(dv))
                        };

                        egui::ComboBox::from_id_source("rcp_blueprint_selector")
                            .selected_text(&bp_display)
                            .width(300.0)
                            .show_ui(ui, |ui| {
                                for bp in &all_blueprints {
                                    let dv = crate::colony::transfer::blueprint_total_delta_v(
                                        bp, part_defs,
                                    );
                                    let label = format!("{} ({} dv)", bp.name, format_dv(dv));
                                    if ui
                                        .selectable_label(
                                            state.blueprint_name == bp.name,
                                            &label,
                                        )
                                        .clicked()
                                    {
                                        state.blueprint_name = bp.name.clone();
                                    }
                                }
                            });
                    }
                    ui.add_space(8.0);

                    // 3. Source
                    section_heading(ui, "Source");
                    {
                        // Build source list: Earth + colonies with Launchpad
                        let mut sources: Vec<(Option<usize>, String)> = Vec::new();
                        sources.push((None, "Earth".to_string()));
                        for colony in &colony_manager.colonies {
                            if colony.operational_building_count(
                                crate::colony::BuildingType::Launchpad,
                            ) > 0
                            {
                                let name = body_names
                                    .get(colony.body_index)
                                    .cloned()
                                    .unwrap_or_else(|| format!("Body {}", colony.body_index));
                                sources.push((Some(colony.body_index), name));
                            }
                        }

                        let current_source =
                            body_name_or_earth(state.source_body, body_names, earth_index);
                        egui::ComboBox::from_id_source("rcp_source_selector")
                            .selected_text(&current_source)
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for (body, name) in &sources {
                                    if ui
                                        .selectable_label(state.source_body == *body, name)
                                        .clicked()
                                    {
                                        state.source_body = *body;
                                    }
                                }
                            });
                    }
                    ui.add_space(8.0);

                    // 4. Destination
                    section_heading(ui, "Destination");
                    {
                        let mut destinations: Vec<(Option<usize>, String)> = Vec::new();
                        // Add Earth if source is not Earth
                        if state.source_body.is_some() {
                            destinations.push((None, "Earth".to_string()));
                        }
                        // Add all colonies except source
                        for colony in &colony_manager.colonies {
                            if Some(colony.body_index) != state.source_body {
                                let name = body_names
                                    .get(colony.body_index)
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        format!("Body {}", colony.body_index)
                                    });
                                destinations.push((Some(colony.body_index), name));
                            }
                        }

                        let current_dest =
                            body_name_or_earth(state.dest_body, body_names, earth_index);
                        egui::ComboBox::from_id_source("rcp_dest_selector")
                            .selected_text(&current_dest)
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for (body, name) in &destinations {
                                    if ui
                                        .selectable_label(state.dest_body == *body, name)
                                        .clicked()
                                    {
                                        state.dest_body = *body;
                                    }
                                }
                            });
                    }
                    ui.add_space(8.0);

                    // === Cache invalidation ===
                    let new_key = state.compute_cache_key();
                    if new_key != state.cache_key {
                        state.cache_key = new_key;
                        // Recompute cached values
                        let category = crate::colony::transfer::classify_route(
                            state.source_body,
                            state.dest_body,
                            solar_system,
                        );
                        state.cached_category = Some(category);

                        // Hohmann baseline
                        state.cached_leg = crate::colony::transfer::compute_leg_delta_v(
                            state.source_body,
                            state.dest_body,
                            sim_time,
                            0.0,
                            solar_system,
                        );
                        state.cached_min_dv =
                            state.cached_leg.as_ref().map_or(0.0, |l| l.total_dv);

                        // Initialize dv_budget to minimum if not set yet
                        if state.dv_budget < state.cached_min_dv {
                            state.dv_budget = state.cached_min_dv;
                        }

                        // Ship dv
                        if let Some(bp) = blueprints.get(&state.blueprint_name) {
                            state.cached_ship_dv_empty =
                                crate::colony::transfer::blueprint_total_delta_v(bp, part_defs);
                            let cargo_mass: f64 =
                                state.cargo_items.iter().map(|(_, kg)| kg).sum();
                            state.cached_ship_dv_with_cargo =
                                crate::colony::transfer::blueprint_dv_with_cargo(
                                    bp, part_defs, cargo_mass,
                                );
                            state.cached_cargo_capacity =
                                crate::colony::transfer::compute_cargo_capacity(
                                    bp,
                                    part_defs,
                                    state.dv_budget,
                                );
                            state.cached_container_capacity =
                                crate::colony::transfer::blueprint_cargo_container_capacity(
                                    bp, part_defs,
                                );
                            state.cached_fuel_reqs =
                                FleetManager::fuel_requirements_for_blueprint(bp, part_defs);

                            // Crew capacity from blueprint pods
                            let mut crew_cap = 0u32;
                            let mut has_probe = false;
                            for placed in &bp.parts {
                                if let Some(def) = part_defs.get(&placed.definition_id) {
                                    if let Some(ref pod) = def.pod {
                                        crew_cap += pod.crew_capacity;
                                        if pod.can_control && pod.crew_capacity == 0 {
                                            has_probe = true;
                                        }
                                    }
                                }
                            }
                            state.cached_crew_capacity = crew_cap;
                            state.cached_has_probe_core = has_probe;
                        } else {
                            state.cached_ship_dv_empty = 0.0;
                            state.cached_ship_dv_with_cargo = 0.0;
                            state.cached_cargo_capacity = 0.0;
                            state.cached_container_capacity = 0.0;
                            state.cached_fuel_reqs = Vec::new();
                            state.cached_crew_capacity = 0;
                            state.cached_has_probe_core = false;
                        }

                        // Synodic period
                        state.cached_synodic_period =
                            crate::colony::transfer::compute_synodic_period(
                                state.source_body,
                                state.dest_body,
                                solar_system,
                            );

                        // Flight time
                        state.cached_flight_time =
                            crate::colony::transfer::estimate_flight_time(
                                state.source_body,
                                state.dest_body,
                                state.dv_budget,
                                sim_time,
                                solar_system,
                            )
                            .unwrap_or(0.0);
                    }

                    // 5. Transfer Analysis
                    let has_route =
                        state.source_body != state.dest_body || state.source_body.is_some();
                    if has_route && state.cached_leg.is_some() {
                        section_heading(ui, "Transfer Analysis");

                        // Category
                        if let Some(cat) = state.cached_category {
                            let cat_str = match cat {
                                RouteCategory::SameSOI => "Same-SOI",
                                RouteCategory::Interplanetary => "Interplanetary",
                                RouteCategory::Interstellar => "Interstellar",
                            };
                            ui.label(
                                egui::RichText::new(format!("Route type: {}", cat_str))
                                    .size(12.0),
                            );
                        }

                        // Min dv
                        ui.label(
                            egui::RichText::new(format!(
                                "Minimum \u{0394}v (Hohmann): {}",
                                format_dv(state.cached_min_dv)
                            ))
                            .size(12.0)
                            .color(COLOR_GRAY),
                        );

                        // Dv budget
                        ui.horizontal(|ui| {
                            ui.label("\u{0394}v budget:");
                            ui.add(
                                egui::DragValue::new(&mut state.dv_budget)
                                    .clamp_range(state.cached_min_dv..=1_000_000.0)
                                    .speed(100.0)
                                    .suffix(" m/s"),
                            );
                            if ui.small_button("Min").on_hover_text("Set to minimum (Hohmann)").clicked() {
                                state.dv_budget = state.cached_min_dv;
                            }
                        });

                        // Ship dv with cargo
                        if !state.blueprint_name.is_empty() {
                            let cargo_mass: f64 =
                                state.cargo_items.iter().map(|(_, kg)| kg).sum();
                            let ship_dv = if cargo_mass > 0.0 {
                                state.cached_ship_dv_with_cargo
                            } else {
                                state.cached_ship_dv_empty
                            };
                            let sufficient = ship_dv >= state.dv_budget;
                            let color = if sufficient { COLOR_GREEN } else { COLOR_RED };
                            let label = if cargo_mass > 0.0 {
                                format!(
                                    "Ship \u{0394}v (with {:.0} kg cargo): {}",
                                    cargo_mass,
                                    format_dv(ship_dv)
                                )
                            } else {
                                format!("Ship \u{0394}v (empty): {}", format_dv(ship_dv))
                            };
                            ui.label(egui::RichText::new(label).size(12.0).color(color));
                            if !sufficient {
                                ui.label(
                                    egui::RichText::new(
                                        "WARNING: Ship has insufficient \u{0394}v for this route",
                                    )
                                    .size(11.0)
                                    .strong()
                                    .color(COLOR_RED),
                                );
                            }

                            // Cargo capacity
                            ui.label(
                                egui::RichText::new(format!(
                                    "Max cargo capacity: {:.0} kg",
                                    state.cached_cargo_capacity
                                ))
                                .size(12.0)
                                .color(COLOR_GRAY),
                            );
                        }

                        // Travel time
                        if state.cached_flight_time > 0.0 {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Estimated travel time: {}",
                                    format_flight_time(state.cached_flight_time)
                                ))
                                .size(12.0),
                            );
                        }

                        // Transfer window frequency (interplanetary)
                        if let Some(synodic) = state.cached_synodic_period {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Transfer window every {}",
                                    format_flight_time(synodic)
                                ))
                                .size(12.0)
                                .color(COLOR_GRAY),
                            );
                        }

                        // Per-leg breakdown
                        if let Some(ref leg) = state.cached_leg {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Launch: {} | Transfer: {} | Landing: {}",
                                    format_dv(leg.launch_dv),
                                    format_dv(leg.transfer_dv),
                                    format_dv(leg.landing_dv)
                                ))
                                .size(11.0)
                                .color(COLOR_GRAY),
                            );
                        }

                        ui.add_space(8.0);
                    }

                    // 6. Cargo Manifest
                    section_heading(ui, "Cargo Manifest");

                    // Effective capacity: min of dv-limited capacity and physical container capacity
                    let effective_capacity = if state.cached_container_capacity > 0.0 {
                        state.cached_cargo_capacity.min(state.cached_container_capacity)
                    } else {
                        0.0
                    };

                    // Filter out ship fuels from cargo dropdown
                    let cargo_resources: Vec<ResourceType> = ResourceType::all()
                        .iter()
                        .copied()
                        .filter(|rt| !rt.is_ship_fuel())
                        .collect();

                    ui.horizontal(|ui| {
                        let selected_name = cargo_resources
                            .get(state.selected_resource)
                            .map(|r| r.display_name())
                            .unwrap_or("Select...");

                        egui::ComboBox::from_id_source("rcp_cargo_resource")
                            .selected_text(selected_name)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for (i, rt) in cargo_resources.iter().enumerate() {
                                    if ui
                                        .selectable_label(
                                            state.selected_resource == i,
                                            rt.display_name(),
                                        )
                                        .clicked()
                                    {
                                        state.selected_resource = i;
                                    }
                                }
                            });

                        ui.add(
                            egui::DragValue::new(&mut state.cargo_amount)
                                .clamp_range(0.0..=1_000_000.0)
                                .speed(100.0)
                                .suffix(" kg"),
                        );

                        let total_cargo: f64 = state.cargo_items.iter().map(|(_, kg)| kg).sum();
                        let can_add = state.cargo_amount > 0.0
                            && state.selected_resource < cargo_resources.len()
                            && effective_capacity > 0.0
                            && total_cargo + state.cargo_amount <= effective_capacity;

                        if ui.add_enabled(can_add, egui::Button::new("Add")).clicked() {
                            let rt = cargo_resources[state.selected_resource];
                            if let Some(entry) =
                                state.cargo_items.iter_mut().find(|(r, _)| *r == rt)
                            {
                                entry.1 += state.cargo_amount;
                            } else {
                                state.cargo_items.push((rt, state.cargo_amount));
                            }
                        }
                    });

                    let mut remove_idx = None;
                    let total_cargo: f64 = state.cargo_items.iter().map(|(_, kg)| kg).sum();
                    for (i, (rt, amount)) in state.cargo_items.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "  {} - {:.0} kg",
                                rt.display_name(),
                                amount
                            ));
                            if ui.small_button("\u{2716}").clicked() {
                                remove_idx = Some(i);
                            }
                        });
                    }
                    if let Some(idx) = remove_idx {
                        state.cargo_items.remove(idx);
                    }
                    if effective_capacity > 0.0 {
                        let over = total_cargo > effective_capacity;
                        let color = if over { COLOR_RED } else { COLOR_GRAY };
                        ui.label(
                            egui::RichText::new(format!(
                                "Total: {:.0} / {:.0} kg",
                                total_cargo, effective_capacity
                            ))
                            .size(11.0)
                            .color(color),
                        );
                    } else if !state.blueprint_name.is_empty() && state.cached_container_capacity <= 0.0 {
                        ui.label(
                            egui::RichText::new("No cargo containers on this ship")
                                .size(11.0)
                                .color(COLOR_RED),
                        );
                    }
                    ui.add_space(8.0);

                    // 7. Crew
                    section_heading(ui, "Crew");
                    {
                        let min_crew: u32 = if state.cached_has_probe_core { 0 } else { 1 };
                        let max_crew = state.cached_crew_capacity;
                        // Clamp current value
                        if state.crew < min_crew { state.crew = min_crew; }
                        if state.crew > max_crew && max_crew > 0 { state.crew = max_crew; }

                        ui.horizontal(|ui| {
                            ui.label("Crew to send:");
                            ui.add(
                                egui::DragValue::new(&mut state.crew)
                                    .clamp_range(min_crew..=max_crew.max(min_crew)),
                            );
                            ui.label(
                                egui::RichText::new(format!("/ {} seats", max_crew))
                                    .size(11.0)
                                    .color(COLOR_GRAY),
                            );
                        });
                        if !state.cached_has_probe_core && max_crew > 0 {
                            ui.label(
                                egui::RichText::new("Min 1 crew required (no probe core)")
                                    .size(11.0)
                                    .color(COLOR_GRAY),
                            );
                        }
                    }
                    ui.add_space(8.0);

                    // 8. Departure Inventory
                    section_heading(ui, "Departure Inventory");
                    if !state.cached_fuel_reqs.is_empty() {
                        ui.label(
                            egui::RichText::new("Fuel:")
                                .size(12.0)
                                .color(COLOR_GRAY),
                        );
                        for (rt, kg) in &state.cached_fuel_reqs {
                            ui.label(
                                egui::RichText::new(format!(
                                    "  {}: {:.0} kg",
                                    rt.display_name(),
                                    kg
                                ))
                                .size(11.0),
                            );
                        }
                    }
                    if !state.cargo_items.is_empty() {
                        ui.label(
                            egui::RichText::new("Cargo:")
                                .size(12.0)
                                .color(COLOR_GRAY),
                        );
                        for (rt, kg) in &state.cargo_items {
                            ui.label(
                                egui::RichText::new(format!(
                                    "  {}: {:.0} kg",
                                    rt.display_name(),
                                    kg
                                ))
                                .size(11.0),
                            );
                        }
                    }
                    // Food for journey: crew * 0.5 kg/day * flight_days
                    let flight_days = state.cached_flight_time / 86400.0;
                    let journey_food_kg = state.crew as f64 * 0.5 * flight_days;
                    if journey_food_kg > 0.0 {
                        ui.label(
                            egui::RichText::new(format!(
                                "Food (journey): {:.0} kg ({} crew x {:.1} days x 0.5 kg/day)",
                                journey_food_kg, state.crew, flight_days
                            ))
                            .size(11.0),
                        );
                    }
                    if state.crew > 0 {
                        ui.label(
                            egui::RichText::new(format!("Crew: {}", state.crew))
                                .size(11.0),
                        );
                    }

                    // Total launch cost (Earth source only)
                    if state.source_body.is_none() && !state.blueprint_name.is_empty() {
                        ui.add_space(4.0);
                        let mut total_cost = 0.0;
                        // Rocket cost (material breakdown + fuel)
                        let rocket_cost = if let Some(bp) = blueprints.get(&state.blueprint_name) {
                            bp.calculate_cost(part_defs)
                        } else {
                            0.0
                        };
                        total_cost += rocket_cost;
                        // Fuel cost
                        let mut fuel_cost = 0.0;
                        for (rt, kg) in &state.cached_fuel_reqs {
                            if let Some(price) = rt.earth_price() {
                                fuel_cost += price * kg;
                            }
                        }
                        total_cost += fuel_cost;
                        // Cargo cost
                        let mut cargo_cost = 0.0;
                        for (rt, kg) in &state.cargo_items {
                            if let Some(price) = rt.earth_price() {
                                cargo_cost += price * kg;
                            }
                        }
                        total_cost += cargo_cost;
                        // Food cost
                        let mut food_cost = 0.0;
                        if journey_food_kg > 0.0 {
                            let food_price = ResourceType::Food.earth_price().unwrap_or(50.0);
                            food_cost = food_price * journey_food_kg;
                        }
                        total_cost += food_cost;

                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Rocket: {}  Fuel: {}  Cargo: {}  Food: {}",
                                crate::colony::format_money(rocket_cost),
                                crate::colony::format_money(fuel_cost),
                                crate::colony::format_money(cargo_cost),
                                crate::colony::format_money(food_cost),
                            ))
                            .size(11.0)
                            .color(COLOR_GRAY),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Total launch cost: {}",
                                crate::colony::format_money(total_cost)
                            ))
                            .size(12.0)
                            .strong()
                            .color(COLOR_YELLOW),
                        );
                    }
                    ui.add_space(8.0);

                    // 8b. Destination Inventory
                    section_heading(ui, "Destination Inventory");
                    if !state.blueprint_name.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("Ship: {}", state.blueprint_name))
                                .size(11.0),
                        );
                    }
                    // Remaining fuel estimate: fuel_loaded * max(0, 1 - route_dv / ship_dv)
                    if !state.cached_fuel_reqs.is_empty() && state.cached_ship_dv_with_cargo > 0.0 {
                        let ship_dv = if state.cargo_items.is_empty() {
                            state.cached_ship_dv_empty
                        } else {
                            state.cached_ship_dv_with_cargo
                        };
                        let fuel_fraction_remaining = (1.0 - state.dv_budget / ship_dv).max(0.0);
                        ui.label(
                            egui::RichText::new("Remaining fuel (est.):")
                                .size(12.0)
                                .color(COLOR_GRAY),
                        );
                        for (rt, kg) in &state.cached_fuel_reqs {
                            let remaining = kg * fuel_fraction_remaining;
                            ui.label(
                                egui::RichText::new(format!(
                                    "  {}: {:.0} kg",
                                    rt.display_name(),
                                    remaining
                                ))
                                .size(11.0),
                            );
                        }
                    }
                    // Cargo delivered intact
                    if !state.cargo_items.is_empty() {
                        ui.label(
                            egui::RichText::new("Cargo (delivered):")
                                .size(12.0)
                                .color(COLOR_GRAY),
                        );
                        for (rt, kg) in &state.cargo_items {
                            ui.label(
                                egui::RichText::new(format!(
                                    "  {}: {:.0} kg",
                                    rt.display_name(),
                                    kg
                                ))
                                .size(11.0),
                            );
                        }
                    }
                    // Food remaining: 0 (consumed during transit)
                    if journey_food_kg > 0.0 {
                        ui.label(
                            egui::RichText::new("Food remaining: 0 kg (consumed in transit)")
                                .size(11.0)
                                .color(COLOR_GRAY),
                        );
                    }
                    if state.crew > 0 {
                        ui.label(
                            egui::RichText::new(format!("Crew arriving: {}", state.crew))
                                .size(11.0),
                        );
                    }
                    ui.add_space(8.0);

                    // 9. Scheduling
                    section_heading(ui, "Scheduling");
                    if let Some(cat) = state.cached_category {
                        match cat {
                            RouteCategory::Interplanetary => {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::DragValue::new(&mut state.ships_per_window)
                                            .clamp_range(1..=10)
                                            .speed(0.1),
                                    );
                                    ui.label("ship(s) per transfer window");
                                });
                            }
                            RouteCategory::SameSOI | RouteCategory::Interstellar => {
                                ui.horizontal(|ui| {
                                    ui.label("Every");
                                    ui.add(
                                        egui::DragValue::new(&mut state.interval_days)
                                            .clamp_range(1.0..=36500.0)
                                            .speed(1.0),
                                    );
                                    ui.label("days");
                                });
                            }
                        }
                    }
                    ui.add_space(8.0);

                    // 10. Action buttons
                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let has_source_dest = state.source_body != state.dest_body
                            || (state.source_body.is_none() && state.dest_body.is_some())
                            || (state.source_body.is_some() && state.dest_body.is_none());
                        let can_create = !state.blueprint_name.is_empty()
                            && has_source_dest
                            && !state.route_name.is_empty()
                            && state.cached_leg.is_some();

                        let btn_label = if state.editing_route_id.is_some() {
                            "Update Route"
                        } else {
                            "Create Route"
                        };

                        if ui
                            .add_enabled(can_create, egui::Button::new(btn_label))
                            .clicked()
                        {
                            if state.cached_leg.is_some() {
                                let category =
                                    state.cached_category.unwrap_or(RouteCategory::SameSOI);
                                let route = TradeRoute {
                                    id: state.editing_route_id.unwrap_or(0),
                                    name: state.route_name.clone(),
                                    legs: vec![RouteLeg {
                                        from_body: state.source_body,
                                        to_body: state.dest_body,
                                        delta_v: state.dv_budget,
                                        flight_time: state.cached_flight_time,
                                    }],
                                    blueprint_name: state.blueprint_name.clone(),
                                    outbound_cargo: CargoManifest {
                                        items: state.cargo_items.clone(),
                                    },
                                    return_cargo: CargoManifest::default(),
                                    crew: state.crew,
                                    automation: crate::colony::trade::AutomationMode::Manual,
                                    frequency_days: state.interval_days,
                                    dv_threshold: 0.0,
                                    priority: 0,
                                    min_stockpile: 0.0,
                                    paused: false,
                                    last_launch_time: 0.0,
                                    assigned_ship_id: None,
                                    total_delta_v: state.dv_budget,
                                    total_flight_time: state.cached_flight_time,
                                    max_cargo_capacity: state.cached_cargo_capacity,
                                    route_category: category,
                                    interval_days: state.interval_days,
                                    ships_per_window: state.ships_per_window,
                                    alert_reason: None,
                                };

                                if state.editing_route_id.is_some() {
                                    action = TradeAction::EditRoute {
                                        route_id: state.editing_route_id.unwrap(),
                                        route,
                                    };
                                } else {
                                    action = TradeAction::CreateRoute { route };
                                }
                                state.active = false;
                            }
                        }

                        if ui.button("Cancel").clicked() {
                            state.active = false;
                        }
                    });
                });
        });

    action
}

// ============================================================
// Colony Trade Tab (read-only, for per-colony screen)
// ============================================================

/// Render read-only trade information in the colony screen.
pub fn render_colony_trade_section(
    ui: &mut egui::Ui,
    body_index: usize,
    fleet: &FleetManager,
    body_names: &[String],
    earth_index: usize,
) -> TradeAction {
    let action = TradeAction::None;

    section_heading(ui, "Trade Routes");

    // Routes involving this colony
    let body_opt = Some(body_index);
    let routes: Vec<&TradeRoute> = fleet.routes_involving(body_opt);

    if routes.is_empty() {
        ui.label(
            egui::RichText::new("No trade routes for this colony.")
                .size(12.0)
                .color(COLOR_GRAY),
        );
    } else {
        for route in &routes {
            card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let status_color = if route.paused { COLOR_RED } else { COLOR_GREEN };
                    let status_text = if route.paused { "Paused" } else { "Active" };

                    ui.label(
                        egui::RichText::new(&route.name)
                            .size(13.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                    ui.label(
                        egui::RichText::new(format!("[{}]", status_text))
                            .size(11.0)
                            .color(status_color),
                    );

                    let source = body_name_or_earth(
                        FleetManager::route_source(route),
                        body_names,
                        earth_index,
                    );
                    let dest = body_name_or_earth(
                        FleetManager::route_destination(route),
                        body_names,
                        earth_index,
                    );
                    ui.label(
                        egui::RichText::new(format!("{} \u{2192} {}", source, dest))
                            .size(12.0)
                            .color(COLOR_GRAY),
                    );
                });

                // Alert line (if any)
                if let Some(ref alert) = route.alert_reason {
                    ui.label(
                        egui::RichText::new(format!("\u{26a0} {}", alert))
                            .size(11.0)
                            .color(COLOR_RED),
                    );
                }

                // Cargo summary
                if !route.outbound_cargo.is_empty() {
                    let cargo_summary: Vec<String> = route
                        .outbound_cargo
                        .items
                        .iter()
                        .map(|(rt, kg)| format!("{}: {:.0} kg", rt.display_name(), kg))
                        .collect();
                    ui.label(
                        egui::RichText::new(format!("Cargo: {}", cargo_summary.join(", ")))
                            .size(11.0)
                            .color(COLOR_GRAY),
                    );
                }

                // Ship status
                if let Some(ship_id) = route.assigned_ship_id {
                    if let Some(ship) = fleet.get_ship(ship_id) {
                        let status = match ship.state {
                            TradeShipState::Stationed => {
                                let loc =
                                    body_name_or_earth(ship.location, body_names, earth_index);
                                format!("Ship: {} (stationed at {})", ship.name, loc)
                            }
                            TradeShipState::InTransit => {
                                format!(
                                    "Ship: {} (in transit, {} remaining)",
                                    ship.name,
                                    format_flight_time(ship.transit_remaining)
                                )
                            }
                        };
                        ui.label(
                            egui::RichText::new(status)
                                .size(11.0)
                                .color(COLOR_YELLOW),
                        );

                        // No launch button here — managed from Colony Overview
                    }
                }
            });
        }
    }

    // Ships at this colony (read-only)
    let ships_here = fleet.ships_at(Some(body_index));
    if !ships_here.is_empty() {
        ui.add_space(8.0);
        section_heading(ui, "Ships at Colony");
        for ship in ships_here {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&ship.name)
                        .size(12.0)
                        .color(egui::Color32::WHITE),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "({}, {} dv)",
                        ship.blueprint_name,
                        format_dv(ship.cached_delta_v)
                    ))
                    .size(11.0)
                    .color(COLOR_GRAY),
                );
            });
        }
    }

    // "Manage routes in Colony Overview" hint
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Manage routes in Colony Overview")
            .size(11.0)
            .color(COLOR_GRAY)
            .italics(),
    );

    action
}

// ============================================================
// Helpers
// ============================================================

fn body_name_or_earth(
    body: Option<usize>,
    body_names: &[String],
    earth_index: usize,
) -> String {
    match body {
        None => "Earth".to_string(),
        Some(idx) if idx == earth_index => "Earth".to_string(),
        Some(idx) => body_names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("Body {}", idx)),
    }
}
