use super::state::RenderState;

impl RenderState {
    /// Update hover state based on mouse position
    pub fn update_hover(&mut self, screen_x: f32, screen_y: f32) {
        let world_pos = self.camera.screen_to_world(
            screen_x,
            screen_y,
            self.size.width as f32,
            self.size.height as f32,
        );

        self.hovered_body = None;
        let mut closest_dist = f64::MAX;

        for (i, body) in self.bodies.iter().enumerate() {
            // Skip hidden bodies (radius=0, indicator_radius=0)
            if body.radius <= 0.0 && body.indicator_radius <= 0.0 {
                continue;
            }

            let dx = world_pos[0] - body.x;
            let dy = world_pos[1] - body.y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Use indicator radius if present, otherwise body radius
            let hover_radius = if body.indicator_radius > 0.0 {
                body.indicator_radius
            } else {
                body.radius
            };

            if dist <= hover_radius && dist < closest_dist {
                closest_dist = dist;
                self.hovered_body = Some(i);
            }
        }

        // Also check procedural stars (body hover takes priority)
        if self.hovered_body.is_none() {
            self.update_star_hover(screen_x, screen_y);
        } else {
            self.hovered_star = None;
        }
    }

    /// Find body at screen position, returns index of closest body within click range
    pub fn body_at_screen_pos(&self, screen_x: f32, screen_y: f32) -> Option<usize> {
        let world_pos = self.camera.screen_to_world(
            screen_x,
            screen_y,
            self.size.width as f32,
            self.size.height as f32,
        );

        let mut closest: Option<(usize, f64)> = None;

        for (i, body) in self.bodies.iter().enumerate() {
            // Skip hidden bodies (radius=0, indicator_radius=0)
            if body.radius <= 0.0 && body.indicator_radius <= 0.0 {
                continue;
            }

            let dx = world_pos[0] - body.x;
            let dy = world_pos[1] - body.y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Use indicator radius if present, otherwise use body radius with minimum
            let click_radius = if body.indicator_radius > 0.0 {
                body.indicator_radius
            } else {
                body.radius
            };

            if dist <= click_radius {
                // Select closest body center to click point
                match closest {
                    None => closest = Some((i, dist)),
                    Some((_, prev_dist)) if dist < prev_dist => closest = Some((i, dist)),
                    _ => {}
                }
            }
        }

        closest.map(|(i, _)| i)
    }

    /// Find background vessel at screen position, returns vessel ID if within click range (20px)
    pub fn background_vessel_at_screen_pos(&self, screen_x: f32, screen_y: f32) -> Option<u64> {
        let threshold = 20.0f32;
        let mut closest: Option<(u64, f32)> = None;

        for &(id, pos) in &self.background_vessel_screen_positions {
            let dx = screen_x - pos[0];
            let dy = screen_y - pos[1];
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < threshold {
                if closest.is_none() || dist < closest.unwrap().1 {
                    closest = Some((id, dist));
                }
            }
        }

        closest.map(|(id, _)| id)
    }

    /// Update hovered procedural star based on mouse screen position.
    /// Uses screen-space hit testing with a 20px radius.
    pub fn update_star_hover(&mut self, screen_x: f32, screen_y: f32) {
        self.hovered_star = self.star_at_screen_pos(screen_x, screen_y);
    }

    /// Find procedural star at screen position. Returns index into current_procedural_stars.
    pub fn star_at_screen_pos(&self, screen_x: f32, screen_y: f32) -> Option<usize> {
        let threshold = 20.0f32;
        let mut closest: Option<(usize, f32)> = None;

        for &(idx, pos) in &self.procedural_star_screen_positions {
            let dx = screen_x - pos[0];
            let dy = screen_y - pos[1];
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < threshold {
                if closest.is_none() || dist < closest.unwrap().1 {
                    closest = Some((idx, dist));
                }
            }
        }

        closest.map(|(idx, _)| idx)
    }

    /// Focus camera on a body by index and start tracking it
    pub fn focus_on_body(&mut self, index: usize) {
        if let Some(body) = self.bodies.get(index) {
            self.camera.focus_on([body.x, body.y]); // Both are now f64
            self.tracked_body = Some(index);
            self.tracked_vessel = None; // Stop tracking any vessel
            self.focused_star = None;   // Stop tracking any procedural star
            self.focused_star_world_pos = None;
            self.focused_star_id = None;
        }
    }

    /// Update camera to follow tracked body or focused star using current positions
    pub fn update_tracking(&mut self, positions: &[[f64; 2]], scale: f64) {
        if let Some(index) = self.tracked_body {
            if let Some(pos) = positions.get(index) {
                // Set camera position directly to body position (in f64 for precision)
                self.camera.position[0] = pos[0] * scale;
                self.camera.position[1] = pos[1] * scale;
                // When tracking a body (not ship), camera is at body center with no offset
                self.camera.body_center = self.camera.position;
                self.camera.ship_offset = [0.0, 0.0];
            }
        } else if let Some(world_pos) = self.focused_star_world_pos {
            // Track focused procedural star (position in meters, needs scale)
            self.camera.position[0] = world_pos[0] * scale;
            self.camera.position[1] = world_pos[1] * scale;
            self.camera.body_center = self.camera.position;
            self.camera.ship_offset = [0.0, 0.0];
        }
    }
}
