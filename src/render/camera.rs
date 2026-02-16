/// Camera uniform data sent to GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub position: [f32; 2],
    pub zoom: f32,
    pub aspect_ratio: f32,
}

/// Camera state for pan and zoom
pub struct Camera {
    pub position: [f64; 2],
    pub zoom: f32,
    pub aspect_ratio: f32,
    pub is_dragging: bool,
    pub last_mouse_pos: [f32; 2],
}

impl Camera {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: [0.0, 0.0],
            zoom: 1.0,
            aspect_ratio,
            is_dragging: false,
            last_mouse_pos: [0.0, 0.0],
        }
    }

    pub fn to_uniform(&self) -> CameraUniform {
        CameraUniform {
            position: [self.position[0] as f32, self.position[1] as f32],
            zoom: self.zoom,
            aspect_ratio: self.aspect_ratio,
        }
    }

    /// Convert screen coordinates (pixels) to world coordinates
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32, screen_width: f32, screen_height: f32) -> [f64; 2] {
        let ndc_x = (screen_x / screen_width) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y / screen_height) * 2.0;

        let world_x = (ndc_x * self.aspect_ratio / self.zoom) as f64 + self.position[0];
        let world_y = (ndc_y / self.zoom) as f64 + self.position[1];

        [world_x, world_y]
    }

    /// Pan the camera by a screen-space delta
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.position[0] -= (dx / self.zoom) as f64;
        self.position[1] += (dy / self.zoom) as f64;
    }

    /// Zoom the camera by a factor, centered on a world position
    pub fn zoom_at(&mut self, factor: f32, world_pos: [f64; 2]) {
        let old_zoom = self.zoom;
        self.zoom *= factor;
        self.zoom = self.zoom.clamp(0.001, 1e10);

        let scale = (old_zoom / self.zoom) as f64;
        self.position[0] = world_pos[0] - (world_pos[0] - self.position[0]) * scale;
        self.position[1] = world_pos[1] - (world_pos[1] - self.position[1]) * scale;
    }

    /// Focus camera on a world position
    pub fn focus_on(&mut self, world_pos: [f64; 2]) {
        self.position = world_pos;
    }

    /// Simple zoom (centered on camera position)
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom *= factor;
        self.zoom = self.zoom.clamp(0.00001, 1e10);
    }
}
