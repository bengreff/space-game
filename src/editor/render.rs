use crate::parts::{PartDefinitions, PartShape, PartDefinition, PartCategory};
use crate::render::Vertex;
use super::EditorState;

/// Grid line color
const GRID_COLOR: [f32; 4] = [0.2, 0.2, 0.3, 0.5];
const GRID_MAJOR_COLOR: [f32; 4] = [0.3, 0.3, 0.4, 0.7];

/// Part colors - grey boxes
const PART_COLOR: [f32; 4] = [0.4, 0.4, 0.45, 1.0];
const PART_SELECTED_COLOR: [f32; 4] = [0.5, 0.7, 1.0, 1.0];
const PART_HOVERED_COLOR: [f32; 4] = [0.55, 0.55, 0.6, 1.0];

/// Ghost colors - transparent preview
const GHOST_VALID_COLOR: [f32; 4] = [0.3, 0.9, 0.3, 0.4];
const GHOST_INVALID_COLOR: [f32; 4] = [0.9, 0.3, 0.3, 0.4];

// Engine base colors (darker than regular parts)
// First stage engines are darker, upper stage are slightly lighter
const ENGINE_NOZZLE_DARK: [f32; 4] = [0.08, 0.08, 0.10, 1.0];       // Very dark for first stage
const ENGINE_NOZZLE_LIGHT: [f32; 4] = [0.15, 0.15, 0.17, 1.0];      // Lighter for upper stage
const ENGINE_CHAMBER_COLOR: [f32; 4] = [0.18, 0.18, 0.20, 1.0];     // Combustion chamber
const ENGINE_RING_DARK: [f32; 4] = [0.14, 0.14, 0.16, 1.0];         // Rings on dark nozzles
const ENGINE_RING_LIGHT: [f32; 4] = [0.22, 0.22, 0.24, 1.0];        // Rings on light nozzles
const ENGINE_TURBOPUMP_COLOR: [f32; 4] = [0.20, 0.20, 0.22, 1.0];   // Turbopump housing
const ENGINE_GAS_GEN_COLOR: [f32; 4] = [0.16, 0.16, 0.18, 1.0];     // Gas generator box

// Pod colors
const POD_COLOR: [f32; 4] = [0.15, 0.15, 0.18, 1.0];               // Dark grey pod
const POD_WINDOW_COLOR: [f32; 4] = [0.9, 0.9, 0.95, 1.0];          // White/light grey window

/// Generate vertices for the editor grid (as thin quads for triangle rendering)
/// Note: Vertices are output in CAMERA-RELATIVE coordinates (shader expects this)
pub fn generate_grid_vertices(
    editor: &EditorState,
    screen_width: f32,
    screen_height: f32,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    let zoom = editor.camera_zoom;
    let cam_x = editor.camera_offset[0] as f32;
    let cam_y = editor.camera_offset[1] as f32;

    // Calculate visible area in world coordinates
    let half_width = (screen_width / 2.0) / zoom;
    let half_height = (screen_height / 2.0) / zoom;

    // World coordinate bounds
    let min_x = cam_x - half_width;
    let max_x = cam_x + half_width;
    let min_y = cam_y - half_height;
    let max_y = cam_y + half_height;

    // Grid spacing (in meters)
    let minor_spacing = 0.5;
    let major_spacing = 2.5;

    // Line thickness in world units (thinner at higher zoom)
    let line_thickness = 0.005 / zoom.sqrt();

    // Calculate grid line positions
    let start_x = (min_x / minor_spacing).floor() * minor_spacing;
    let start_y = (min_y / minor_spacing).floor() * minor_spacing;

    // Helper to add a line as a quad (two triangles)
    // Input: world coordinates. Output: camera-relative coordinates.
    let mut add_line = |x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 4]| {
        // Convert to camera-relative coordinates
        let x1 = x1 - cam_x;
        let y1 = y1 - cam_y;
        let x2 = x2 - cam_x;
        let y2 = y2 - cam_y;

        // Calculate perpendicular direction for line thickness
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.0001 {
            return;
        }
        let nx = -dy / len * line_thickness;
        let ny = dx / len * line_thickness;

        // Four corners of the line quad
        let p1 = [x1 - nx, y1 - ny];
        let p2 = [x1 + nx, y1 + ny];
        let p3 = [x2 + nx, y2 + ny];
        let p4 = [x2 - nx, y2 - ny];

        // Triangle 1
        vertices.push(Vertex { position: p1, color });
        vertices.push(Vertex { position: p2, color });
        vertices.push(Vertex { position: p3, color });

        // Triangle 2
        vertices.push(Vertex { position: p1, color });
        vertices.push(Vertex { position: p3, color });
        vertices.push(Vertex { position: p4, color });
    };

    // Vertical grid lines
    let mut x = start_x;
    while x <= max_x {
        let is_major = (x / major_spacing).abs().fract() < 0.01 || x.abs() < 0.01;
        let color = if is_major { GRID_MAJOR_COLOR } else { GRID_COLOR };
        add_line(x, min_y, x, max_y, color);
        x += minor_spacing;
    }

    // Horizontal grid lines
    let mut y = start_y;
    while y <= max_y {
        let is_major = (y / major_spacing).abs().fract() < 0.01 || y.abs() < 0.01;
        let color = if is_major { GRID_MAJOR_COLOR } else { GRID_COLOR };
        add_line(min_x, y, max_x, y, color);
        y += minor_spacing;
    }

    vertices
}

