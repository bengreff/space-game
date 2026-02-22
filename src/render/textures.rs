//! Body texture loading — loads pre-baked polar projection PNGs from data/textures/bodies/,
//! builds a texture_2d_array + sampler, and provides a mapping from body index → layer index.

use std::collections::HashMap;
use std::path::Path;

const TEXTURE_SIZE: u32 = 1024;

/// Bleed disc-edge colors outward so GPU bilinear filtering at the body edge blends with
/// valid colors instead of the black pixels outside the polar projection disc.
fn bleed_disc_edges(img: &mut image::RgbaImage) {
    let (w, h) = img.dimensions();
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let disc_r = w as f64 / 2.0;
    let margin = 4.0; // covers AA zone + bilinear sampling radius

    for y in 0..h {
        for x in 0..w {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist >= disc_r - margin && dist > 0.0 {
                // Sample from a clean interior point at the same angle
                let scale = (disc_r - margin - 1.0) / dist;
                let src_x = (cx + dx * scale).round() as u32;
                let src_y = (cy + dy * scale).round() as u32;
                if src_x < w && src_y < h {
                    let color = *img.get_pixel(src_x, src_y);
                    img.put_pixel(x, y, color);
                }
            }
        }
    }
}

/// Maps body index to texture array layer index. None means no texture for that body.
pub struct BodyTextureMap {
    layers: HashMap<usize, u32>,
}

impl BodyTextureMap {
    pub fn layer_for_body(&self, body_index: usize) -> Option<u32> {
        self.layers.get(&body_index).copied()
    }
}

/// Load body textures from `data/textures/bodies/<name>.{png,jpg}` and create a texture array.
/// Textures should be pre-baked 1024x1024 north-pole polar projections.
/// Returns the texture view, sampler, and body→layer mapping.
/// If no textures are found, creates a 1-layer dummy texture so the bind group is always valid.
pub fn load_body_textures(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    body_names: &[String],
) -> (wgpu::TextureView, wgpu::Sampler, BodyTextureMap) {
    let texture_dir = Path::new("data/textures/bodies");
    let mut images: Vec<(usize, image::RgbaImage)> = Vec::new();

    for (i, name) in body_names.iter().enumerate() {
        let lower = name.to_lowercase();
        let path = ["png", "jpg", "jpeg"].iter()
            .map(|ext| texture_dir.join(format!("{}.{}", lower, ext)))
            .find(|p| p.exists());

        if let Some(path) = path {
            match image::open(&path) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let mut final_img = if w != TEXTURE_SIZE || h != TEXTURE_SIZE {
                        image::imageops::resize(&rgba, TEXTURE_SIZE, TEXTURE_SIZE, image::imageops::FilterType::Triangle)
                    } else {
                        rgba
                    };
                    bleed_disc_edges(&mut final_img);
                    log::info!("Loaded body texture: {} (layer {})", path.display(), images.len());
                    images.push((i, final_img));
                }
                Err(e) => {
                    log::warn!("Failed to load body texture {}: {}", path.display(), e);
                }
            }
        }
    }

    let layer_count = images.len().max(1) as u32;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Body Texture Array"),
        size: wgpu::Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: layer_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let mut layers = HashMap::new();
    for (layer_idx, (body_idx, img)) in images.iter().enumerate() {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer_idx as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            img.as_raw(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * TEXTURE_SIZE),
                rows_per_image: Some(TEXTURE_SIZE),
            },
            wgpu::Extent3d {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );
        layers.insert(*body_idx, layer_idx as u32);
    }

    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Body Texture Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    (view, sampler, BodyTextureMap { layers })
}
