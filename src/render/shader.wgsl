// Camera uniform for view transformation
struct CameraUniform {
    position: vec2<f32>,  // Camera center in world space
    zoom: f32,            // Zoom level
    aspect_ratio: f32,    // Window aspect ratio
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// Vertex shader

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Transform camera-relative position to clip space:
    // Positions are already relative to camera (computed in CPU for precision)
    // 1. Multiply by zoom
    // 2. Correct for aspect ratio
    let view_pos = in.position * camera.zoom;
    let corrected_x = view_pos.x / camera.aspect_ratio;

    out.clip_position = vec4<f32>(corrected_x, view_pos.y, 0.0, 1.0);
    out.color = in.color;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
