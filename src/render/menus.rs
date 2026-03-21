use egui_wgpu::ScreenDescriptor;

use super::formatting::{format_duration, format_distance, format_mass, format_pressure};
use super::types::{
    BodyInfoData, MainMenuAction, PauseAction, TitleScreenAction, TrackingStationAction,
    TrackingVesselData,
};
use super::state::RenderState;

impl RenderState {
    /// Render the main menu: planets + centered menu buttons + time warp panel.
    /// Returns a MainMenuAction if the user clicked a button.
    pub fn render_main_menu(
        &mut self,
        warp_levels: &[f64],
        current_warp_index: usize,
        date_str: &str,
        egui_callback: impl FnOnce(&egui::Context) -> MainMenuAction,
    ) -> Result<(usize, MainMenuAction), wgpu::SurfaceError> {
        self.update_camera_buffer();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut menu_action = MainMenuAction::None;
        let mut new_warp_index = current_warp_index;

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Time warp panel at top
            egui::TopBottomPanel::top("time_warp_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
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
                            new_warp_index = i;
                        }
                    }
                    ui.separator();
                    let current_warp = warp_levels[current_warp_index];
                    ui.label(format!("Current: {}x", current_warp as i64));

                    ui.separator();
                    ui.label(date_str);
                });
            });

            menu_action = egui_callback(ctx);
        });

        self.egui_state.handle_platform_output(&self.window, full_output.platform_output);
        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Main Menu Render Encoder"),
        });
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_descriptor);

        // Geometry pass (planets/orbits already in self.vertex_buffer)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Menu Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.body_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.sprite_atlas.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Egui pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Menu Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.egui_renderer.render(&mut render_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok((new_warp_index, menu_action))
    }

    /// Render the title screen: planets + egui overlay (no time warp bar).
    pub fn render_title_screen(
        &mut self,
        egui_callback: impl FnOnce(&egui::Context) -> TitleScreenAction,
    ) -> Result<TitleScreenAction, wgpu::SurfaceError> {
        self.update_camera_buffer();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut action = TitleScreenAction::None;

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // No time warp panel — just the callback
            action = egui_callback(ctx);
        });

        self.egui_state.handle_platform_output(&self.window, full_output.platform_output);
        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Title Screen Render Encoder"),
        });
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_descriptor);

        // Geometry pass (planets/orbits already in self.vertex_buffer)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Title Screen Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.body_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.sprite_atlas.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Egui pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Title Screen Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.egui_renderer.render(&mut render_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(action)
    }

    /// Render tracking station: planets + time warp panel + vessel list.
    /// Returns the new warp index, pause action, and tracking station action.
    pub fn render_tracking_station(
        &mut self,
        body_names: &[String],
        warp_levels: &[f64],
        current_warp_index: usize,
        paused: bool,
        date_str: &str,
        vessels: &[TrackingVesselData],
        active_vessel_id: u64,
        body_info: &[BodyInfoData],
    ) -> Result<(usize, PauseAction, TrackingStationAction), wgpu::SurfaceError> {
        self.update_camera_buffer();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut new_warp_index = current_warp_index;
        let mut pause_action = PauseAction::None;
        let mut ts_action = TrackingStationAction::None;
        let hovered = self.hovered_body;
        let bodies_copy = self.bodies.clone();
        let size = self.size;
        let camera_pos = self.camera.position;
        let camera_zoom = self.camera.zoom;
        let camera_rotation = self.camera.rotation;
        let aspect_ratio = self.camera.aspect_ratio;
        let scale_factor = self.window.scale_factor() as f32;

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Time warp panel at top
            egui::TopBottomPanel::top("time_warp_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
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
                            new_warp_index = i;
                        }
                    }
                    ui.separator();
                    let current_warp = warp_levels[current_warp_index];
                    ui.label(format!("Current: {}x", current_warp as i64));

                    ui.separator();
                    ui.label(date_str);
                });
            });

            // Vessels sidebar
            if !vessels.is_empty() {
                egui::SidePanel::left("vessels_panel")
                    .default_width(180.0)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.heading(egui::RichText::new("Vessels").size(16.0).color(egui::Color32::WHITE));
                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for vessel in vessels {
                                let is_active = vessel.id == active_vessel_id;
                                let body_name = body_names.get(vessel.soi_body)
                                    .cloned()
                                    .unwrap_or_else(|| "Unknown".to_string());

                                ui.horizontal(|ui| {
                                    // Color indicator
                                    let color = egui::Color32::from_rgba_unmultiplied(
                                        (vessel.color[0] * 255.0) as u8,
                                        (vessel.color[1] * 255.0) as u8,
                                        (vessel.color[2] * 255.0) as u8,
                                        255,
                                    );
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(8.0, 8.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().circle_filled(rect.center(), 4.0, color);

                                    ui.vertical(|ui| {
                                        let name_text = if is_active {
                                            egui::RichText::new(&vessel.name)
                                                .color(egui::Color32::from_rgb(100, 255, 100))
                                                .size(13.0)
                                        } else if vessel.is_debris {
                                            egui::RichText::new(&vessel.name)
                                                .color(egui::Color32::from_rgb(140, 140, 140))
                                                .size(13.0)
                                        } else {
                                            egui::RichText::new(&vessel.name)
                                                .color(egui::Color32::WHITE)
                                                .size(13.0)
                                        };
                                        if ui.add(egui::Label::new(name_text).sense(egui::Sense::click())).clicked() {
                                            ts_action = TrackingStationAction::FocusVessel(vessel.id);
                                        }
                                        ui.label(egui::RichText::new(format!("SOI: {}", body_name))
                                            .size(11.0)
                                            .color(egui::Color32::GRAY));
                                    });

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if !vessel.is_debris {
                                            if ui.small_button("Fly").clicked() {
                                                ts_action = TrackingStationAction::FlyVessel(vessel.id);
                                            }
                                        }
                                        let delete_btn = egui::Button::new(
                                            egui::RichText::new("X").color(egui::Color32::from_rgb(200, 80, 80))
                                        ).small();
                                        if ui.add(delete_btn).on_hover_text("Delete vessel").clicked() {
                                            ts_action = TrackingStationAction::DeleteVessel(vessel.id);
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        });
                    });
            }

            // Body info right panel (shown when a body is tracked)
            if let Some(idx) = self.tracked_body {
                if let Some(info) = body_info.get(idx) {
                    egui::SidePanel::right("body_info_panel")
                        .default_width(220.0)
                        .resizable(false)
                        .show(ctx, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.heading(egui::RichText::new(&info.name).size(18.0).color(egui::Color32::WHITE));
                                if !info.description.is_empty() {
                                    ui.label(egui::RichText::new(&info.description)
                                        .size(12.0)
                                        .italics()
                                        .color(egui::Color32::from_rgb(160, 160, 160)));
                                }
                                ui.separator();

                                // Physical properties
                                ui.label(egui::RichText::new("Physical Properties").size(13.0).color(egui::Color32::from_rgb(200, 200, 200)));
                                ui.add_space(2.0);
                                ui.label(format!("Radius: {}", format_distance(info.radius_m)));
                                ui.label(format!("Surface gravity: {:.2} m/s\u{b2}", info.surface_gravity_ms2));
                                ui.label(format!("Mass: {}", format_mass(info.mass_kg)));

                                // Atmosphere
                                ui.add_space(4.0);
                                if let Some(pressure) = info.atmosphere_pressure_pa {
                                    ui.label(egui::RichText::new("Atmosphere").size(13.0).color(egui::Color32::from_rgb(200, 200, 200)));
                                    ui.add_space(2.0);
                                    ui.label(format!("Pressure: {}", format_pressure(pressure)));
                                    if let Some(height) = info.atmosphere_height_m {
                                        ui.label(format!("Height: {}", format_distance(height)));
                                    }
                                } else {
                                    ui.label(egui::RichText::new("No atmosphere")
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(120, 120, 120)));
                                }

                                // Orbit (skip for root body / Sun)
                                if let Some(sma) = info.orbit_semi_major_axis_m {
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.label(egui::RichText::new("Orbit").size(13.0).color(egui::Color32::from_rgb(200, 200, 200)));
                                    ui.add_space(2.0);
                                    ui.label(format!("Semi-major axis: {}", format_distance(sma)));
                                    if let Some(ecc) = info.orbit_eccentricity {
                                        ui.label(format!("Eccentricity: {:.4}", ecc));
                                    }
                                    if let Some(period) = info.orbit_period_s {
                                        ui.label(format!("Period: {}", format_duration(period)));
                                    }
                                }
                            });
                        });
                }
            }

            // Bottom panel: Tracking Station label
            egui::TopBottomPanel::bottom("tracking_station_label").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(egui::RichText::new("Tracking Station").size(14.0).color(egui::Color32::from_rgb(180, 180, 180)));
                });
            });

            // Hovered body labels (same logic as flight)
            if let Some(idx) = hovered {
                if let Some(body) = bodies_copy.get(idx) {
                    if let Some(name) = body_names.get(idx) {
                        let rel_x = (body.x - camera_pos[0]) as f32;
                        let rel_y = (body.y - camera_pos[1]) as f32;
                        let cos_r = camera_rotation.cos();
                        let sin_r = camera_rotation.sin();
                        let rot_x = rel_x * cos_r - rel_y * sin_r;
                        let rot_y = rel_x * sin_r + rel_y * cos_r;
                        let ndc_x = rot_x * camera_zoom / aspect_ratio;
                        let ndc_y = rot_y * camera_zoom;
                        let screen_x = (ndc_x + 1.0) * 0.5 * size.width as f32 / scale_factor;
                        let screen_y = (1.0 - ndc_y) * 0.5 * size.height as f32 / scale_factor;
                        let label_y = screen_y - 20.0;

                        egui::Area::new(egui::Id::new("body_label"))
                            .fixed_pos(egui::pos2(screen_x, label_y))
                            .pivot(egui::Align2::CENTER_BOTTOM)
                            .show(ctx, |ui| {
                                ui.label(egui::RichText::new(name).color(egui::Color32::WHITE).size(14.0));
                            });
                    }
                }
            }

            // Pause overlay
            if paused {
                egui::Area::new(egui::Id::new("pause_overlay"))
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                            .inner_margin(egui::Margin::same(40.0))
                            .rounding(egui::Rounding::same(8.0))
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.heading(egui::RichText::new("Paused").size(32.0).color(egui::Color32::WHITE));
                                    ui.add_space(20.0);
                                    if ui.button(egui::RichText::new("Main Menu").size(18.0)).clicked() {
                                        pause_action = PauseAction::MainMenu;
                                    }
                                });
                            });
                    });
            }
        });

        self.egui_state.handle_platform_output(&self.window, full_output.platform_output);
        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Tracking Station Render Encoder"),
        });
        self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &tris, &screen_descriptor);

        // Geometry pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tracking Station Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.body_texture_bind_group, &[]);
            render_pass.set_bind_group(2, &self.sprite_atlas.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Egui pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tracking Station Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.egui_renderer.render(&mut render_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok((new_warp_index, pause_action, ts_action))
    }
}
