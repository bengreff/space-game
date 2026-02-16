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
    pub render_scale: f64,
}
