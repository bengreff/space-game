/// Margin from hyperbolic asymptote for trajectory endpoint rendering
pub const HYPERBOLIC_RENDER_MARGIN: f64 = 0.01;

/// Skip margin for points too close to hyperbolic asymptote
pub const HYPERBOLIC_SKIP_MARGIN: f64 = 0.005;

/// Vertex for 2D rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,  // position
        1 => Float32x4,  // color
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Stored body data for hit testing (in world units)
#[derive(Clone)]
pub struct BodyData {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
    pub indicator_radius: f64,
}

/// Orbit data for rendering orbit lines
#[derive(Clone)]
pub struct OrbitRenderData {
    pub parent_x: f64,
    pub parent_y: f64,
    pub semi_major_axis: f64,
    pub eccentricity: f64,
    pub argument_of_periapsis: f64,
    pub color: [f32; 4],
}

/// Ship render data
#[derive(Clone)]
pub struct ShipRenderData {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub size: f64,
    pub color: [f32; 4],
    pub orbit: Option<ShipOrbitData>,
    pub patched_trajectory: Vec<OrbitSegmentData>,
    pub velocity: f64,
    pub altitude: f64,
    pub soi_body_name: String,
    pub throttle: f64,
    pub time_to_intercept: Option<f64>,
    pub acceleration: f64,  // m/s^2 - ship's current thrust / mass
    pub current_true_anomaly: f64, // Ship's current true anomaly in its orbit
}

/// Ship orbit data for rendering
#[derive(Clone)]
pub struct ShipOrbitData {
    pub parent_x: f64,
    pub parent_y: f64,
    pub semi_major_axis: f64,
    pub eccentricity: f64,
    pub argument_of_periapsis: f64,
    pub apoapsis: f64,
    pub periapsis: f64,
    pub orbital_period: f64,
    pub time_to_apoapsis: f64,
    pub time_to_periapsis: f64,
    pub parent_body_radius: f64,
    pub parent_name: String,
    pub retrograde: bool,
}

/// A single segment of a patched conics trajectory for rendering
#[derive(Clone)]
pub struct OrbitSegmentData {
    pub parent_x: f64,
    pub parent_y: f64,
    pub semi_major_axis: f64,
    pub eccentricity: f64,
    pub argument_of_periapsis: f64,
    pub start_true_anomaly: f64,
    pub end_true_anomaly: Option<f64>,
    pub color: [f32; 4],
    pub is_first_segment: bool,
    pub retrograde: bool,
    pub soi_radius: f64,
    pub parent_body_radius: f64,
    pub parent_mass: f64,       // kg - for velocity calculations
    pub parent_idx: usize,      // Index of parent body
    pub render_scale: f64,
}

/// Delta-V components for a maneuver node
#[derive(Clone, Debug, Default)]
pub struct ManeuverDeltaV {
    pub prograde: f64,    // m/s (positive = prograde, negative = retrograde)
    pub radial_out: f64,  // m/s (positive = radial out, negative = radial in)
}

/// A maneuver node - fixed on the orbit it was created on
#[derive(Clone, Debug)]
pub struct ManeuverNode {
    pub id: u64,
    // Orbit parameters (fixed at creation time)
    pub semi_major_axis: f64,      // Scaled for rendering
    pub eccentricity: f64,
    pub argument_of_periapsis: f64,
    pub parent_x: f64,             // Parent position at creation (scaled)
    pub parent_y: f64,
    pub retrograde: bool,
    // Position on the orbit
    pub true_anomaly: f64,         // Position on the stored orbit
    // Parent body info
    pub parent_idx: usize,
    pub parent_mass: f64,
    pub render_scale: f64,
    // Delta-v (original - used for trajectory prediction)
    pub delta_v: ManeuverDeltaV,
    // Remaining delta-v (counts down during burns, displayed in UI)
    pub remaining_delta_v: ManeuverDeltaV,
}

impl ManeuverNode {
    /// Calculate total delta-v magnitude (original, for trajectory)
    pub fn total_delta_v(&self) -> f64 {
        (self.delta_v.prograde.powi(2) + self.delta_v.radial_out.powi(2)).sqrt()
    }

    /// Calculate remaining delta-v magnitude (for display)
    pub fn total_remaining_delta_v(&self) -> f64 {
        (self.remaining_delta_v.prograde.powi(2) + self.remaining_delta_v.radial_out.powi(2)).sqrt()
    }

    /// Calculate world position from stored orbit parameters and current parent position
    pub fn world_pos(&self, current_parent_x: f64, current_parent_y: f64) -> [f64; 2] {
        let e = self.eccentricity;
        let ta = self.true_anomaly;
        let arg_peri = self.argument_of_periapsis;

        let r = if e >= 1.0 {
            // Hyperbolic
            let a_abs = self.semi_major_axis.abs();
            let p = a_abs * (e * e - 1.0);
            let denom = 1.0 + e * ta.cos();
            if denom <= 0.001 { self.semi_major_axis.abs() } else { p / denom }
        } else {
            // Elliptical
            let p = self.semi_major_axis * (1.0 - e * e);
            p / (1.0 + e * ta.cos())
        };

        let angle = ta + arg_peri;
        [
            current_parent_x + r * angle.cos(),
            current_parent_y + r * angle.sin(),
        ]
    }

    /// Calculate velocity at this point on the orbit (unscaled, m/s)
    pub fn velocity(&self) -> [f64; 2] {
        let e = self.eccentricity;
        let ta = self.true_anomaly;
        let arg_peri = self.argument_of_periapsis;
        let a_unscaled = self.semi_major_axis / self.render_scale;

        let r_unscaled = if e >= 1.0 {
            let a_abs = a_unscaled.abs();
            let p = a_abs * (e * e - 1.0);
            let denom = 1.0 + e * ta.cos();
            if denom <= 0.001 { a_abs } else { p / denom }
        } else {
            let p = a_unscaled * (1.0 - e * e);
            p / (1.0 + e * ta.cos())
        };

        let mu = 6.67430e-11 * self.parent_mass;
        let v_squared = mu * (2.0 / r_unscaled - 1.0 / a_unscaled);
        let v_mag = if v_squared > 0.0 { v_squared.sqrt() } else { 0.0 };

        let flight_path_angle = (e * ta.sin()).atan2(1.0 + e * ta.cos());
        let direction_sign = if self.retrograde { -1.0 } else { 1.0 };
        let angle = ta + arg_peri;
        let velocity_angle = angle + direction_sign * std::f64::consts::FRAC_PI_2 - flight_path_angle;

        [v_mag * velocity_angle.cos(), v_mag * velocity_angle.sin()]
    }

    /// Get prograde unit vector at this node
    pub fn prograde_unit(&self) -> [f64; 2] {
        let vel = self.velocity();
        let vel_mag = (vel[0].powi(2) + vel[1].powi(2)).sqrt();
        if vel_mag > 0.0 {
            [vel[0] / vel_mag, vel[1] / vel_mag]
        } else {
            [1.0, 0.0]
        }
    }

    /// Get radial out unit vector at this node
    pub fn radial_unit(&self) -> [f64; 2] {
        let prograde = self.prograde_unit();
        [prograde[1], -prograde[0]]
    }
}
