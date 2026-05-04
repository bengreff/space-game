//! Offline asset pipeline.
//!
//! For wasm targets we pre-pack the sprite atlas at compile time so the runtime
//! can `include_bytes!` it instead of walking the filesystem and decoding 185
//! PNGs in serial in the browser.
//!
//! The packer mirrors the runtime logic in `src/render/sprites.rs` but caps the
//! atlas at 4096px (a width + height that's safe across WebGL2, Safari iOS, and
//! WebGPU). On native builds this entire script is a no-op.

use std::collections::HashMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const ATLAS_MAX_DIM: u32 = 4096;
const MAX_SPRITE_DIM: u32 = 4096;
const PADDING: u32 = 1;

fn main() {
    println!("cargo:rerun-if-changed=data/sprites");
    println!("cargo:rerun-if-changed=build.rs");

    let target = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target != "wasm32" {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by cargo"));
    let sprite_dir = Path::new("data/sprites");
    if !sprite_dir.exists() {
        eprintln!("data/sprites missing; skipping wasm atlas pack");
        return;
    }

    pack_atlas(sprite_dir, &out_dir);
}

struct SpriteEntry {
    id: String,
    image: image::RgbaImage,
    width: u32,
    height: u32,
    /// "part" / "engine" / "plume:<propellant>:<frame>"
    kind: String,
}

struct PackedSprite {
    id: String,
    kind: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn pack_atlas(sprite_dir: &Path, out_dir: &Path) {
    let mut entries = Vec::new();
    entries.extend(collect_dir(&sprite_dir.join("engines"), "engine"));
    entries.extend(collect_dir(&sprite_dir.join("parts"), "part"));
    entries.extend(collect_plumes(&sprite_dir.join("plumes")));
    if entries.is_empty() {
        eprintln!("no sprites found under data/sprites");
        return;
    }

    // Cap individual sprite size before packing.
    for e in entries.iter_mut() {
        if e.width > MAX_SPRITE_DIM || e.height > MAX_SPRITE_DIM {
            let scale = MAX_SPRITE_DIM as f64 / e.width.max(e.height) as f64;
            let nw = ((e.width as f64 * scale) as u32).max(1);
            let nh = ((e.height as f64 * scale) as u32).max(1);
            e.image = image::imageops::resize(&e.image, nw, nh, image::imageops::FilterType::Lanczos3);
            e.width = nw;
            e.height = nh;
        }
    }

    let atlas_width = ATLAS_MAX_DIM;
    let (mut packed, mut atlas_height) = shelf_pack(&mut entries, atlas_width);
    while atlas_height > ATLAS_MAX_DIM {
        eprintln!("atlas {}x{} exceeds {}, downscaling 50%", atlas_width, atlas_height, ATLAS_MAX_DIM);
        for e in entries.iter_mut() {
            let nw = (e.width / 2).max(1);
            let nh = (e.height / 2).max(1);
            e.image = image::imageops::resize(&e.image, nw, nh, image::imageops::FilterType::Nearest);
            e.width = nw;
            e.height = nh;
        }
        let r = shelf_pack(&mut entries, atlas_width);
        packed = r.0;
        atlas_height = r.1;
    }

    let mut atlas = image::RgbaImage::new(atlas_width, atlas_height);
    let by_id: HashMap<String, &image::RgbaImage> =
        entries.iter().map(|e| (e.id.clone(), &e.image)).collect();
    for p in &packed {
        let img = by_id[&p.id];
        for y in 0..p.height {
            for x in 0..p.width {
                let px = img.get_pixel(x, y);
                atlas.put_pixel(p.x + x, p.y + y, *px);
            }
        }
    }

    let png_path = out_dir.join("sprite_atlas.png");
    atlas.save(&png_path).unwrap_or_else(|e| panic!("save {}: {}", png_path.display(), e));

    let ron = build_metadata_ron(&packed, atlas_width, atlas_height);
    let ron_path = out_dir.join("sprite_atlas.ron");
    fs::write(&ron_path, ron).unwrap_or_else(|e| panic!("write {}: {}", ron_path.display(), e));

    eprintln!(
        "packed atlas {}x{} ({} sprites) -> {}",
        atlas_width, atlas_height, packed.len(), png_path.display()
    );
}

fn collect_dir(dir: &Path, kind: &str) -> Vec<SpriteEntry> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") { continue; }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.into_rgba8();
                let (w, h) = rgba.dimensions();
                out.push(SpriteEntry { id: stem, image: rgba, width: w, height: h, kind: kind.to_string() });
            }
            Err(e) => eprintln!("decode {}: {}", path.display(), e),
        }
    }
    out
}

