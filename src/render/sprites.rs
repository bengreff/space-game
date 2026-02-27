//! Sprite atlas — loads part/engine/plume PNGs, packs into a single GPU texture,
//! and provides part ID → UV rect mapping.

use std::collections::HashMap;
use std::path::Path;

/// Interstellar engine IDs — excluded from atlas (use procedural fallback)
const INTERSTELLAR_ENGINES: &[&str] = &[
    "engine_orion_pulse",
    "engine_daedalus_s1",
    "engine_daedalus_s2",
    "engine_zpinch_advanced",
    "engine_zpinch_probe",
    "engine_amcat_fusion",
    "engine_am_torch",
    "engine_gamma_conversion",
];

/// UV rectangle in the atlas (normalized 0–1 coordinates)
#[derive(Clone, Debug)]
pub struct SpriteRect {
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

/// 4-frame plume animation
#[derive(Clone, Debug)]
pub struct PlumeAnimation {
    pub frames: [SpriteRect; 4],
}

/// Sprite atlas: single GPU texture + lookup tables
pub struct SpriteAtlas {
    pub bind_group: wgpu::BindGroup,
    pub parts: HashMap<String, SpriteRect>,
    pub plumes: HashMap<String, PlumeAnimation>,
}

/// Entry for atlas packing (before GPU upload)
struct SpriteEntry {
    id: String,
    image: image::RgbaImage,
    width: u32,
    height: u32,
    /// "part", "engine", or "plume:<propellant>:<frame>"
    kind: String,
}

/// Packed position result
struct PackedSprite {
    id: String,
    kind: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// Round up to next power of 2
fn next_power_of_2(v: u32) -> u32 {
    let mut p = 1;
    while p < v {
        p *= 2;
    }
    p
}

/// Shelf-based atlas packer
fn shelf_pack(entries: &mut Vec<SpriteEntry>, atlas_width: u32) -> (Vec<PackedSprite>, u32) {
    // Sort by height descending for better shelf packing
    entries.sort_by(|a, b| b.height.cmp(&a.height));

    let mut packed = Vec::new();
    let mut shelf_y: u32 = 0;
    let mut shelf_height: u32 = 0;
    let mut cursor_x: u32 = 0;
    let padding = 1u32; // 1px padding between sprites to avoid bleeding

    for entry in entries.iter() {
        let w = entry.width + padding;
        let h = entry.height + padding;

        // Does it fit in the current shelf?
        if cursor_x + w > atlas_width {
            // Start new shelf
            shelf_y += shelf_height;
            shelf_height = 0;
            cursor_x = 0;
        }

        packed.push(PackedSprite {
            id: entry.id.clone(),
            kind: entry.kind.clone(),
            x: cursor_x,
            y: shelf_y,
            width: entry.width,
            height: entry.height,
        });

        cursor_x += w;
        shelf_height = shelf_height.max(h);
    }

    let total_height = shelf_y + shelf_height;
    let atlas_height = next_power_of_2(total_height);
    (packed, atlas_height)
}

/// Load all sprites and create the atlas
pub fn load_sprite_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> SpriteAtlas {
    let sprite_dir = Path::new("data/sprites");
    let mut entries: Vec<SpriteEntry> = Vec::new();

    // Load engine sprites (skip interstellar)
    load_dir_sprites(&mut entries, &sprite_dir.join("engines"), "engine", |stem| {
        !INTERSTELLAR_ENGINES.contains(&stem)
    });

    // Load part sprites
    load_dir_sprites(&mut entries, &sprite_dir.join("parts"), "part", |_| true);

    // Load plume sprites (kerolox_frame0..3, etc.)
    load_plume_sprites(&mut entries, &sprite_dir.join("plumes"));

    if entries.is_empty() {
        log::warn!("No sprite files found, creating dummy sprite atlas");
        return create_dummy_atlas(device, queue);
    }

    log::info!("Packing {} sprites into atlas", entries.len());

    let max_dim = device.limits().max_texture_dimension_2d;
    let atlas_width = max_dim.min(8192);
    let (mut packed, mut atlas_height) = shelf_pack(&mut entries, atlas_width);

    // If atlas is too tall, downscale all sprites and repack
    while atlas_height > max_dim {
        log::warn!(
            "Sprite atlas {}x{} exceeds GPU limit {}, downscaling sprites by 50%",
            atlas_width, atlas_height, max_dim
        );
        for entry in entries.iter_mut() {
            let new_w = (entry.width / 2).max(1);
            let new_h = (entry.height / 2).max(1);
            entry.image = image::imageops::resize(
                &entry.image, new_w, new_h, image::imageops::FilterType::Lanczos3,
            );
            entry.width = new_w;
            entry.height = new_h;
        }
        let result = shelf_pack(&mut entries, atlas_width);
        packed = result.0;
        atlas_height = result.1;
    }

    log::info!("Sprite atlas: {}x{}", atlas_width, atlas_height);

    // Create RGBA buffer and blit sprites
    let mut atlas_data = vec![0u8; (atlas_width * atlas_height * 4) as usize];

    // Build a map from id to image for blitting
    let image_map: HashMap<String, &image::RgbaImage> = entries.iter()
        .map(|e| (e.id.clone(), &e.image))
        .collect();

    for p in &packed {
        let img = image_map[&p.id];
        for row in 0..p.height {
            let src_offset = (row * p.width * 4) as usize;
            let dst_offset = ((p.y + row) * atlas_width * 4 + p.x * 4) as usize;
            let row_bytes = (p.width * 4) as usize;
            atlas_data[dst_offset..dst_offset + row_bytes]
                .copy_from_slice(&img.as_raw()[src_offset..src_offset + row_bytes]);
        }
    }

    // Create GPU texture
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Sprite Atlas"),
        size: wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &atlas_data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * atlas_width),
            rows_per_image: Some(atlas_height),
        },
        wgpu::Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Sprite Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let bind_group_layout = create_sprite_bind_group_layout(device);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Sprite Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    // Build lookup tables
    let mut parts: HashMap<String, SpriteRect> = HashMap::new();
    let mut plume_frames: HashMap<String, Vec<(usize, SpriteRect)>> = HashMap::new();