/// Generate vertices for placed parts
/// Note: Vertices are output in CAMERA-RELATIVE coordinates
pub fn generate_part_vertices(
    editor: &EditorState,
    part_defs: &PartDefinitions,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    let cam_x = editor.camera_offset[0] as f32;
    let cam_y = editor.camera_offset[1] as f32;

    for (id, part) in &editor.parts {
        let Some(def) = part_defs.get(&part.definition_id) else {
            continue;
        };

        let is_selected = editor.selected_placed_part == Some(*id);
        let is_hovered = editor.hovered_part == Some(*id);
        let is_dragging = editor.dragging_part == Some(*id);
        let drag_invalid = is_dragging && !editor.drag_valid;

        let color = if is_selected {
            PART_SELECTED_COLOR
        } else if is_hovered {
            PART_HOVERED_COLOR
        } else {
            PART_COLOR
        };

        let half_w = (def.width() / 2.0) as f32;
        let half_h = (def.height() / 2.0) as f32;
        // Convert to camera-relative coordinates
        let x = part.position[0] as f32 - cam_x;
        let y = part.position[1] as f32 - cam_y;

        // For engines, use dedicated engine rendering
        if def.category == PartCategory::Propulsion && def.engine.is_some() {
            generate_engine_details(&mut vertices, def, x, y, 1.0);

            // Draw overlay for selection, hover, or invalid drag
            if is_selected || is_hovered || drag_invalid {
                let highlight_color = if drag_invalid {
                    [0.9, 0.2, 0.2, 0.4]  // Red tint for invalid position
                } else if is_selected {
                    [0.5, 0.7, 1.0, 0.3]
                } else {
                    [0.55, 0.55, 0.6, 0.2]
                };
                let half_top_w = (def.top_width() / 2.0) as f32;
                vertices.push(Vertex { position: [x - half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_top_w, y + half_h], color: highlight_color });
            }
            continue;
        }

        // For pods, use dedicated pod rendering
        if def.category == PartCategory::Pods {
            generate_pod_details(&mut vertices, def, x, y, 1.0);

            // Draw overlay for selection, hover, or invalid drag
            if is_selected || is_hovered || drag_invalid {
                let highlight_color = if drag_invalid {
                    [0.9, 0.2, 0.2, 0.4]
                } else if is_selected {
                    [0.5, 0.7, 1.0, 0.3]
                } else {
                    [0.55, 0.55, 0.6, 0.2]
                };
                let half_top_w = (def.top_width() / 2.0) as f32;
                vertices.push(Vertex { position: [x - half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_top_w, y + half_h], color: highlight_color });
            }
            continue;
        }

        // Draw non-engine parts based on shape
        match def.shape {
            PartShape::Rectangle => {
                // Two triangles for a rectangle
                vertices.push(Vertex { position: [x - half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_w, y + half_h], color });

                vertices.push(Vertex { position: [x - half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_w, y + half_h], color });
                vertices.push(Vertex { position: [x - half_w, y + half_h], color });

                // Invalid drag overlay for rectangles
                if drag_invalid {
                    let overlay = [0.9, 0.2, 0.2, 0.4];
                    vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_w, y + half_h], color: overlay });
                    vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_w, y + half_h], color: overlay });
                    vertices.push(Vertex { position: [x - half_w, y + half_h], color: overlay });
                }
            }
            PartShape::Triangle => {
                // Single triangle with base at bottom, point at top
                vertices.push(Vertex { position: [x - half_w, y - half_h], color }); // bottom left
                vertices.push(Vertex { position: [x + half_w, y - half_h], color }); // bottom right
                vertices.push(Vertex { position: [x, y + half_h], color });          // top center

                // Invalid drag overlay for triangles
                if drag_invalid {
                    let overlay = [0.9, 0.2, 0.2, 0.4];
                    vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x, y + half_h], color: overlay });
                }
            }
            PartShape::Trapezoid => {
                // Trapezoid: wider at bottom, narrower at top
                let half_top_w = (def.top_width() / 2.0) as f32;

                // Two triangles for trapezoid
                // Triangle 1: bottom left, bottom right, top right
                vertices.push(Vertex { position: [x - half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_top_w, y + half_h], color });

                // Triangle 2: bottom left, top right, top left
                vertices.push(Vertex { position: [x - half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_top_w, y + half_h], color });
                vertices.push(Vertex { position: [x - half_top_w, y + half_h], color });

                // Invalid drag overlay for trapezoids
                if drag_invalid {
                    let overlay = [0.9, 0.2, 0.2, 0.4];
                    vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: overlay });
                    vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: overlay });
                    vertices.push(Vertex { position: [x - half_top_w, y + half_h], color: overlay });
                }
            }
        }
    }

    vertices
}

