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

// Decoupler colors
const DECOUPLER_RING_COLOR: [f32; 4] = [0.25, 0.25, 0.28, 1.0];    // Dark metallic grey ring

// Heat shield colors
const HEAT_SHIELD_FACE_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];  // Near-black ablative face
const HEAT_SHIELD_BACK_COLOR: [f32; 4] = [0.12, 0.12, 0.12, 1.0];  // Dark backing structure

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

        let is_selected = editor.selected_placed_part == Some(*id)
            || editor.selected_placed_part.and_then(|sel| editor.parts.get(&sel)?.mirror_partner) == Some(*id);
        let is_hovered = editor.hovered_part == Some(*id)
            || editor.hovered_part.and_then(|hov| editor.parts.get(&hov)?.mirror_partner) == Some(*id);
        let is_dragging = editor.dragging_part == Some(*id)
            || editor.dragging_part.and_then(|d| editor.parts.get(&d)?.mirror_partner) == Some(*id);
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

        // For heat shields, use dedicated heat shield rendering
        if def.is_heat_shield {
            generate_heat_shield_details(&mut vertices, def, x, y, 1.0);

            if is_selected || is_hovered || drag_invalid {
                let highlight_color = if drag_invalid {
                    [0.9, 0.2, 0.2, 0.4]
                } else if is_selected {
                    [0.5, 0.7, 1.0, 0.3]
                } else {
                    [0.55, 0.55, 0.6, 0.2]
                };
                let hitbox_half_h = (def.hitbox_height() / 2.0) as f32;
                vertices.push(Vertex { position: [x - half_w, y - hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y - hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y + hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_w, y - hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y + hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_w, y + hitbox_half_h], color: highlight_color });
            }
            continue;
        }

        // For decouplers, use dedicated decoupler rendering
        if def.decoupler.is_some() {
            generate_decoupler_details(&mut vertices, def, x, y, 1.0);

            // Draw overlay for selection, hover, or invalid drag
            if is_selected || is_hovered || drag_invalid {
                let highlight_color = if drag_invalid {
                    [0.9, 0.2, 0.2, 0.4]
                } else if is_selected {
                    [0.5, 0.7, 1.0, 0.3]
                } else {
                    [0.55, 0.55, 0.6, 0.2]
                };
                let hitbox_half_h = (def.hitbox_height() / 2.0) as f32;
                vertices.push(Vertex { position: [x - half_w, y - hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y - hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y + hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_w, y - hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y + hitbox_half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_w, y + hitbox_half_h], color: highlight_color });
            }
            continue;
        }

        // For RCS thrusters, use dedicated RCS rendering
        if def.rcs.is_some() {
            generate_rcs_details(&mut vertices, def, x, y, 1.0);

            if is_selected || is_hovered || drag_invalid {
                let highlight_color = if drag_invalid {
                    [0.9, 0.2, 0.2, 0.4]
                } else if is_selected {
                    [0.5, 0.7, 1.0, 0.3]
                } else {
                    [0.55, 0.55, 0.6, 0.2]
                };
                vertices.push(Vertex { position: [x - half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y + half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_w, y - half_h], color: highlight_color });
                vertices.push(Vertex { position: [x + half_w, y + half_h], color: highlight_color });
                vertices.push(Vertex { position: [x - half_w, y + half_h], color: highlight_color });
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
            PartShape::TriangleRight => {
                // Right triangle: vertical edge on right, hypotenuse on left
                vertices.push(Vertex { position: [x - half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_w, y + half_h], color });

                if drag_invalid {
                    let overlay = [0.9, 0.2, 0.2, 0.4];
                    vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_w, y + half_h], color: overlay });
                }
            }
            PartShape::TriangleLeft => {
                // Right triangle: vertical edge on left, hypotenuse on right
                vertices.push(Vertex { position: [x - half_w, y - half_h], color });
                vertices.push(Vertex { position: [x + half_w, y - half_h], color });
                vertices.push(Vertex { position: [x - half_w, y + half_h], color });

                if drag_invalid {
                    let overlay = [0.9, 0.2, 0.2, 0.4];
                    vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x + half_w, y - half_h], color: overlay });
                    vertices.push(Vertex { position: [x - half_w, y + half_h], color: overlay });
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

    // Second pass: draw adapter trapezoids for decouplers
    for (_, part) in &editor.parts {
        let Some(def) = part_defs.get(&part.definition_id) else {
            continue;
        };
        if def.decoupler.is_none() {
            continue;
        }
        let draw_x = part.position[0] as f32 - cam_x;
        let draw_y = part.position[1] as f32 - cam_y;
        let world_x = part.position[0] as f32;
        let world_y = part.position[1] as f32;
        generate_decoupler_adapter(&mut vertices, def, draw_x, draw_y, world_x, world_y, &editor.parts, part_defs, 1.0);
    }

    vertices
}