fn collect_plumes(dir: &Path) -> Vec<SpriteEntry> {
    let Ok(rd) = fs::read_dir(dir) else { return Vec::new() };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("png") { continue; }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let Some(idx) = stem.rfind("_frame") else { continue };
        let propellant = &stem[..idx];
        let Ok(frame) = stem[idx + 6..].parse::<usize>() else { continue };
        let kind = format!("plume:{}:{}", propellant, frame);
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.into_rgba8();
                let (w, h) = rgba.dimensions();
                out.push(SpriteEntry { id: stem, image: rgba, width: w, height: h, kind });
            }
            Err(e) => eprintln!("decode plume {}: {}", path.display(), e),
        }
    }
    out
}

fn shelf_pack(entries: &mut Vec<SpriteEntry>, atlas_width: u32) -> (Vec<PackedSprite>, u32) {
    entries.sort_by(|a, b| b.height.cmp(&a.height));
    let mut packed = Vec::new();
    let mut shelf_y = 0u32;
    let mut shelf_height = 0u32;
    let mut cursor_x = 0u32;
    for e in entries.iter() {
        let w = e.width + PADDING;
        let h = e.height + PADDING;
        if cursor_x + w > atlas_width {
            shelf_y += shelf_height;
            shelf_height = 0;
            cursor_x = 0;
        }
        packed.push(PackedSprite {
            id: e.id.clone(),
            kind: e.kind.clone(),
            x: cursor_x,
            y: shelf_y,
            width: e.width,
            height: e.height,
        });
        cursor_x += w;
        shelf_height = shelf_height.max(h);
    }
    let total = shelf_y + shelf_height;
    let mut p = 1u32;
    while p < total { p *= 2; }
    (packed, p)
}

fn build_metadata_ron(packed: &[PackedSprite], aw: u32, ah: u32) -> String {
    let aw_f = aw as f32;
    let ah_f = ah as f32;

    let mut parts = Vec::new();
    let mut plume_frames: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    for p in packed {
        let u_min = p.x as f32 / aw_f;
        let v_min = p.y as f32 / ah_f;
        let u_max = (p.x + p.width) as f32 / aw_f;
        let v_max = (p.y + p.height) as f32 / ah_f;
        let rect = format!(
            "(u_min: {}, v_min: {}, u_max: {}, v_max: {}, pixel_width: {}, pixel_height: {})",
            u_min, v_min, u_max, v_max, p.width, p.height
        );
        if let Some(rest) = p.kind.strip_prefix("plume:") {
            let mut split = rest.splitn(2, ':');
            let propellant = split.next().unwrap_or("").to_string();
            let frame = split.next().and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
            plume_frames.entry(propellant).or_default().push((frame, rect));
        } else {
            parts.push((p.id.clone(), rect));
        }
    }

    parts.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    let _ = writeln!(out, "(");
    let _ = writeln!(out, "    atlas_width: {},", aw);
    let _ = writeln!(out, "    atlas_height: {},", ah);
    let _ = writeln!(out, "    parts: {{");
    for (id, rect) in &parts {
        let _ = writeln!(out, "        \"{}\": {},", id, rect);
    }
    let _ = writeln!(out, "    }},");
    let _ = writeln!(out, "    plumes: {{");
    let mut plumes_sorted: Vec<_> = plume_frames.into_iter().collect();
    plumes_sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for (propellant, mut frames) in plumes_sorted {
        if frames.len() < 4 { continue; }
        frames.sort_by_key(|(idx, _)| *idx);
        // `frames: [SpriteRect; 4]` serializes as a RON tuple, not a sequence,
        // so emit `frames: ((...), (...), (...), (...))` not `frames: [...]`.
        let _ = writeln!(out, "        \"{}\": (frames: (", propellant);
        for (_, rect) in frames.iter().take(4) {
            let _ = writeln!(out, "            {},", rect);
        }
        let _ = writeln!(out, "        )),");
    }
    let _ = writeln!(out, "    }},");
    let _ = writeln!(out, ")");
    out
}