/// Generate vertices for the ghost preview
/// Note: Vertices are output in CAMERA-RELATIVE coordinates
pub fn generate_ghost_vertices(
    editor: &EditorState,
    part_defs: &PartDefinitions,
) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    let Some(ref def_id) = editor.selected_part_def else {
        return vertices;
    };

    let Some(position) = editor.ghost_position else {
        return vertices;
    };

    let Some(def) = part_defs.get(def_id) else {
        return vertices;
    };

    let ghost_alpha = 0.5;

    let half_w = (def.width() / 2.0) as f32;
    let half_h = (def.height() / 2.0) as f32;
    // Convert to camera-relative coordinates
    let cam_x = editor.camera_offset[0] as f32;
    let cam_y = editor.camera_offset[1] as f32;
    let x = position[0] as f32 - cam_x;
    let y = position[1] as f32 - cam_y;

    // For engines, use dedicated engine rendering with ghost overlay
    if def.category == PartCategory::Propulsion && def.engine.is_some() {
        generate_engine_details(&mut vertices, def, x, y, ghost_alpha);

        // Draw validity overlay
        let overlay_color = if editor.ghost_valid {
            [0.3, 0.9, 0.3, 0.25]
        } else {
            [0.9, 0.3, 0.3, 0.25]
        };
        let half_top_w = (def.top_width() / 2.0) as f32;
        vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_top_w, y + half_h], color: overlay_color });

        return vertices;
    }

    // For pods, use dedicated pod rendering with ghost overlay
    if def.category == PartCategory::Pods {
        generate_pod_details(&mut vertices, def, x, y, ghost_alpha);

        // Draw validity overlay
        let overlay_color = if editor.ghost_valid {
            [0.3, 0.9, 0.3, 0.25]
        } else {
            [0.9, 0.3, 0.3, 0.25]
        };
        let half_top_w = (def.top_width() / 2.0) as f32;
        vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_top_w, y + half_h], color: overlay_color });

        return vertices;
    }

    // Draw non-engine ghost based on shape
    let color = if editor.ghost_valid {
        GHOST_VALID_COLOR
    } else {
        GHOST_INVALID_COLOR
    };

    match def.shape {
        PartShape::Rectangle => {
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y + half_h], color });

            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y + half_h], color });
            vertices.push(Vertex { position: [x - half_w, y + half_h], color });
        }
        PartShape::Triangle => {
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x, y + half_h], color });
        }
        PartShape::Trapezoid => {
            let half_top_w = (def.top_width() / 2.0) as f32;

            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_top_w, y + half_h], color });

            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_top_w, y + half_h], color });
            vertices.push(Vertex { position: [x - half_top_w, y + half_h], color });
        }
    }

    vertices
}

