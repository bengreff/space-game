use crate::parts::{FuelType, PartCategory, PartDefinitions, PartSize};
use super::{EditorState, ShipStats};

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
    NewVessel,
    ExitToFlight,
}

/// Body info for TWR calculation
pub struct BodyInfo {
    pub name: String,
    pub surface_gravity: f64,
}

/// Render the editor UI using egui
pub fn render_editor_ui(
    ctx: &egui::Context,
    editor: &mut EditorState,
    part_defs: &PartDefinitions,
    blueprint_names: &[&str],
    stats: &ShipStats,
    bodies: &[BodyInfo],
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

    // Right panel - Part info and staging
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
                        ui.label(format!("Reaction Wheel: {:.1} kN·m", pod.torque));
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

                            // Fuel type selector buttons
                            ui.horizontal_wrapped(|ui| {
                                for fuel_type in FuelType::all() {
                                    let selected = part.fuel_type == *fuel_type;
                                    if ui.selectable_label(selected, fuel_type.display_name()).clicked() {
                                        if let Some(p) = editor.parts.get_mut(&part_id) {
                                            p.fuel_type = *fuel_type;
                                            // Empty tanks when switching type
                                            if *fuel_type == FuelType::Empty {
                                                p.tank_filled = false;
                                            }
                                        }
                                    }
                                }
                            });

                            // Fill/Empty button (only if fuel type selected)
                            if part.fuel_type != FuelType::Empty {
                                ui.separator();
                                let (ox_cap, fuel_cap) = tank.propellant_capacity(part.fuel_type);
                                let total_prop = ox_cap + fuel_cap;

                                if part.tank_filled {
                                    ui.label(format!("O2: {}", format_mass(ox_cap)));
                                    if let Some(fuel_name) = part.fuel_type.fuel_resource_name() {
                                        ui.label(format!("{}: {}", fuel_name.to_uppercase(), format_mass(fuel_cap)));
                                    }
                                    ui.label(format!("Total: {}", format_mass(total_prop)));

                                    if ui.button("Empty Tank").clicked() {
                                        if let Some(p) = editor.parts.get_mut(&part_id) {
                                            p.tank_filled = false;
                                        }
                                    }
                                } else {
                                    ui.label("Tank is empty");
                                    ui.label(format!("Capacity: {}", format_mass(total_prop)));

                                    if ui.button("Fill Tank").clicked() {
                                        if let Some(p) = editor.parts.get_mut(&part_id) {
                                            p.tank_filled = true;
                                        }
                                    }
                                }
                            }

                            // Show total mass
                            ui.separator();
                            let dry_mass = def.mass;
                            let prop_mass = if part.tank_filled && part.fuel_type != FuelType::Empty {
                                let (ox, fuel) = tank.propellant_capacity(part.fuel_type);
                                (ox + fuel) / 1000.0  // Convert kg to tonnes
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
                            ui.label(format!("Reaction Wheel: {:.1} kN·m", pod.torque));
                        }

                        ui.separator();

                        if ui.button("Delete Part").clicked() {
                            editor.part_to_delete = Some(part_id);
                        }
                    }
                }
            }
            // No selection - show staging
            else {
                ui.heading("Staging");
                ui.separator();

                if editor.stages.is_empty() {
                    ui.label("No stages defined");
                    ui.label("(Staging coming soon)");
                } else {
                    for (i, stage) in editor.stages.iter().enumerate() {
                        ui.label(format!("Stage {}: {} parts", i, stage.len()));
                    }
                }

                ui.separator();
                ui.label("Select a part to see details");
            }
        });

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
                            if ui.button(*name).clicked() {
                                action = EditorAction::LoadBlueprint(name.to_string());
                                editor.show_load_dialog = false;
                            }
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
