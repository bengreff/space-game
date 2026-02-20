use crate::parts::{FuelType, PartCategory, PartDefinitions, PartSize, PlacedPartId};
use super::{EditorState, ShipStats};

/// Drag payload for staging panel: either a part or a whole stage being reordered
#[derive(Clone, Copy)]
enum StagingDrag {
    Part(PlacedPartId),
    Stage(usize),
}

/// Format mass - shows tonnes if >= 1000 kg, otherwise kg
fn format_mass(kg: f64) -> String {
    if kg >= 1000.0 {
        format!("{:.2} t", kg / 1000.0)
    } else {
        format!("{:.0} kg", kg)
    }
}

/// Editor UI action that should be handled by the game
#[derive(Debug, Clone)]
pub enum EditorAction {
    None,
    Launch,
    SaveBlueprint(String),
    LoadBlueprint(String),
    DeleteBlueprint(String),
    NewVessel,
    ExitToFlight,
}

/// Body info for TWR calculation
pub struct BodyInfo {
    pub name: String,
    pub surface_gravity: f64,
}

/// Format delta-v for display
fn format_delta_v(dv: f64) -> String {
    if dv >= 1000.0 {
        format!("{:.1} km/s", dv / 1000.0)
    } else {
        format!("{:.0} m/s", dv)
    }
}