/// Generate vertices for engine details (nozzle, pipes, rings, turbopump)
/// This draws the detailed engine visuals - replaces the base part shape entirely
pub fn generate_engine_details(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    alpha: f32,
) {
    let Some(ref engine) = def.engine else { return };

    let half_w = (def.width() / 2.0) as f32;
    let half_h = (def.height() / 2.0) as f32;
    let half_top_w = (def.top_width() / 2.0) as f32;

    // Determine engine characteristics for visual style
    // First stage = good sea level performance (ASL thrust > 70% of vacuum)
    let is_first_stage = engine.thrust_asl > engine.thrust_vac * 0.7;
    let has_gimbal = engine.gimbal_range > 2.0;

    // Apply alpha to colors
    let apply_alpha = |c: [f32; 4]| -> [f32; 4] {
        [c[0], c[1], c[2], c[3] * alpha]
    };

    // Choose colors based on engine type
    let nozzle_color = if is_first_stage {
        apply_alpha(ENGINE_NOZZLE_DARK)
    } else {
        apply_alpha(ENGINE_NOZZLE_LIGHT)
    };
    let ring_color = if is_first_stage {
        apply_alpha(ENGINE_RING_DARK)
    } else {
        apply_alpha(ENGINE_RING_LIGHT)
    };
    let chamber_color = apply_alpha(ENGINE_CHAMBER_COLOR);
    let turbopump_color = apply_alpha(ENGINE_TURBOPUMP_COLOR);
    let gas_gen_color = apply_alpha(ENGINE_GAS_GEN_COLOR);

    // Draw main nozzle bell (trapezoid)
    vertices.push(Vertex { position: [x - half_w, y - half_h], color: nozzle_color });
    vertices.push(Vertex { position: [x + half_w, y - half_h], color: nozzle_color });
    vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: nozzle_color });
    vertices.push(Vertex { position: [x - half_w, y - half_h], color: nozzle_color });
    vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: nozzle_color });
    vertices.push(Vertex { position: [x - half_top_w, y + half_h], color: nozzle_color });

    // Engine-specific details based on ID
    generate_engine_specific_details(
        vertices,
        def,
        x, y,
        half_w, half_h, half_top_w,
        is_first_stage, has_gimbal,
        nozzle_color, chamber_color, ring_color, turbopump_color, gas_gen_color, alpha,
    );
}

