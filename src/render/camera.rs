/// Camera uniform data sent to GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub position: [f32; 2],
    pub zoom: f32,
    pub aspect_ratio: f32,
    pub rotation: f32,
    pub _padding: [f32; 3],
}

/// Camera state for pan and zoom
pub struct Camera {
    pub position: [f64; 2],
    pub zoom: f32,
    pub aspect_ratio: f32,
    pub rotation: f32,
    pub is_dragging: bool,
    pub last_mouse_pos: [f32; 2],
}

impl Camera {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            position: [0.0, 0.0],
            zoom: 1.0,
            aspect_ratio,
            rotation: 0.0,
            is_dragging: false,
            last_mouse_pos: [0.0, 0.0],
        }
    }

    pub fn to_uniform(&self) -> CameraUniform {
        CameraUniform {
            position: [self.position[0] as f32, self.position[1] as f32],
            zoom: self.zoom,
            aspect_ratio: self.aspect_ratio,
            rotation: self.rotation,
            _padding: [0.0; 3],
        }
    }

    /// Convert screen coordinates (pixels) to world coordinates
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32, screen_width: f32, screen_height: f32) -> [f64; 2] {
        let ndc_x = (screen_x / screen_width) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y / screen_height) * 2.0;

        // Undo aspect ratio correction and zoom
        let view_x = ndc_x * self.aspect_ratio / self.zoom;
        let view_y = ndc_y / self.zoom;

        // Undo rotation (apply inverse rotation)
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let unrotated_x = view_x * cos_r + view_y * sin_r;
        let unrotated_y = -view_x * sin_r + view_y * cos_r;

        let world_x = unrotated_x as f64 + self.position[0];
        let world_y = unrotated_y as f64 + self.position[1];

        [world_x, world_y]
    }

    /// Pan the camera by a screen-space delta (rotation-aware)
    pub fn pan(&mut self, dx: f32, dy: f32) {
        // Rotate the drag delta by negative camera rotation so panning feels natural
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let rotated_dx = dx * cos_r + dy * sin_r;
        let rotated_dy = -dx * sin_r + dy * cos_r;
        self.position[0] -= (rotated_dx / self.zoom) as f64;
        self.position[1] += (rotated_dy / self.zoom) as f64;
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
