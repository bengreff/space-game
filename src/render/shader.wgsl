// Camera uniform for view transformation
struct CameraUniform {
    position: vec2<f32>,  // Camera center in world space
    zoom: f32,            // Zoom level
    aspect_ratio: f32,    // Window aspect ratio
    rotation: f32,        // Camera rotation in radians
    _pad0: f32,           // Padding to 32 bytes
    _pad1: f32,
    _pad2: f32,
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
    // 1. Apply rotation
    // 2. Multiply by zoom
    // 3. Correct for aspect ratio
    let cos_r = cos(camera.rotation);
    let sin_r = sin(camera.rotation);
    let rotated = vec2<f32>(
        in.position.x * cos_r - in.position.y * sin_r,
        in.position.x * sin_r + in.position.y * cos_r,
    );
    let view_pos = rotated * camera.zoom;
    let corrected_x = view_pos.x / camera.aspect_ratio;

    out.clip_position = vec4<f32>(corrected_x, view_pos.y, 0.0, 1.0);
    out.color = in.color;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Atmosphere vertices use negative alpha as a flag.
    // Alpha encodes -(1 + t) where t is 0 at surface, 1 at edge.
    // The GPU linearly interpolates alpha from -1 (surface) to -2 (edge).
    if in.color.a < 0.0 {
        let t = -in.color.a - 1.0;
        let brightness = exp(-6.0 * t);
        return vec4<f32>(in.color.r * brightness, in.color.g * brightness, in.color.b * brightness, 1.0);
    }
    return in.color;
}