/// Draw engine-specific details based on the engine ID
fn generate_engine_specific_details(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32, y: f32,
    half_w: f32, half_h: f32, half_top_w: f32,
    is_first_stage: bool, has_gimbal: bool,
    _nozzle_color: [f32; 4],
    chamber_color: [f32; 4],
    ring_color: [f32; 4],
    turbopump_color: [f32; 4],
    gas_gen_color: [f32; 4],
    _alpha: f32,
) {
    match def.id.as_str() {
        // TINY ENGINES
        "engine_hummingbird" => {
            // Small methalox probe engine - compact, few rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.7, half_top_w * 0.7, half_h * 0.3, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 4, ring_color);
        }
        "engine_gecko" => {
            // Kerolox cluster engine - compact with rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.65, half_top_w * 0.8, half_h * 0.35, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 5, ring_color);
        }
        "engine_firefly" => {
            // Vacuum hydrolox - smooth bell, minimal rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.75, half_top_w * 0.6, half_h * 0.25, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 2, ring_color);
        }
        "engine_sparrow" => {
            // High gimbal landing engine
            draw_combustion_chamber(vertices, x, y + half_h * 0.6, half_top_w * 0.7, half_h * 0.4, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 4, ring_color);
            draw_gimbal_actuators(vertices, x, y + half_h * 0.3, half_top_w * 0.9, half_h * 0.15, ring_color);
        }

        // SMALL ENGINES
        "engine_wolf" => {
            // Merlin-style reusable kerolox - many cooling rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.7, half_top_w * 0.9, half_h * 0.3, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 8, ring_color);
            draw_turbopump_box(vertices, x, y + half_h * 0.95, half_top_w * 0.6, half_h * 0.08, turbopump_color);
        }
        "engine_falcon" => {
            // Raptor-style methalox - compact, ringed
            draw_combustion_chamber(vertices, x, y + half_h * 0.65, half_top_w * 0.85, half_h * 0.35, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 6, ring_color);
            draw_gimbal_actuators(vertices, x, y + half_h * 0.2, half_top_w * 0.7, half_h * 0.1, ring_color);
        }
        "engine_owl" => {
            // Vacuum hydrolox - large bell, few rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.8, half_top_w * 0.7, half_h * 0.2, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 3, ring_color);
        }
        "engine_viper" => {
            // High thrust fixed nozzle - dense rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.7, half_top_w * 0.95, half_h * 0.3, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 10, ring_color);
            draw_turbopump_box(vertices, x, y + half_h * 0.95, half_top_w * 0.7, half_h * 0.08, turbopump_color);
        }

        // MEDIUM ENGINES
        "engine_bear" => {
            // Methalox workhorse - balanced design
            draw_combustion_chamber(vertices, x, y + half_h * 0.75, half_top_w * 0.85, half_h * 0.25, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 7, ring_color);
            draw_gimbal_actuators(vertices, x, y + half_h * 0.25, half_top_w * 0.6, half_h * 0.08, ring_color);
            draw_turbopump_box(vertices, x, y + half_h * 0.95, half_top_w * 0.5, half_h * 0.06, turbopump_color);
        }
        "engine_eagle" => {
            // RS-25 heritage - many rings, complex
            draw_combustion_chamber(vertices, x, y + half_h * 0.8, half_top_w * 0.65, half_h * 0.2, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 12, ring_color);
            draw_gimbal_actuators(vertices, x, y + half_h * 0.3, half_top_w * 0.5, half_h * 0.1, ring_color);
            draw_turbopump_box(vertices, x, y + half_h * 0.95, half_top_w * 0.4, half_h * 0.06, turbopump_color);
        }
        "engine_panther" => {
            // RD-180 twin chamber style - gas generator on side
            draw_combustion_chamber(vertices, x, y + half_h * 0.7, half_top_w * 0.9, half_h * 0.3, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 6, ring_color);
            draw_gas_generator(vertices, x + half_top_w * 0.9, y + half_h * 0.5, half_top_w * 0.25, half_h * 0.3, gas_gen_color);
        }
        "engine_crane" => {
            // Landing engine - big gimbal
            draw_combustion_chamber(vertices, x, y + half_h * 0.65, half_top_w * 0.8, half_h * 0.35, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 5, ring_color);
            draw_gimbal_actuators(vertices, x, y + half_h * 0.2, half_top_w * 0.95, half_h * 0.12, ring_color);
        }

        // LARGE ENGINES
        "engine_mammoth" => {
            // F-1 heritage - massive, many rings, gas generator
            draw_combustion_chamber(vertices, x, y + half_h * 0.8, half_top_w * 0.85, half_h * 0.2, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 14, ring_color);
            draw_turbopump_box(vertices, x, y + half_h * 0.95, half_top_w * 0.6, half_h * 0.06, turbopump_color);
            draw_gas_generator(vertices, x + half_top_w * 0.85, y + half_h * 0.6, half_top_w * 0.3, half_h * 0.25, gas_gen_color);
        }
        "engine_whale" => {
            // RS-68 class hydrolox - large bell, moderate rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.8, half_top_w * 0.7, half_h * 0.2, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 8, ring_color);
            draw_turbopump_box(vertices, x, y + half_h * 0.95, half_top_w * 0.5, half_h * 0.06, turbopump_color);
        }
        "engine_bison" => {
            // Raptor vacuum style - ringed, gimbaled
            draw_combustion_chamber(vertices, x, y + half_h * 0.75, half_top_w * 0.9, half_h * 0.25, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 8, ring_color);
            draw_gimbal_actuators(vertices, x, y + half_h * 0.3, half_top_w * 0.85, half_h * 0.1, ring_color);
            draw_turbopump_box(vertices, x, y + half_h * 0.95, half_top_w * 0.65, half_h * 0.06, turbopump_color);
        }
        "engine_titan" => {
            // Large vacuum kerolox - huge bell, many rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.85, half_top_w * 0.6, half_h * 0.15, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 10, ring_color);
            draw_turbopump_box(vertices, x, y + half_h * 0.95, half_top_w * 0.4, half_h * 0.05, turbopump_color);
        }

        // Fallback for unknown engines
        _ => {
            draw_combustion_chamber(vertices, x, y + half_h * 0.75, half_top_w * 0.8, half_h * 0.25, chamber_color);
            let ring_count = if is_first_stage { 8 } else { 4 };
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, ring_count, ring_color);
            if has_gimbal {
                draw_gimbal_actuators(vertices, x, y + half_h * 0.3, half_top_w * 0.6, half_h * 0.1, ring_color);
            }
        }
    }
}