/// Render the editor UI using egui
pub fn render_editor_ui(
    ctx: &egui::Context,
    editor: &mut EditorState,
    part_defs: &PartDefinitions,
    blueprint_names: &[&str],
    stats: &ShipStats,
    bodies: &[BodyInfo],
    stage_delta_vs: &[f64],
) -> EditorAction {
    let mut action = EditorAction::None;

    // Top toolbar
    egui::TopBottomPanel::top("editor_toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Vehicle Editor");
            ui.separator();

            // New button
            if ui.button("New").clicked() {
                action = EditorAction::NewVessel;
            }

            // Save button
            if ui.button("Save").clicked() {
                editor.show_save_dialog = true;
            }

            // Load button
            if ui.button("Load").clicked() {
                editor.show_load_dialog = true;
            }

            ui.separator();

            // Symmetry mode
            ui.label("Symmetry:");
            if ui.button(editor.symmetry_mode.display()).clicked() {
                editor.symmetry_mode = editor.symmetry_mode.cycle_next();
            }

            ui.separator();

            // Launch button
            let can_launch = editor.can_launch();
            ui.add_enabled_ui(can_launch, |ui| {
                if ui.button("🚀 Launch").clicked() {
                    action = EditorAction::Launch;
                }
            });

            // Exit to flight (without launching)
            ui.separator();
            if ui.button("Exit to Flight").clicked() {
                action = EditorAction::ExitToFlight;
            }

            // Part count
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Parts: {}", editor.part_count()));
            });
        });
    });

    // Stats bar (below toolbar)
    egui::TopBottomPanel::top("stats_bar")
        .frame(egui::Frame::none()
            .fill(egui::Color32::from_rgba_unmultiplied(25, 30, 40, 240))
            .inner_margin(egui::Margin::symmetric(8.0, 6.0)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Large stats: Mass, Thrust, TWR
                ui.style_mut().override_text_style = Some(egui::TextStyle::Heading);

                // Mass
                ui.label(format!("Mass: {:.2} t", stats.wet_mass));
                ui.separator();

                // Thrust
                let thrust = if editor.twr_settings.show_asl {
                    stats.thrust_asl
                } else {
                    stats.thrust_vac
                };
                ui.label(format!("Thrust: {:.1} kN", thrust));
                ui.separator();

                // TWR with body selector
                let gravity = bodies.get(editor.twr_settings.body_index)
                    .map(|b| b.surface_gravity)
                    .unwrap_or(9.81);
                let twr = if editor.twr_settings.show_asl {
                    stats.twr_asl(gravity)
                } else {
                    stats.twr_vac(gravity)
                };
                ui.label(format!("TWR: {:.2}", twr));

                // Body dropdown
                ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                let current_body = bodies.get(editor.twr_settings.body_index)
                    .map(|b| b.name.as_str())
                    .unwrap_or("Unknown");

                egui::ComboBox::from_id_source("twr_body")
                    .selected_text(current_body)
                    .show_ui(ui, |ui: &mut egui::Ui| {
                        for (i, body) in bodies.iter().enumerate() {
                            ui.selectable_value(
                                &mut editor.twr_settings.body_index,
                                i,
                                &body.name,
                            );
                        }
                    });

                // ASL/Vacuum toggle
                let atmo_label = if editor.twr_settings.show_asl { "ASL" } else { "Vac" };
                if ui.button(atmo_label).clicked() {
                    editor.twr_settings.show_asl = !editor.twr_settings.show_asl;
                }

                // Delta-v
                let total_dv: f64 = stage_delta_vs.iter().sum();
                if total_dv > 0.0 {
                    ui.separator();
                    ui.label(format!("Δv: {}", format_delta_v(total_dv)));
                }

                ui.separator();

                // Resources (smaller text)
                ui.style_mut().override_text_style = Some(egui::TextStyle::Body);

                // Show resources in a consistent order
                if let Some(ox) = stats.resources.get("oxygen") {
                    ui.label(format!("O2: {}", format_mass(ox.current)));
                }
                if let Some(rp1) = stats.resources.get("rp1") {
                    ui.label(format!("RP1: {}", format_mass(rp1.current)));
                }
                if let Some(ch4) = stats.resources.get("methane") {
                    ui.label(format!("CH4: {}", format_mass(ch4.current)));
                }
                if let Some(lh2) = stats.resources.get("hydrogen") {
                    ui.label(format!("LH2: {}", format_mass(lh2.current)));
                }
                if let Some(mp) = stats.resources.get("monopropellant") {
                    ui.label(format!("MP: {}", format_mass(mp.current)));
                }
            });
        });

    // Left panel - Parts palette
    egui::SidePanel::left("parts_palette")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Parts");
            ui.separator();

            // Category tabs
            ui.horizontal_wrapped(|ui| {
                for category in PartCategory::all() {
                    let selected = editor.selected_category == *category;
                    if ui.selectable_label(selected, category.display_name()).clicked() {
                        editor.selected_category = *category;
                    }
                }
            });

            ui.separator();

            // Parts list for selected category, grouped by size
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut any_parts = false;

                for size in PartSize::all() {
                    if !part_defs.has_parts_for_size(editor.selected_category, *size) {
                        continue;
                    }
                    any_parts = true;

                    let parts = part_defs.by_category_and_size(editor.selected_category, *size);

                    // Collapsible header for each size
                    egui::CollapsingHeader::new(size.display_name())
                        .default_open(true)
                        .show(ui, |ui| {
                            for part in parts {
                                let is_selected = editor.selected_part_def.as_ref() == Some(&part.id);

                                if ui.selectable_label(is_selected, &part.name).clicked() {
                                    if is_selected {
                                        editor.deselect();
                                    } else {
                                        editor.select_part_def(&part.id);
                                    }
                                }
                            }
                        });
                }

                if !any_parts {
                    ui.label("No parts in this category");
                }
            });
        });

    // Right panel - Staging (always visible, far right)
    egui::SidePanel::right("staging_panel")
        .default_width(150.0)
        .show(ctx, |ui| {
            ui.heading("Staging");

            // Total Δv
            let total_dv: f64 = stage_delta_vs.iter().sum();
            if total_dv > 0.0 {
                ui.label(egui::RichText::new(format!("Total Δv: {}", format_delta_v(total_dv)))
                    .size(12.0).strong());
            }

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Track deferred actions
                let mut insert_stage_at: Option<usize> = None;
                let mut move_stage_to: Option<(usize, usize)> = None; // (from, to_insert_pos)
                let mut delete_stage_at: Option<usize> = None;
                let mut drop_action: Option<(StagingDrag, usize)> = None;
                let mut staging_select: Option<PlacedPartId> = None;

                // Helper: render a "+" gap that is also a drop zone for stage reordering.
                // `insert_pos` is where a new/moved stage would be inserted.
                let plus_gap = |ui: &mut egui::Ui, insert_pos: usize,
                                     insert_out: &mut Option<usize>,
                                     move_out: &mut Option<(usize, usize)>| {
                    let frame = egui::Frame::none();
                    let (inner, dropped) = ui.dnd_drop_zone::<StagingDrag, ()>(frame, |ui| {
                        if ui.small_button("+").on_hover_text("Insert stage here").clicked() {
                            *insert_out = Some(insert_pos);
                        }
                    });
                    // Highlight when a stage is hovering
                    if inner.response.hovered() && egui::DragAndDrop::has_any_payload(ui.ctx()) {
                        inner.response.highlight();
                    }
                    if let Some(payload) = dropped {
                        if let StagingDrag::Stage(from_idx) = *payload {
                            *move_out = Some((from_idx, insert_pos));
                        }
                    }
                };

                // Render stages in reverse order (highest stage number at top)
                for stage_idx in (0..editor.stages.len()).rev() {
                    // "+" gap above this stage
                    plus_gap(ui, stage_idx + 1, &mut insert_stage_at, &mut move_stage_to);

                    let frame = egui::Frame::group(ui.style());
                    let (_, dropped_payload) = ui.dnd_drop_zone::<StagingDrag, ()>(frame, |ui| {
                        ui.horizontal(|ui| {
                            let stage_drag_id = egui::Id::new(("staging_stage", stage_idx));
                            ui.dnd_drag_source(stage_drag_id, StagingDrag::Stage(stage_idx), |ui| {
                                ui.label(format!("Stage {}", stage_idx + 1));
                            });
                            // Per-stage Δv
                            let stage_dv = stage_delta_vs.get(stage_idx).copied().unwrap_or(0.0);
                            if stage_dv > 0.0 {
                                ui.label(egui::RichText::new(format_delta_v(stage_dv))
                                    .size(10.0).color(egui::Color32::from_rgb(120, 200, 120)));
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("\u{2715}").on_hover_text("Delete stage").clicked() {
                                    delete_stage_at = Some(stage_idx);
                                }
                            });
                        });

                        if editor.stages[stage_idx].is_empty() {
                            ui.weak("(empty)");
                        }

                        // Track which partner IDs have been rendered to avoid duplicates
                        let mut rendered_partners: Vec<PlacedPartId> = Vec::new();

                        for &part_id in &editor.stages[stage_idx] {
                            // Skip if this part was already rendered as part of a mirrored pair
                            if rendered_partners.contains(&part_id) {
                                continue;
                            }

                            let part = editor.parts.get(&part_id);
                            let name = part
                                .and_then(|p| part_defs.get(&p.definition_id))
                                .map(|d| d.name.clone())
                                .unwrap_or_else(|| format!("Part {}", part_id));

                            // Check if this part has a mirror partner in the same stage
                            let mirror_in_same_stage = part
                                .and_then(|p| p.mirror_partner)
                                .filter(|mid| editor.stages[stage_idx].contains(mid));

                            let display_name = if let Some(mid) = mirror_in_same_stage {
                                rendered_partners.push(mid);
                                format!("{} x2", name)
                            } else {
                                name
                            };

                            let is_selected = editor.selected_placed_part == Some(part_id)
                                || mirror_in_same_stage.is_some()
                                    && editor.selected_placed_part == mirror_in_same_stage;

                            let item_id = egui::Id::new(("staging_item", part_id));
                            let resp = ui.dnd_drag_source(item_id, StagingDrag::Part(part_id), |ui| {
                                let text = egui::RichText::new(&display_name);
                                let text = if is_selected {
                                    text.color(egui::Color32::from_rgb(128, 179, 255))
                                } else {
                                    text
                                };
                                ui.label(text);
                            });
                            if resp.response.clicked() || resp.response.drag_started() {
                                staging_select = Some(part_id);
                            }
                        }
                    });

                    if let Some(payload) = dropped_payload {
                        drop_action = Some((*payload, stage_idx));
                    }
                }

                // "+" gap below the bottom stage
                plus_gap(ui, 0, &mut insert_stage_at, &mut move_stage_to);

                // Apply deferred actions (only one action per frame)
                if let Some((from_idx, to_pos)) = move_stage_to {
                    if from_idx < editor.stages.len() {
                        let stage = editor.stages.remove(from_idx);
                        // Adjust insertion index after removal
                        let insert_at = if from_idx < to_pos {
                            (to_pos - 1).min(editor.stages.len())
                        } else {
                            to_pos.min(editor.stages.len())
                        };
                        editor.stages.insert(insert_at, stage);
                    }
                } else if let Some(idx) = insert_stage_at {
                    editor.stages.insert(idx, Vec::new());
                } else if let Some(idx) = delete_stage_at {
                    editor.stages.remove(idx);
                } else if let Some((drag, target_idx)) = drop_action {
                    match drag {
                        StagingDrag::Part(part_id) => {
                            // Also move mirror partner if linked
                            let mirror_id = editor.parts.get(&part_id).and_then(|p| p.mirror_partner);
                            for stage in &mut editor.stages {
                                stage.retain(|&id| id != part_id && Some(id) != mirror_id);
                            }
                            if target_idx < editor.stages.len() {
                                editor.stages[target_idx].push(part_id);
                                if let Some(mid) = mirror_id {
                                    editor.stages[target_idx].push(mid);
                                }
                            }
                        }
                        StagingDrag::Stage(from_idx) => {
                            if from_idx != target_idx && from_idx < editor.stages.len() {
                                let stage = editor.stages.remove(from_idx);
                                let insert_at = if from_idx < target_idx {
                                    (target_idx - 1).min(editor.stages.len())
                                } else {
                                    target_idx.min(editor.stages.len())
                                };
                                editor.stages.insert(insert_at, stage);
                            }
                        }
                    }
                }

                // Apply staging → grid selection
                if let Some(part_id) = staging_select {
                    editor.selected_placed_part = Some(part_id);
                    editor.selected_part_def = None;
                    editor.ghost_position = None;
                    editor.ghost_valid = false;
                    editor.mirror_ghost_position = None;
                    editor.mirror_ghost_def_id = None;
                }
            });
        });

    // Right panel - Part info (only when a part is selected, renders left of staging)
    let show_info_panel = editor.selected_part_def.is_some() || editor.selected_placed_part.is_some();
    if show_info_panel {
        egui::SidePanel::right("info_panel")
            .default_width(200.0)
            .show(ctx, |ui| {
                // Show part definition info when selected from palette
                if let Some(ref def_id) = editor.selected_part_def {
                    if let Some(def) = part_defs.get(def_id) {
                        ui.heading(&def.name);
                        ui.label(&def.description);

                        ui.separator();
                        ui.label(format!("Size: {}", def.size.display_name()));
                        ui.label(format!("Mass: {:.3} t ({:.0} kg)", def.mass, def.mass * 1000.0));
                        ui.label(format!("Cost: ${}", def.cost));
                        ui.label(format!("Dimensions: {}x{} grid", def.grid_width, def.grid_height));

                        // Engine info
                        if let Some(ref engine) = def.engine {
                            ui.separator();
                            ui.heading("Engine Stats");

                            ui.label(format!("Propellant: {}", engine.propellant.display_name()));

                            ui.label("Thrust:");
                            ui.indent("thrust_indent", |ui| {
                                ui.label(format!("Vacuum: {:.1} kN", engine.thrust_vac));
                                ui.label(format!("Sea Level: {:.1} kN", engine.thrust_asl));
                            });

                            ui.label("Specific Impulse:");
                            ui.indent("isp_indent", |ui| {
                                ui.label(format!("Vacuum: {:.0} s", engine.isp_vac));
                                ui.label(format!("Sea Level: {:.0} s", engine.isp_asl));
                            });

                            ui.label("Gimbal:");
                            ui.indent("gimbal_indent", |ui| {
                                if engine.gimbal_range > 0.0 {
                                    ui.label(format!("Range: ±{:.1}°", engine.gimbal_range));
                                } else {
                                    ui.label("Fixed (no gimbal)");
                                }
                            });

                            if engine.throttleable {
                                ui.label("Throttleable: Yes");
                            } else {
                                ui.label("Throttleable: No");
                            }

                            // TWR calculation for this engine alone
                            ui.separator();
                            ui.label("Single Engine TWR:");
                            let engine_twr = engine.thrust_vac / (def.mass * 9.81);
                            ui.label(format!("  {:.1} (vacuum, Earth)", engine_twr));
                        }

                        // Tank info
                        if let Some(ref tank) = def.tank {
                            ui.separator();
                            ui.heading("Tank Stats");
                            ui.label(format!("Dry Mass: {:.0} kg", tank.dry_mass_kg()));
                            ui.label(format!("Grid Area: {} squares", tank.grid_area));

                            ui.separator();
                            ui.label("Propellant Capacity:");

                            let (ox, fuel) = tank.propellant_capacity(FuelType::Rp1);
                            ui.label(format!("  RP-1: {}", format_mass(ox + fuel)));

                            let (ox, fuel) = tank.propellant_capacity(FuelType::Methane);
                            ui.label(format!("  CH4: {}", format_mass(ox + fuel)));

                            let (ox, fuel) = tank.propellant_capacity(FuelType::Hydrogen);
                            ui.label(format!("  LH2: {}", format_mass(ox + fuel)));
                        }

                        // Pod info
                        if let Some(ref pod) = def.pod {
                            ui.separator();
                            ui.heading("Pod Stats");
                            ui.label(format!("Crew Capacity: {}", pod.crew_capacity));
                        }

                        // RCS info
                        if let Some(ref rcs) = def.rcs {
                            ui.separator();
                            ui.heading("RCS Stats");
                            ui.label(format!("Thrust: {:.1} kN", rcs.thrust));
                            ui.label(format!("Isp: {:.0} s", rcs.isp));
                            ui.label("Fuel: Monopropellant");
                        }
                    }
                }
                // Show placed part info when selected
                else if let Some(part_id) = editor.selected_placed_part {
                    if let Some(part) = editor.parts.get(&part_id).cloned() {
                        if let Some(def) = part_defs.get(&part.definition_id) {
                            ui.heading(&def.name);
                            ui.label(&def.description);

                            ui.separator();
                            ui.label(format!("Size: {}", def.size.display_name()));
                            ui.label(format!("Mass: {:.3} t ({:.0} kg)", def.mass, def.mass * 1000.0));
                            ui.label(format!("Cost: ${}", def.cost));
                            ui.label(format!("Dimensions: {}x{} grid", def.grid_width, def.grid_height));

                            // Engine info
                            if let Some(ref engine) = def.engine {
                                ui.separator();
                                ui.heading("Engine Stats");

                                ui.label(format!("Propellant: {}", engine.propellant.display_name()));

                                ui.label("Thrust:");
                                ui.indent("placed_thrust_indent", |ui| {
                                    ui.label(format!("Vacuum: {:.1} kN", engine.thrust_vac));
                                    ui.label(format!("Sea Level: {:.1} kN", engine.thrust_asl));
                                });

                                ui.label("Specific Impulse:");
                                ui.indent("placed_isp_indent", |ui| {
                                    ui.label(format!("Vacuum: {:.0} s", engine.isp_vac));
                                    ui.label(format!("Sea Level: {:.0} s", engine.isp_asl));
                                });

                                ui.label("Gimbal:");
                                ui.indent("placed_gimbal_indent", |ui| {
                                    if engine.gimbal_range > 0.0 {
                                        ui.label(format!("Range: ±{:.1}°", engine.gimbal_range));
                                    } else {
                                        ui.label("Fixed (no gimbal)");
                                    }
                                });

                                if engine.throttleable {
                                    ui.label("Throttleable: Yes");
                                } else {
                                    ui.label("Throttleable: No");
                                }

                                // TWR calculation for this engine alone
                                ui.separator();
                                ui.label("Single Engine TWR:");
                                let engine_twr = engine.thrust_vac / (def.mass * 9.81);
                                ui.label(format!("  {:.1} (vacuum, Earth)", engine_twr));
                            }

                            // Tank info with controls
                            if let Some(ref tank) = def.tank {
                                ui.separator();
                                ui.heading("Tank Stats");
                                ui.label(format!("Dry Mass: {:.0} kg", tank.dry_mass_kg()));
                                ui.label(format!("Grid Area: {} squares", tank.grid_area));

                                ui.separator();
                                ui.label("Fuel Type:");

                                let mirror_id = part.mirror_partner;

                                // Fuel type selector buttons
                                ui.horizontal_wrapped(|ui| {
                                    for fuel_type in FuelType::all() {
                                        let selected = part.fuel_type == *fuel_type;
                                        if ui.selectable_label(selected, fuel_type.display_name()).clicked() {
                                            let new_fill = if *fuel_type != FuelType::Empty { 1.0 } else { 0.0 };
                                            if let Some(p) = editor.parts.get_mut(&part_id) {
                                                p.fuel_type = *fuel_type;
                                                p.fill_fraction = new_fill;
                                            }
                                            // Apply to mirror partner
                                            if let Some(mid) = mirror_id {
                                                if let Some(mp) = editor.parts.get_mut(&mid) {
                                                    mp.fuel_type = *fuel_type;
                                                    mp.fill_fraction = new_fill;
                                                }
                                            }
                                        }
                                    }
                                });

                                // Draggable fuel bars (only if fuel type selected)
                                if part.fuel_type != FuelType::Empty {
                                    ui.separator();
                                    let (ox_cap, fuel_cap) = tank.propellant_capacity(part.fuel_type);
                                    let frac = part.fill_fraction as f32;
                                    let mut new_frac: Option<f64> = None;

                                    // Oxidizer bar (only for fuels that have oxidizer)
                                    if ox_cap > 0.0 {
                                        let ox_current = format_mass(ox_cap * part.fill_fraction);
                                        let ox_total = format_mass(ox_cap);
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(ui.available_width(), 20.0),
                                            egui::Sense::click_and_drag(),
                                        );
                                        if ui.is_rect_visible(rect) {
                                            let painter = ui.painter();
                                            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(40, 50, 60));
                                            let fill_rect = egui::Rect::from_min_max(
                                                rect.min,
                                                egui::pos2(rect.min.x + rect.width() * frac, rect.max.y),
                                            );
                                            painter.rect_filled(fill_rect, 2.0, egui::Color32::from_rgb(80, 140, 200));
                                            painter.text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                format!("O2: {}/{}", ox_current, ox_total),
                                                egui::FontId::proportional(12.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                        if response.clicked() || response.dragged() {
                                            if let Some(pos) = response.interact_pointer_pos() {
                                                let f = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0) as f64;
                                                new_frac = Some(f);
                                            }
                                        }
                                    }

                                    // Fuel bar
                                    if let Some(fuel_name) = part.fuel_type.fuel_resource_name() {
                                        let fuel_current = format_mass(fuel_cap * part.fill_fraction);
                                        let fuel_total = format_mass(fuel_cap);
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(ui.available_width(), 20.0),
                                            egui::Sense::click_and_drag(),
                                        );
                                        if ui.is_rect_visible(rect) {
                                            let painter = ui.painter();
                                            painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(40, 50, 60));
                                            let fill_rect = egui::Rect::from_min_max(
                                                rect.min,
                                                egui::pos2(rect.min.x + rect.width() * frac, rect.max.y),
                                            );
                                            painter.rect_filled(fill_rect, 2.0, egui::Color32::from_rgb(200, 160, 60));
                                            painter.text(
                                                rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                format!("{}: {}/{}", fuel_name.to_uppercase(), fuel_current, fuel_total),
                                                egui::FontId::proportional(12.0),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                        if response.clicked() || response.dragged() {
                                            if let Some(pos) = response.interact_pointer_pos() {
                                                let f = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0) as f64;
                                                new_frac = Some(f);
                                            }
                                        }
                                    }

                                    // Apply dragged fill fraction
                                    if let Some(f) = new_frac {
                                        if let Some(p) = editor.parts.get_mut(&part_id) {
                                            p.fill_fraction = f;
                                        }
                                        if let Some(mid) = mirror_id {
                                            if let Some(mp) = editor.parts.get_mut(&mid) {
                                                mp.fill_fraction = f;
                                            }
                                        }
                                    }

                                    // Fill/Empty convenience buttons
                                    ui.horizontal(|ui| {
                                        if ui.button("Fill").clicked() {
                                            if let Some(p) = editor.parts.get_mut(&part_id) {
                                                p.fill_fraction = 1.0;
                                            }
                                            if let Some(mid) = mirror_id {
                                                if let Some(mp) = editor.parts.get_mut(&mid) {
                                                    mp.fill_fraction = 1.0;
                                                }
                                            }
                                        }
                                        if ui.button("Empty").clicked() {
                                            if let Some(p) = editor.parts.get_mut(&part_id) {
                                                p.fill_fraction = 0.0;
                                            }
                                            if let Some(mid) = mirror_id {
                                                if let Some(mp) = editor.parts.get_mut(&mid) {
                                                    mp.fill_fraction = 0.0;
                                                }
                                            }
                                        }
                                    });
                                }

                                // Show total mass
                                ui.separator();
                                let dry_mass = def.mass;
                                let prop_mass = if part.fill_fraction > 0.0 && part.fuel_type != FuelType::Empty {
                                    let (ox, fuel) = tank.propellant_capacity(part.fuel_type);
                                    (ox + fuel) * part.fill_fraction / 1000.0  // Convert kg to tonnes
                                } else {
                                    0.0
                                };
                                ui.label(format!("Dry: {:.3} t", dry_mass));
                                ui.label(format!("Prop: {:.3} t", prop_mass));
                                ui.label(format!("Total: {:.3} t", dry_mass + prop_mass));
                            }

                            // Pod info
                            if let Some(ref pod) = def.pod {
                                ui.separator();
                                ui.heading("Pod Stats");
                                ui.label(format!("Crew Capacity: {}", pod.crew_capacity));
                            }

                            // RCS info
                            if let Some(ref rcs) = def.rcs {
                                ui.separator();
                                ui.heading("RCS Stats");
                                ui.label(format!("Thrust: {:.1} kN", rcs.thrust));
                                ui.label(format!("Isp: {:.0} s", rcs.isp));
                                ui.label("Fuel: Monopropellant");
                            }

                            // Decoupler info
                            if def.decoupler.is_some() {
                                ui.separator();
                                ui.heading("Decoupler");
                                let mut crossfeed = part.crossfeed_enabled;
                                if ui.checkbox(&mut crossfeed, "Fuel Crossfeed").changed() {
                                    if let Some(p) = editor.parts.get_mut(&part_id) {
                                        p.crossfeed_enabled = crossfeed;
                                    }
                                    if let Some(mid) = part.mirror_partner {
                                        if let Some(mp) = editor.parts.get_mut(&mid) {
                                            mp.crossfeed_enabled = crossfeed;
                                        }
                                    }
                                }
                            }

                            ui.separator();

                            let delete_label = if part.mirror_partner.is_some() {
                                "Delete Parts (x2)"
                            } else {
                                "Delete Part"
                            };
                            if ui.button(delete_label).clicked() {
                                editor.part_to_delete = Some(part_id);
                            }
                        }
                    }
                }
            });
    }

    // Bottom panel - Instructions
    egui::TopBottomPanel::bottom("editor_instructions")
        .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(20, 20, 30, 200)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Click part to select • Click build area to place • Right-click to deselect • Scroll to zoom • Drag to pan");
            });
        });

    // Save dialog
    if editor.show_save_dialog {
        egui::Window::new("Save Blueprint")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut editor.vessel_name);
                });

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        action = EditorAction::SaveBlueprint(editor.vessel_name.clone());
                        editor.show_save_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        editor.show_save_dialog = false;
                    }
                });
            });
    }

    // Load dialog
    if editor.show_load_dialog {
        egui::Window::new("Load Blueprint")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if blueprint_names.is_empty() {
                    ui.label("No saved blueprints");
                } else {
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for name in blueprint_names {
                            ui.horizontal(|ui| {
                                if ui.button(*name).clicked() {
                                    action = EditorAction::LoadBlueprint(name.to_string());
                                    editor.show_load_dialog = false;
                                }
                                if ui.small_button("🗑").on_hover_text("Delete").clicked() {
                                    action = EditorAction::DeleteBlueprint(name.to_string());
                                }
                            });
                        }
                    });
                }

                if ui.button("Cancel").clicked() {
                    editor.show_load_dialog = false;
                }
            });
    }

    action
}

/// Check if the mouse is over any UI element
pub fn is_mouse_over_ui(ctx: &egui::Context) -> bool {
    ctx.is_pointer_over_area()
}