/// Generate ghost vertices for a single ghost at the given world position
fn generate_single_ghost_vertices(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    position: [f64; 2],
    ghost_valid: bool,
    editor: &EditorState,
    part_defs: &PartDefinitions,
) {
    let ghost_alpha = 0.5;

    let half_w = (def.width() / 2.0) as f32;
    let half_h = (def.height() / 2.0) as f32;
    let cam_x = editor.camera_offset[0] as f32;
    let cam_y = editor.camera_offset[1] as f32;
    let x = position[0] as f32 - cam_x;
    let y = position[1] as f32 - cam_y;

    if def.category == PartCategory::Propulsion && def.engine.is_some() {
        generate_engine_details(vertices, def, x, y, ghost_alpha);

        let overlay_color = if ghost_valid {
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
        return;
    }

    if def.category == PartCategory::Pods {
        generate_pod_details(vertices, def, x, y, ghost_alpha);

        let overlay_color = if ghost_valid {
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
        return;
    }

    if def.is_heat_shield {
        generate_heat_shield_details(vertices, def, x, y, ghost_alpha);

        let overlay_color = if ghost_valid {
            [0.3, 0.9, 0.3, 0.25]
        } else {
            [0.9, 0.3, 0.3, 0.25]
        };
        let hitbox_half_h = (def.hitbox_height() / 2.0) as f32;
        vertices.push(Vertex { position: [x - half_w, y - hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y - hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y + hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_w, y - hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y + hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_w, y + hitbox_half_h], color: overlay_color });
        return;
    }

    if def.decoupler.is_some() {
        generate_decoupler_details(vertices, def, x, y, ghost_alpha);

        let ghost_world_x = position[0] as f32;
        let ghost_world_y = position[1] as f32;
        generate_decoupler_adapter(vertices, def, x, y, ghost_world_x, ghost_world_y, &editor.parts, part_defs, ghost_alpha);

        let overlay_color = if ghost_valid {
            [0.3, 0.9, 0.3, 0.25]
        } else {
            [0.9, 0.3, 0.3, 0.25]
        };
        let hitbox_half_h = (def.hitbox_height() / 2.0) as f32;
        vertices.push(Vertex { position: [x - half_w, y - hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y - hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y + hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_w, y - hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y + hitbox_half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_w, y + hitbox_half_h], color: overlay_color });
        return;
    }

    if def.rcs.is_some() {
        generate_rcs_details(vertices, def, x, y, ghost_alpha);

        let overlay_color = if ghost_valid {
            [0.3, 0.9, 0.3, 0.25]
        } else {
            [0.9, 0.3, 0.3, 0.25]
        };
        vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y + half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_w, y - half_h], color: overlay_color });
        vertices.push(Vertex { position: [x + half_w, y + half_h], color: overlay_color });
        vertices.push(Vertex { position: [x - half_w, y + half_h], color: overlay_color });
        return;
    }

    let color = if ghost_valid {
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
        PartShape::TriangleRight => {
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y + half_h], color });
        }
        PartShape::TriangleLeft => {
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x - half_w, y + half_h], color });
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