// Helper functions for drawing engine details

/// Draw a rectangular turbopump housing at the top of the engine
fn draw_turbopump_box(
    vertices: &mut Vec<Vertex>,
    x: f32, y: f32,
    half_w: f32, half_h: f32,
    color: [f32; 4],
) {
    // Rectangle centered at (x, y)
    vertices.push(Vertex { position: [x - half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y + half_h], color });
    vertices.push(Vertex { position: [x - half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y + half_h], color });
    vertices.push(Vertex { position: [x - half_w, y + half_h], color });
}

/// Draw cooling rings/tubes wrapping all around the nozzle (horizontal bands)
fn draw_nozzle_rings(
    vertices: &mut Vec<Vertex>,
    x: f32, y: f32,
    half_w: f32, half_top_w: f32, half_h: f32,
    count: u32,
    color: [f32; 4],
) {
    let ring_thickness = half_h * 0.04;

    for i in 0..count {
        // Position along nozzle (from bottom toward top, but not all the way)
        let t = (i as f32 + 1.0) / (count as f32 + 1.5) * 0.7;
        let y_pos = y - half_h + half_h * 2.0 * t;

        // Interpolate width at this height (nozzle tapers from bottom to top)
        let width_at_y = half_w + (half_top_w - half_w) * t;

        // Draw ring as horizontal band across the entire nozzle width
        vertices.push(Vertex { position: [x - width_at_y, y_pos - ring_thickness], color });
        vertices.push(Vertex { position: [x + width_at_y, y_pos - ring_thickness], color });
        vertices.push(Vertex { position: [x + width_at_y, y_pos + ring_thickness], color });
        vertices.push(Vertex { position: [x - width_at_y, y_pos - ring_thickness], color });
        vertices.push(Vertex { position: [x + width_at_y, y_pos + ring_thickness], color });
        vertices.push(Vertex { position: [x - width_at_y, y_pos + ring_thickness], color });
    }
}

/// Draw gimbal actuator brackets
fn draw_gimbal_actuators(
    vertices: &mut Vec<Vertex>,
    x: f32, y: f32,
    half_w: f32, half_h: f32,
    color: [f32; 4],
) {
    // Left actuator (angled bracket)
    let left_x = x - half_w;
    vertices.push(Vertex { position: [left_x, y - half_h], color });
    vertices.push(Vertex { position: [left_x - half_w * 0.3, y], color });
    vertices.push(Vertex { position: [left_x, y + half_h], color });

    // Right actuator
    let right_x = x + half_w;
    vertices.push(Vertex { position: [right_x, y - half_h], color });
    vertices.push(Vertex { position: [right_x + half_w * 0.3, y], color });
    vertices.push(Vertex { position: [right_x, y + half_h], color });
}