    let aw = atlas_width as f32;
    let ah = atlas_height as f32;

    for p in &packed {
        let rect = SpriteRect {
            u_min: p.x as f32 / aw,
            v_min: p.y as f32 / ah,
            u_max: (p.x + p.width) as f32 / aw,
            v_max: (p.y + p.height) as f32 / ah,
        };

        if p.kind.starts_with("plume:") {
            // "plume:kerolox:2" -> propellant="kerolox", frame=2
            let parts_str: Vec<&str> = p.kind.splitn(3, ':').collect();
            if parts_str.len() == 3 {
                let propellant = parts_str[1].to_string();
                let frame: usize = parts_str[2].parse().unwrap_or(0);
                plume_frames.entry(propellant).or_default().push((frame, rect));
            }
        } else {
            parts.insert(p.id.clone(), rect);
        }
    }

    // Assemble plume animations
    let mut plumes: HashMap<String, PlumeAnimation> = HashMap::new();
    for (propellant, mut frames) in plume_frames {
        frames.sort_by_key(|(idx, _)| *idx);
        if frames.len() >= 4 {
            let dummy = SpriteRect { u_min: 0.0, v_min: 0.0, u_max: 0.0, v_max: 0.0 };
            let mut anim_frames = [dummy.clone(), dummy.clone(), dummy.clone(), dummy];
            for (idx, rect) in frames.into_iter().take(4) {
                anim_frames[idx] = rect;
            }
            plumes.insert(propellant, PlumeAnimation { frames: anim_frames });
        }
    }

    log::info!("Sprite atlas loaded: {} parts, {} plume types", parts.len(), plumes.len());

    SpriteAtlas { bind_group, parts, plumes }
}

/// Create the bind group layout for sprite atlas (group 2)
pub fn create_sprite_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("sprite_bind_group_layout"),
    })
}

/// Load PNG sprites from a directory
fn load_dir_sprites(
    entries: &mut Vec<SpriteEntry>,
    dir: &Path,
    kind: &str,
    filter: impl Fn(&str) -> bool,
) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        log::warn!("Cannot read sprite dir: {}", dir.display());
        return;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        if !filter(&stem) {
            continue;
        }

        match image::open(&path) {
            Ok(img) => {
                let rgba = img.into_rgba8();
                let (w, h) = rgba.dimensions();
                entries.push(SpriteEntry {
                    id: stem,
                    image: rgba,
                    width: w,
                    height: h,
                    kind: kind.to_string(),
                });
            }
            Err(e) => {
                log::warn!("Failed to load sprite {}: {}", path.display(), e);
            }
        }
    }
}

/// Load plume animation sprites (e.g. kerolox_frame0.png)
fn load_plume_sprites(entries: &mut Vec<SpriteEntry>, dir: &Path) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        log::warn!("Cannot read plume dir: {}", dir.display());
        return;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();

        // Parse "kerolox_frame2" -> propellant="kerolox", frame=2
        if let Some(idx) = stem.rfind("_frame") {
            let propellant = &stem[..idx];
            let frame_str = &stem[idx + 6..];
            if let Ok(frame) = frame_str.parse::<usize>() {
                match image::open(&path) {
                    Ok(img) => {
                        let rgba = img.into_rgba8();
                        let (w, h) = rgba.dimensions();
                        entries.push(SpriteEntry {
                            id: stem.clone(),
                            image: rgba,
                            width: w,
                            height: h,
                            kind: format!("plume:{}:{}", propellant, frame),
                        });
                    }
                    Err(e) => {
                        log::warn!("Failed to load plume sprite {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
}

/// Create a dummy 1x1 atlas when no sprites are found
fn create_dummy_atlas(device: &wgpu::Device, queue: &wgpu::Queue) -> SpriteAtlas {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Dummy Sprite Atlas"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[0u8; 4],
        wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
        wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Dummy Sprite Sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let layout = create_sprite_bind_group_layout(device);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Dummy Sprite Bind Group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
    });

    SpriteAtlas {
        bind_group,
        parts: HashMap::new(),
        plumes: HashMap::new(),
    }
}