/// Generate vertices for the ghost preview (primary + mirror if applicable)
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

    // Render primary ghost
    generate_single_ghost_vertices(&mut vertices, def, position, editor.ghost_valid, editor, part_defs);

    // Render mirror ghost if applicable, using mirror def if available
    if let Some(mirror_pos) = editor.mirror_ghost_position {
        let mirror_def = editor.mirror_ghost_def_id.as_ref()
            .and_then(|mid| part_defs.get(mid))
            .unwrap_or(def);
        generate_single_ghost_vertices(&mut vertices, mirror_def, mirror_pos, editor.ghost_valid, editor, part_defs);
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
        "engine_wren" => {
            // Compact hydrolox upper stage - small chamber, 2 rings
            draw_combustion_chamber(vertices, x, y + half_h * 0.8, half_top_w * 0.6, half_h * 0.2, chamber_color);
            draw_nozzle_rings(vertices, x, y, half_w, half_top_w, half_h, 2, ring_color);
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

    // Draw small triangular RCS nozzles if pod has built-in RCS
    if def.rcs.is_some() {
        let nozzle_color = [RCS_NOZZLE_COLOR[0], RCS_NOZZLE_COLOR[1], RCS_NOZZLE_COLOR[2], RCS_NOZZLE_COLOR[3] * alpha];
        // Nozzle position: ~80% up the pod height, at left and right edges
        let nozzle_y = y - half_h + half_h * 2.0 * 0.8;
        // Interpolate pod width at nozzle height (trapezoid shape)
        let t = 0.8; // fraction from bottom to top
        let edge_x = half_w + t * (half_top_w - half_w); // half-width at nozzle height
        let nozzle_hw: f32 = 0.04; // half-width of nozzle base
        let nozzle_len: f32 = 0.08; // length of nozzle tip from base

        // Right nozzle: triangle pointing right, base on pod edge
        let r_base_x = x + edge_x;
        vertices.push(Vertex { position: [r_base_x, nozzle_y - nozzle_hw], color: nozzle_color });
        vertices.push(Vertex { position: [r_base_x, nozzle_y + nozzle_hw], color: nozzle_color });
        vertices.push(Vertex { position: [r_base_x + nozzle_len, nozzle_y], color: nozzle_color });

        // Left nozzle: triangle pointing left, base on pod edge
        let l_base_x = x - edge_x;
        vertices.push(Vertex { position: [l_base_x, nozzle_y - nozzle_hw], color: nozzle_color });
        vertices.push(Vertex { position: [l_base_x, nozzle_y + nozzle_hw], color: nozzle_color });
        vertices.push(Vertex { position: [l_base_x - nozzle_len, nozzle_y], color: nozzle_color });
    }
}

/// Generate decoupler ring details (dark horizontal band on bottom half of hitbox)
pub fn generate_decoupler_details(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    alpha: f32,
) {
    let half_w = (def.width() / 2.0) as f32;
    let hitbox_half_h = (def.hitbox_height() / 2.0) as f32;
    let visual_h = (def.height()) as f32;

    let ring_color = [DECOUPLER_RING_COLOR[0], DECOUPLER_RING_COLOR[1], DECOUPLER_RING_COLOR[2], DECOUPLER_RING_COLOR[3] * alpha];

    // Ring is drawn on the bottom half of the hitbox
    let ring_bottom = y - hitbox_half_h;
    let ring_top = ring_bottom + visual_h;

    // Two triangles for the ring rectangle
    vertices.push(Vertex { position: [x - half_w, ring_bottom], color: ring_color });
    vertices.push(Vertex { position: [x + half_w, ring_bottom], color: ring_color });
    vertices.push(Vertex { position: [x + half_w, ring_top], color: ring_color });

    vertices.push(Vertex { position: [x - half_w, ring_bottom], color: ring_color });
    vertices.push(Vertex { position: [x + half_w, ring_top], color: ring_color });
    vertices.push(Vertex { position: [x - half_w, ring_top], color: ring_color });
}

/// Generate heat shield details (black ablative face with convex dome, dark backing band)
/// Drawn on the upper half of the hitbox
pub fn generate_heat_shield_details(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    alpha: f32,
) {
    let half_w = (def.width() / 2.0) as f32;
    let hitbox_half_h = (def.hitbox_height() / 2.0) as f32;
    let visual_h = def.height() as f32;

    let face_color = [HEAT_SHIELD_FACE_COLOR[0], HEAT_SHIELD_FACE_COLOR[1], HEAT_SHIELD_FACE_COLOR[2], HEAT_SHIELD_FACE_COLOR[3] * alpha];
    let back_color = [HEAT_SHIELD_BACK_COLOR[0], HEAT_SHIELD_BACK_COLOR[1], HEAT_SHIELD_BACK_COLOR[2], HEAT_SHIELD_BACK_COLOR[3] * alpha];

    // Heat shield is drawn on the upper half of the hitbox
    let shield_top = y + hitbox_half_h;
    let shield_bottom = shield_top - visual_h;

    // Backing structure (top 40%) — flat rectangle
    let back_bottom = shield_top - visual_h * 0.4;
    vertices.push(Vertex { position: [x - half_w, back_bottom], color: back_color });
    vertices.push(Vertex { position: [x + half_w, back_bottom], color: back_color });
    vertices.push(Vertex { position: [x + half_w, shield_top], color: back_color });
    vertices.push(Vertex { position: [x - half_w, back_bottom], color: back_color });
    vertices.push(Vertex { position: [x + half_w, shield_top], color: back_color });
    vertices.push(Vertex { position: [x - half_w, shield_top], color: back_color });

    // Ablative face (bottom 60%) — convex dome with curved bottom edge
    let face_top = back_bottom;
    let face_flat_bottom = shield_bottom;
    let sag = visual_h * 0.3; // How far the dome bulges downward
    let segments = 8;

    for i in 0..segments {
        // Theta spans from -PI/2 to PI/2 across the width
        let theta0 = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::PI * (i as f32) / (segments as f32);
        let theta1 = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::PI * ((i + 1) as f32) / (segments as f32);

        // X positions along the width
        let x0 = x + half_w * theta0.sin();
        let x1 = x + half_w * theta1.sin();

        // Bottom edge Y dips by sag * cos(theta) — deepest at center
        let y0_bot = face_flat_bottom - sag * theta0.cos();
        let y1_bot = face_flat_bottom - sag * theta1.cos();

        // Two triangles: top-left to bottom edge segment
        // Triangle 1: top-left, top-right, bottom-right
        vertices.push(Vertex { position: [x0, face_top], color: face_color });
        vertices.push(Vertex { position: [x1, face_top], color: face_color });
        vertices.push(Vertex { position: [x1, y1_bot], color: face_color });

        // Triangle 2: top-left, bottom-right, bottom-left
        vertices.push(Vertex { position: [x0, face_top], color: face_color });
        vertices.push(Vertex { position: [x1, y1_bot], color: face_color });
        vertices.push(Vertex { position: [x0, y0_bot], color: face_color });
    }
}

// RCS colors
const RCS_BODY_COLOR: [f32; 4] = [0.20, 0.20, 0.22, 1.0];     // Dark grey body
const RCS_NOZZLE_COLOR: [f32; 4] = [0.12, 0.12, 0.14, 1.0];   // Darker nozzle tips

/// Generate RCS thruster details — thin side-mount with 3 directional nozzles.
/// Right-mount (default): body on right side of hitbox, nozzles point left/up/down.
/// Left-mount (is_mirrored): body on left side of hitbox, nozzles point right/up/down.
pub fn generate_rcs_details(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    alpha: f32,
) {
    let is_mirrored = def.rcs.as_ref().map(|r| r.is_mirrored).unwrap_or(false);
    // sign: 1.0 = right-mount (sprite on right side, nozzles face left)
    //       -1.0 = left-mount  (sprite on left side, nozzles face right)
    let sign: f32 = if is_mirrored { -1.0 } else { 1.0 };

    let half_w = (def.width() / 2.0) as f32;   // visual half-width (0.125m for 0.5 grid)
    let half_h = (def.height() / 2.0) as f32;  // visual half-height (0.25m for 1.0 grid)
    let hitbox_half_w = (def.hitbox_width() / 2.0) as f32;  // hitbox half-width (0.25m for 1 grid)

    // Offset visual center to the side of the hitbox
    let cx = x + sign * (hitbox_half_w - half_w);

    let body_color = [RCS_BODY_COLOR[0], RCS_BODY_COLOR[1], RCS_BODY_COLOR[2], RCS_BODY_COLOR[3] * alpha];
    let nozzle_color = [RCS_NOZZLE_COLOR[0], RCS_NOZZLE_COLOR[1], RCS_NOZZLE_COLOR[2], RCS_NOZZLE_COLOR[3] * alpha];

    // Body: rectangle covering 80% of visual extents
    let body_hw = half_w * 0.8;
    let body_hh = half_h * 0.8;
    vertices.push(Vertex { position: [cx - body_hw, y - body_hh], color: body_color });
    vertices.push(Vertex { position: [cx + body_hw, y - body_hh], color: body_color });
    vertices.push(Vertex { position: [cx + body_hw, y + body_hh], color: body_color });
    vertices.push(Vertex { position: [cx - body_hw, y - body_hh], color: body_color });
    vertices.push(Vertex { position: [cx + body_hw, y + body_hh], color: body_color });
    vertices.push(Vertex { position: [cx - body_hw, y + body_hh], color: body_color });

    // Nozzle dimensions
    let nozzle_len = half_h * 0.3;
    let nozzle_hw = half_w * 0.35;  // half-width of nozzle base

    // Lateral nozzle: points away from vessel (left for right-mount, right for left-mount)
    let lateral_base_x = cx - sign * body_hw;
    vertices.push(Vertex { position: [lateral_base_x, y - nozzle_hw], color: nozzle_color });
    vertices.push(Vertex { position: [lateral_base_x, y + nozzle_hw], color: nozzle_color });
    vertices.push(Vertex { position: [lateral_base_x - sign * nozzle_len, y], color: nozzle_color });

    // Top nozzle
    vertices.push(Vertex { position: [cx - nozzle_hw, y + body_hh], color: nozzle_color });
    vertices.push(Vertex { position: [cx + nozzle_hw, y + body_hh], color: nozzle_color });
    vertices.push(Vertex { position: [cx, y + body_hh + nozzle_len], color: nozzle_color });

    // Bottom nozzle
    vertices.push(Vertex { position: [cx - nozzle_hw, y - body_hh], color: nozzle_color });
    vertices.push(Vertex { position: [cx + nozzle_hw, y - body_hh], color: nozzle_color });
    vertices.push(Vertex { position: [cx, y - body_hh - nozzle_len], color: nozzle_color });
}

/// Generate white RCS plume vertices for active nozzles.
/// Called at origin (0,0) in part-local space, like engine plumes.
pub fn generate_rcs_plume_vertices(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    nozzle_state: &crate::render::RcsNozzleState,
) {
    let is_mirrored = def.rcs.as_ref().map(|r| r.is_mirrored).unwrap_or(false);
    let sign: f32 = if is_mirrored { -1.0 } else { 1.0 };

    let half_w = (def.width() / 2.0) as f32;
    let half_h = (def.height() / 2.0) as f32;
    let hitbox_half_w = (def.hitbox_width() / 2.0) as f32;
    let cx = x + sign * (hitbox_half_w - half_w);

    let body_hw = half_w * 0.8;
    let body_hh = half_h * 0.8;
    let nozzle_len = half_h * 0.3;
    let nozzle_hw = half_w * 0.35;

    // Plume dimensions: extends outward from nozzle tip
    let plume_len = nozzle_len * 1.5;
    let plume_hw = nozzle_hw * 0.6;

    let plume_color: [f32; 4] = [0.95, 0.95, 1.0, 0.85];

    // Lateral plume
    if nozzle_state.lateral {
        let tip_x = cx - sign * (body_hw + nozzle_len);
        let end_x = tip_x - sign * plume_len;
        // Rectangle as two triangles
        vertices.push(Vertex { position: [tip_x, y - plume_hw], color: plume_color });
        vertices.push(Vertex { position: [tip_x, y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [tip_x, y - plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, y - plume_hw], color: plume_color });
    }

    // Top plume
    if nozzle_state.up {
        let tip_y = y + body_hh + nozzle_len;
        let end_y = tip_y + plume_len;
        vertices.push(Vertex { position: [cx - plume_hw, tip_y], color: plume_color });
        vertices.push(Vertex { position: [cx + plume_hw, tip_y], color: plume_color });
        vertices.push(Vertex { position: [cx + plume_hw, end_y], color: plume_color });
        vertices.push(Vertex { position: [cx - plume_hw, tip_y], color: plume_color });
        vertices.push(Vertex { position: [cx + plume_hw, end_y], color: plume_color });
        vertices.push(Vertex { position: [cx - plume_hw, end_y], color: plume_color });
    }

    // Bottom plume
    if nozzle_state.down {
        let tip_y = y - body_hh - nozzle_len;
        let end_y = tip_y - plume_len;
        vertices.push(Vertex { position: [cx - plume_hw, tip_y], color: plume_color });
        vertices.push(Vertex { position: [cx + plume_hw, tip_y], color: plume_color });
        vertices.push(Vertex { position: [cx + plume_hw, end_y], color: plume_color });
        vertices.push(Vertex { position: [cx - plume_hw, tip_y], color: plume_color });
        vertices.push(Vertex { position: [cx + plume_hw, end_y], color: plume_color });
        vertices.push(Vertex { position: [cx - plume_hw, end_y], color: plume_color });
    }
}

/// Generate RCS plume vertices for pods with built-in bilateral RCS nozzles.
/// Pods have nozzles on both left and right sides near the top.
/// `lateral` = left nozzle fires (exhaust left), `lateral_mirrored` = right nozzle fires (exhaust right).
pub fn generate_pod_rcs_plume_vertices(
    vertices: &mut Vec<Vertex>,
    def: &PartDefinition,
    x: f32,
    y: f32,
    nozzle_state: &crate::render::RcsNozzleState,
) {
    let half_w = (def.width() / 2.0) as f32;
    let half_h = (def.height() / 2.0) as f32;
    let half_top_w = (def.top_width() / 2.0) as f32;

    // Nozzle position matches generate_pod_details: 80% up the pod
    let nozzle_y = y - half_h + half_h * 2.0 * 0.8;
    let t = 0.8;
    let edge_x = half_w + t * (half_top_w - half_w);
    let nozzle_len: f32 = 0.08; // matches nozzle triangle length

    let plume_len: f32 = 0.12;
    let plume_hw: f32 = 0.03;
    let plume_color: [f32; 4] = [0.95, 0.95, 1.0, 0.85];

    // Left nozzle plume (exhaust goes left, from triangle tip)
    if nozzle_state.lateral {
        let tip_x = x - edge_x - nozzle_len;
        let end_x = tip_x - plume_len;
        vertices.push(Vertex { position: [tip_x, nozzle_y - plume_hw], color: plume_color });
        vertices.push(Vertex { position: [tip_x, nozzle_y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, nozzle_y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [tip_x, nozzle_y - plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, nozzle_y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, nozzle_y - plume_hw], color: plume_color });
    }

    // Right nozzle plume (exhaust goes right, from triangle tip)
    if nozzle_state.lateral_mirrored {
        let tip_x = x + edge_x + nozzle_len;
        let end_x = tip_x + plume_len;
        vertices.push(Vertex { position: [tip_x, nozzle_y - plume_hw], color: plume_color });
        vertices.push(Vertex { position: [tip_x, nozzle_y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, nozzle_y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [tip_x, nozzle_y - plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, nozzle_y + plume_hw], color: plume_color });
        vertices.push(Vertex { position: [end_x, nozzle_y - plume_hw], color: plume_color });
    }
}

/// Generate adapter trapezoid connecting the closest aligned fuel tank above to a decoupler ring.
/// Draws from two of the tank's bottom vertices to two of the decoupler's top vertices.
/// draw_x/draw_y are camera-relative coords for rendering; world_x/world_y are world coords for adjacency checks.
fn generate_decoupler_adapter(
    vertices: &mut Vec<Vertex>,
    decoupler_def: &PartDefinition,
    draw_x: f32,
    draw_y: f32,
    world_x: f32,
    world_y: f32,
    parts: &std::collections::HashMap<crate::parts::PlacedPartId, crate::parts::PlacedPart>,
    part_defs: &PartDefinitions,
    alpha: f32,
) {
    let decoupler_hitbox_half_h = (decoupler_def.hitbox_height() / 2.0) as f32;
    let decoupler_visual_h = decoupler_def.height() as f32;
    let decoupler_half_w = (decoupler_def.width() / 2.0) as f32;
    let decoupler_ring_top_world = world_y - decoupler_hitbox_half_h + decoupler_visual_h;

    let tolerance = 0.01;

    // Find the closest aligned fuel tank above the decoupler ring
    let mut best_dist = f32::MAX;
    let mut best_tank_bottom_world: f32 = 0.0;
    let mut best_tank_half_w: f32 = 0.0;

    for (_, other_part) in parts {
        let Some(other_def) = part_defs.get(&other_part.definition_id) else {
            continue;
        };

        if other_def.tank.is_none() && other_def.pod.is_none() {
            continue;
        }

        let other_x = other_part.position[0] as f32;
        let other_y = other_part.position[1] as f32;

        // Must be aligned (same center x)
        if (other_x - world_x).abs() > tolerance {
            continue;
        }

        // Tank must be above the decoupler ring top
        let tank_bottom = other_y - (other_def.hitbox_height() / 2.0) as f32;
        if tank_bottom < decoupler_ring_top_world - tolerance {
            continue;
        }

        // Pick closest
        let dist = tank_bottom - decoupler_ring_top_world;
        if dist < best_dist {
            best_dist = dist;
            best_tank_bottom_world = tank_bottom;
            best_tank_half_w = (other_def.width() / 2.0) as f32;
        }
    }

    if best_dist == f32::MAX {
        return;
    }

    // Convert tank bottom from world to camera-relative
    let cam_offset_y = world_y - draw_y;
    let adapter_bottom_draw = draw_y - decoupler_hitbox_half_h + decoupler_visual_h;
    let adapter_top_draw = best_tank_bottom_world - cam_offset_y;

    let adapter_color = [PART_COLOR[0], PART_COLOR[1], PART_COLOR[2], PART_COLOR[3] * alpha];

    // Bottom edge = decoupler ring top (decoupler width), top edge = tank bottom (tank width)
    vertices.push(Vertex { position: [draw_x - decoupler_half_w, adapter_bottom_draw], color: adapter_color });
    vertices.push(Vertex { position: [draw_x + decoupler_half_w, adapter_bottom_draw], color: adapter_color });
    vertices.push(Vertex { position: [draw_x + best_tank_half_w, adapter_top_draw], color: adapter_color });

    vertices.push(Vertex { position: [draw_x - decoupler_half_w, adapter_bottom_draw], color: adapter_color });
    vertices.push(Vertex { position: [draw_x + best_tank_half_w, adapter_top_draw], color: adapter_color });
    vertices.push(Vertex { position: [draw_x - best_tank_half_w, adapter_top_draw], color: adapter_color });

    // Draw detail lines on the adapter surface.
    // Model as a frustum viewed from the side: lines are evenly spaced in angle,
    // so near edges they follow the trapezoid slope and near center they're vertical.
    // 3 lines per grid square of the decoupler's width.
    let num_lines = (decoupler_def.grid_width * 3.0).round() as u32;
    if num_lines == 0 {
        return;
    }
    let line_color = [0.18, 0.18, 0.20, alpha];
    let line_half_thickness = 0.008_f32;

    for i in 0..num_lines {
        // Evenly space in angle across the visible half-cylinder (-π/2 to π/2)
        let theta = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::PI * (i as f32 + 1.0) / (num_lines as f32 + 1.0);
        let s = theta.sin();

        // Bottom x (at decoupler ring top) and top x (at tank bottom)
        let bot_x = draw_x + decoupler_half_w * s;
        let top_x = draw_x + best_tank_half_w * s;

        // Direction vector of the line
        let dx = top_x - bot_x;
        let dy = adapter_top_draw - adapter_bottom_draw;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.0001 {
            continue;
        }
        // Perpendicular for thickness
        let nx = -dy / len * line_half_thickness;
        let ny = dx / len * line_half_thickness;

        // Quad for the line
        let bl = [bot_x - nx, adapter_bottom_draw - ny];
        let br = [bot_x + nx, adapter_bottom_draw + ny];
        let tr = [top_x + nx, adapter_top_draw + ny];
        let tl = [top_x - nx, adapter_top_draw - ny];

        vertices.push(Vertex { position: bl, color: line_color });
        vertices.push(Vertex { position: br, color: line_color });
        vertices.push(Vertex { position: tr, color: line_color });

        vertices.push(Vertex { position: bl, color: line_color });
        vertices.push(Vertex { position: tr, color: line_color });
        vertices.push(Vertex { position: tl, color: line_color });
    }
}

/// Generate adapter trapezoid for flight rendering.
/// Works with ShipPartRenderData (local coordinates relative to vessel COM).
pub fn generate_flight_decoupler_adapter(
    vertices: &mut Vec<Vertex>,
    decoupler_def: &PartDefinition,
    dec_x: f32,
    dec_y: f32,
    parts: &[crate::render::ShipPartRenderData],
    part_defs: &PartDefinitions,
    alpha: f32,
) {
    let decoupler_hitbox_half_h = (decoupler_def.hitbox_height() / 2.0) as f32;
    let decoupler_visual_h = decoupler_def.height() as f32;
    let decoupler_half_w = (decoupler_def.width() / 2.0) as f32;
    let decoupler_ring_top = dec_y - decoupler_hitbox_half_h + decoupler_visual_h;

    let tolerance = 0.01;

    // Find the closest aligned fuel tank above the decoupler ring
    let mut best_dist = f32::MAX;
    let mut best_tank_bottom: f32 = 0.0;
    let mut best_tank_half_w: f32 = 0.0;

    for other in parts {
        let Some(other_def) = part_defs.get(&other.definition_id) else {
            continue;
        };

        if other_def.tank.is_none() && other_def.pod.is_none() {
            continue;
        }

        let other_x = other.local_x as f32;
        let other_y = other.local_y as f32;

        // Must be aligned (same center x)
        if (other_x - dec_x).abs() > tolerance {
            continue;
        }

        // Tank must be above the decoupler ring top
        let tank_bottom = other_y - (other_def.hitbox_height() / 2.0) as f32;
        if tank_bottom < decoupler_ring_top - tolerance {
            continue;
        }

        let dist = tank_bottom - decoupler_ring_top;
        if dist < best_dist {
            best_dist = dist;
            best_tank_bottom = tank_bottom;
            best_tank_half_w = (other_def.width() / 2.0) as f32;
        }
    }

    if best_dist == f32::MAX {
        return;
    }

    let adapter_bottom = decoupler_ring_top;
    let adapter_top = best_tank_bottom;

    let adapter_color = [PART_COLOR[0], PART_COLOR[1], PART_COLOR[2], PART_COLOR[3] * alpha];

    // Trapezoid: bottom = decoupler width, top = tank width
    vertices.push(Vertex { position: [dec_x - decoupler_half_w, adapter_bottom], color: adapter_color });
    vertices.push(Vertex { position: [dec_x + decoupler_half_w, adapter_bottom], color: adapter_color });
    vertices.push(Vertex { position: [dec_x + best_tank_half_w, adapter_top], color: adapter_color });

    vertices.push(Vertex { position: [dec_x - decoupler_half_w, adapter_bottom], color: adapter_color });
    vertices.push(Vertex { position: [dec_x + best_tank_half_w, adapter_top], color: adapter_color });
    vertices.push(Vertex { position: [dec_x - best_tank_half_w, adapter_top], color: adapter_color });

    // Detail lines (same frustum projection as editor)
    let num_lines = (decoupler_def.grid_width * 3.0).round() as u32;
    if num_lines == 0 {
        return;
    }
    let line_color = [0.18, 0.18, 0.20, alpha];
    let line_half_thickness = 0.008_f32;

    for i in 0..num_lines {
        let theta = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::PI * (i as f32 + 1.0) / (num_lines as f32 + 1.0);
        let s = theta.sin();

        let bot_x = dec_x + decoupler_half_w * s;
        let top_x = dec_x + best_tank_half_w * s;

        let dx = top_x - bot_x;
        let dy = adapter_top - adapter_bottom;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.0001 {
            continue;
        }
        let nx = -dy / len * line_half_thickness;
        let ny = dx / len * line_half_thickness;

        let bl = [bot_x - nx, adapter_bottom - ny];
        let br = [bot_x + nx, adapter_bottom + ny];
        let tr = [top_x + nx, adapter_top + ny];
        let tl = [top_x - nx, adapter_top - ny];

        vertices.push(Vertex { position: bl, color: line_color });
        vertices.push(Vertex { position: br, color: line_color });
        vertices.push(Vertex { position: tr, color: line_color });

        vertices.push(Vertex { position: bl, color: line_color });
        vertices.push(Vertex { position: tr, color: line_color });
        vertices.push(Vertex { position: tl, color: line_color });
    }
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

    // For heat shields, use dedicated rendering
    if def.is_heat_shield {
        generate_heat_shield_details(vertices, def, x, y, alpha);
        return;
    }

    // For decouplers, draw the ring (adapter needs parts list, handled separately)
    if def.decoupler.is_some() {
        generate_decoupler_details(vertices, def, x, y, alpha);
        return;
    }

    // For RCS thrusters
    if def.rcs.is_some() {
        generate_rcs_details(vertices, def, x, y, alpha);
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
        PartShape::TriangleRight => {
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y + half_h], color });
        }
        PartShape::TriangleLeft => {
            vertices.push(Vertex { position: [x - half_w, y - half_h], color });
            vertices.push(Vertex { position: [x + half_w, y - half_h], color });
            vertices.push(Vertex { position: [x - half_w, y + half_h], color });
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