/// Draw the combustion chamber at the top of the engine (rectangular box)
fn draw_combustion_chamber(
    vertices: &mut Vec<Vertex>,
    x: f32, y: f32,
    half_w: f32, half_h: f32,
    color: [f32; 4],
) {
    // Rectangle centered at (x, y)
    vertices.push(Vertex { position: [x - half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y + half_h], color });
    vertices.push(Vertex { position: [x - half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y + half_h], color });
    vertices.push(Vertex { position: [x - half_w, y + half_h], color });
}

/// Draw a gas generator box on the side of the engine (like F-1 or RD-180)
fn draw_gas_generator(
    vertices: &mut Vec<Vertex>,
    x: f32, y: f32,
    half_w: f32, half_h: f32,
    color: [f32; 4],
) {
    // Rectangle centered at (x, y)
    vertices.push(Vertex { position: [x - half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y + half_h], color });
    vertices.push(Vertex { position: [x - half_w, y - half_h], color });
    vertices.push(Vertex { position: [x + half_w, y + half_h], color });
    vertices.push(Vertex { position: [x - half_w, y + half_h], color });
}

/// Generate pod details (dark grey trapezoid with white circle window)
pub fn generate_pod_details(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    alpha: f32,
) {
    let half_w = (def.width() / 2.0) as f32;
    let half_h = (def.height() / 2.0) as f32;
    let half_top_w = (def.top_width() / 2.0) as f32;

    // Apply alpha to colors
    let pod_color = [POD_COLOR[0], POD_COLOR[1], POD_COLOR[2], POD_COLOR[3] * alpha];
    let window_color = [POD_WINDOW_COLOR[0], POD_WINDOW_COLOR[1], POD_WINDOW_COLOR[2], POD_WINDOW_COLOR[3] * alpha];

    // Draw main pod body (trapezoid)
    vertices.push(Vertex { position: [x - half_w, y - half_h], color: pod_color });
    vertices.push(Vertex { position: [x + half_w, y - half_h], color: pod_color });
    vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: pod_color });
    vertices.push(Vertex { position: [x - half_w, y - half_h], color: pod_color });
    vertices.push(Vertex { position: [x + half_top_w, y + half_h], color: pod_color });
    vertices.push(Vertex { position: [x - half_top_w, y + half_h], color: pod_color });

    // Draw white circular window in the center
    // Window size proportional to pod size
    let window_radius = half_w.min(half_h) * 0.25;
    let window_y = y + half_h * 0.2;  // Slightly above center
    draw_circle(vertices, x, window_y, window_radius, window_color);
}

/// Draw a filled circle
pub fn draw_circle(
    vertices: &mut Vec<Vertex>,
    x: f32, y: f32,
    radius: f32,
    color: [f32; 4],
) {
    let segments = 12;
    for i in 0..segments {
        let a1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        vertices.push(Vertex { position: [x, y], color });
        vertices.push(Vertex { position: [x + a1.cos() * radius, y + a1.sin() * radius], color });
        vertices.push(Vertex { position: [x + a2.cos() * radius, y + a2.sin() * radius], color });
    }
}

/// Generate exhaust plume vertices for a firing engine.
/// Draws a red outer triangle and yellow inner triangle extending from the nozzle exit.
/// Plume width = nozzle diameter, plume length = 2 × nozzle diameter.
pub fn generate_engine_plume_vertices(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    throttle: f32,
) {
    if throttle <= 0.0 {
        return;
    }

    let half_h = (def.height() / 2.0) as f32;
    let nozzle_width = def.width() as f32;
    let half_nozzle = nozzle_width / 2.0;
    let plume_length = nozzle_width * 2.0 * throttle;

    // Nozzle exit is at the bottom of the engine (y - half_h)
    let nozzle_y = y - half_h;

    // Red outer plume triangle
    let red = [1.0, 0.2, 0.0, 0.9];
    vertices.push(Vertex { position: [x - half_nozzle, nozzle_y], color: red });
    vertices.push(Vertex { position: [x + half_nozzle, nozzle_y], color: red });
    vertices.push(Vertex { position: [x, nozzle_y - plume_length], color: red });

    // Yellow inner plume triangle (60% width, 40% length)
    let yellow = [1.0, 0.9, 0.1, 1.0];
    let inner_half_w = half_nozzle * 0.6;
    let inner_length = plume_length * 0.4;
    vertices.push(Vertex { position: [x - inner_half_w, nozzle_y], color: yellow });
    vertices.push(Vertex { position: [x + inner_half_w, nozzle_y], color: yellow });
    vertices.push(Vertex { position: [x, nozzle_y - inner_length], color: yellow });
}

/// Generate vertices for a single part at the given (x, y) center position.
/// Dispatches to the correct shape/category renderer.
/// Used by both the editor and flight rendering.
pub fn generate_part_shape_vertices(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    alpha: f32,
) {
    // For engines, use dedicated engine rendering
    if def.category == PartCategory::Propulsion && def.engine.is_some() {
        generate_engine_details(vertices, def, x, y, alpha);
        return;
    }

    // For pods, use dedicated pod rendering
    if def.category == PartCategory::Pods {
        generate_pod_details(vertices, def, x, y, alpha);
        return;
    }

    let half_w = (def.width() / 2.0) as f32;
    let half_h = (def.height() / 2.0) as f32;
    let color = [0.4, 0.4, 0.45, alpha];

    match def.shape {
        PartShape::Rectangle => {
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y + half_h], color });
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y + half_h], color });
            vertices.push(Vertex { position: [x - half_w, y + half_h], color });
        }
        PartShape::Triangle => {
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x, y + half_h], color });
        }
        PartShape::Trapezoid => {
            let half_top_w = (def.top_width() / 2.0) as f32;
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_top_w, y + half_h], color });
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_top_w, y + half_h], color });
            vertices.push(Vertex { position: [x - half_top_w, y + half_h], color });
        }
    }
}

/// Convert screen coordinates to world coordinates in the editor
pub fn screen_to_world(
    screen_x: f32,
    screen_y: f32,
    screen_width: f32,
    screen_height: f32,
    editor: &EditorState,
) -> [f64; 2] {
    let zoom = editor.camera_zoom;
    let offset = editor.camera_offset;
    let aspect_ratio = screen_width / screen_height;

    // Convert from screen space (0,0 at top-left) to NDC (-1 to 1)
    let ndc_x = (screen_x / screen_width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_y / screen_height) * 2.0; // Flip Y

    // Invert the shader transform: shader does (world * zoom) / aspect for x
    // So world_x = ndc_x * aspect / zoom
    let world_x = (ndc_x * aspect_ratio / zoom) as f64 + offset[0];
    let world_y = (ndc_y / zoom) as f64 + offset[1];

    [world_x, world_y]
}

/// Convert world coordinates to screen coordinates in the editor
pub fn world_to_screen(
    world_x: f64,
    world_y: f64,
    screen_width: f32,
    screen_height: f32,
    editor: &EditorState,
) -> [f32; 2] {
    let zoom = editor.camera_zoom;
    let offset = editor.camera_offset;

    let rel_x = world_x - offset[0];
    let rel_y = world_y - offset[1];

    let screen_x = (rel_x * zoom as f64) as f32 + screen_width / 2.0;
    let screen_y = screen_height / 2.0 - (rel_y * zoom as f64) as f32; // Flip Y

    [screen_x, screen_y]
}

/// Find which part (if any) is at the given screen position
/// Uses hitbox dimensions for click detection
pub fn part_at_screen_pos(
    screen_x: f32,
    screen_y: f32,
    screen_width: f32,
    screen_height: f32,
    editor: &EditorState,
    part_defs: &PartDefinitions,
) -> Option<crate::parts::PlacedPartId> {
    let [world_x, world_y] = screen_to_world(screen_x, screen_y, screen_width, screen_height, editor);

    for (id, part) in &editor.parts {
        let Some(def) = part_defs.get(&part.definition_id) else {
            continue;
        };

        // Use hitbox dimensions for click detection
        let half_w = def.hitbox_width() / 2.0;
        let half_h = def.hitbox_height() / 2.0;

        if world_x >= part.position[0] - half_w
            && world_x <= part.position[0] + half_w
            && world_y >= part.position[1] - half_h
            && world_y <= part.position[1] + half_h
        {
            return Some(*id);
        }
    }

    None
}
