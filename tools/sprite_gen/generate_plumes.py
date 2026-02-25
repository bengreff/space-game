#!/usr/bin/env python3
"""
Generate engine plume animation sprites for Sunscatter.

3 propellants (Kerolox, Methalox, Hydrolox) x 4 animation frames = 12 PNGs.
Each plume has layered structure: hot core + mid zone + turbulent outer envelope.
Uses noise-displaced boundaries + Gaussian blur for billowy-but-smooth look.

Reference nozzle width: 200px. Game scales to match actual engine nozzle.
"""

from PIL import Image, ImageDraw, ImageFilter
import math
import os
import random

PAD = 6
NUM_FRAMES = 4
NOZZLE_W = 200

# Canvas: room for expansion + noise + blur margin
CANVAS_W = 320
CANVAS_H = 800

# ================================================================
# Propellant color palettes: (R, G, B, A)
# ================================================================

PALETTES = {
    "kerolox": {
        "core":  (255, 255, 210, 255),       # white-yellow
        "mid":   (255, 165,  50, 235),        # bright orange
        "outer": (205,  75,  15, 160),        # deep orange-red
    },
    "methalox": {
        "core":  (235, 228, 255, 255),        # white with subtle purple
        "mid":   (175, 145, 220, 225),        # muted purple
        "outer": (125,  90, 190, 130),        # desaturated purple
    },
    "hydrolox": {
        "core":  (245, 248, 255, 235),        # near-white
        "mid":   (190, 210, 255, 140),        # very pale blue
        "outer": (135, 170, 250,  60),        # very transparent pale blue
    },
}


# ================================================================
# Noise
# ================================================================

def _hash(ix, iy, seed):
    h = (ix * 374761393 + iy * 668265263 + seed) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    return (h & 0xFFFF) / 65535.0 * 2.0 - 1.0


def value_noise(x, seed, scale):
    """1D value noise with smoothstep interpolation."""
    sx = x * scale
    ix = int(math.floor(sx))
    fx = sx - ix
    fx = fx * fx * (3 - 2 * fx)  # smoothstep
    v0 = _hash(ix, 0, seed)
    v1 = _hash(ix + 1, 0, seed)
    return v0 + fx * (v1 - v0)


def fbm(x, seed, octaves=3, scale=0.015, lacunarity=2.0, gain=0.5):
    """Fractal Brownian motion — sum of noise at multiple scales."""
    val = 0.0
    amp = 1.0
    s = scale
    for i in range(octaves):
        val += amp * value_noise(x, seed + i * 1337, s)
        amp *= gain
        s *= lacunarity
    return val


# ================================================================
# Plume shape
# ================================================================

def plume_taper(frac):
    """Relaxed plume envelope: slight expansion near nozzle, then gradual taper."""
    if frac < 0.08:
        # Expand slightly near nozzle exit
        return 1.0 + 0.25 * (frac / 0.08)
    elif frac < 0.25:
        # Settle back toward nozzle width
        return 1.25 - 0.2 * ((frac - 0.08) / 0.17)
    else:
        # Stay wide, taper late
        t = (frac - 0.25) / 0.75
        return 1.05 * max(0.0, 1.0 - t ** 1.8)


def generate_boundary(plume_h, nozzle_hw, width_fn, seed, noise_amp, noise_scale):
    """Generate left and right half-width arrays with noise displacement.

    width_fn: callable(frac) -> multiplier relative to base envelope.
    Returns: (left_hw[], right_hw[]) arrays of length plume_h.
    """
    left = []
    right = []
    for row in range(plume_h):
        frac = row / max(1, plume_h - 1)
        base_hw = plume_taper(frac) * nozzle_hw * width_fn(frac)

        # Noise envelope: smooth at top, increasingly turbulent toward bottom
        noise_env = frac ** 0.6

        # Large-scale billowy noise (different seeds for left/right -> asymmetry)
        nl = fbm(row, seed, octaves=3, scale=noise_scale) * noise_amp * noise_env
        nr = fbm(row, seed + 7777, octaves=3, scale=noise_scale) * noise_amp * noise_env

        # Smaller-scale texture on top
        nl += fbm(row, seed + 500, octaves=2, scale=noise_scale * 3) * noise_amp * 0.25 * noise_env
        nr += fbm(row, seed + 8277, octaves=2, scale=noise_scale * 3) * noise_amp * 0.25 * noise_env

        left.append(max(0.0, base_hw + nl))
        right.append(max(0.0, base_hw + nr))

    return left, right


# ================================================================
# Drawing
# ================================================================

def draw_layer(cw, ch, left_hw, right_hw, color_rgba, cx, y0, plume_h, tip_power=1.3):
    """Draw a single plume layer as row-by-row horizontal lines."""
    img = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    base_alpha = color_rgba[3]
    rgb = color_rgba[:3]

    for row in range(plume_h):
        y = y0 + row
        frac = row / max(1, plume_h - 1)

        lw = left_hw[row]
        rw = right_hw[row]
        if lw < 0.5 and rw < 0.5:
            continue

        # Alpha fades toward tip
        tip_fade = max(0.0, 1.0 - frac ** tip_power)
        a = int(base_alpha * tip_fade)
        if a < 2:
            continue

        x1 = max(0, int(cx - lw))
        x2 = min(cw - 1, int(cx + rw))
        if x1 >= x2:
            continue

        d.line([(x1, y), (x2, y)], fill=(*rgb, a), width=1)

    return img


def generate_plume_frame(propellant, frame_idx):
    """Generate a single plume animation frame."""
    palette = PALETTES[propellant]
    rng = random.Random(propellant + str(frame_idx))
    seed = rng.randint(0, 1000000)

    cw, ch = CANVAS_W, CANVAS_H
    plume_h = ch - PAD * 2
    nozzle_hw = NOZZLE_W / 2.0
    cx = cw / 2.0
    y0 = PAD

    # Layer width functions (fraction of base envelope at each height)
    # Core is wider near nozzle, narrows further down
    def core_width(frac):
        return 0.35 - 0.15 * frac

    def mid_width(frac):
        return 0.58 - 0.10 * frac

    def outer_width(frac):
        return 1.0

    # Noise amplitudes and scale
    outer_amp = nozzle_hw * 0.30
    mid_amp = nozzle_hw * 0.18
    core_amp = nozzle_hw * 0.10
    noise_scale = 0.013

    # Generate boundaries for each layer
    outer_l, outer_r = generate_boundary(
        plume_h, nozzle_hw, outer_width, seed, outer_amp, noise_scale)
    mid_l, mid_r = generate_boundary(
        plume_h, nozzle_hw, mid_width, seed + 1000, mid_amp, noise_scale * 1.3)
    core_l, core_r = generate_boundary(
        plume_h, nozzle_hw, core_width, seed + 2000, core_amp, noise_scale * 1.8)

    # Clamp to ensure proper nesting: core <= mid <= outer
    for row in range(plume_h):
        mid_l[row] = min(mid_l[row], outer_l[row])
        mid_r[row] = min(mid_r[row], outer_r[row])
        core_l[row] = min(core_l[row], mid_l[row])
        core_r[row] = min(core_r[row], mid_r[row])

    # Draw each layer on its own image
    outer_img = draw_layer(cw, ch, outer_l, outer_r, palette["outer"],
                           cx, y0, plume_h, tip_power=1.2)
    mid_img = draw_layer(cw, ch, mid_l, mid_r, palette["mid"],
                         cx, y0, plume_h, tip_power=1.3)
    core_img = draw_layer(cw, ch, core_l, core_r, palette["core"],
                          cx, y0, plume_h, tip_power=1.5)

    # Composite layers: outer (bottom) -> mid -> core (top)
    result = Image.new("RGBA", (cw, ch), (0, 0, 0, 0))
    result = Image.alpha_composite(result, outer_img)
    result = Image.alpha_composite(result, mid_img)
    result = Image.alpha_composite(result, core_img)

    # Gaussian blur for smooth, billowy look
    result = result.filter(ImageFilter.GaussianBlur(radius=5))

    return result


# ================================================================
# Main
# ================================================================

PROPELLANTS = ["kerolox", "methalox", "hydrolox"]

if __name__ == "__main__":
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.abspath(os.path.join(script_dir, "..", ".."))
    output_dir = os.path.join(project_root, "data", "sprites", "plumes")
    os.makedirs(output_dir, exist_ok=True)

    # Clean up old ASL/vacuum files
    for f in os.listdir(output_dir):
        if f.endswith(".png") and ("_asl_" in f or "_vacuum_" in f):
            os.remove(os.path.join(output_dir, f))
            print(f"  Removed old: {f}")

    count = 0
    for propellant in PROPELLANTS:
        for frame in range(NUM_FRAMES):
            img = generate_plume_frame(propellant, frame)
            name = f"{propellant}_frame{frame}"
            out = os.path.join(output_dir, f"{name}.png")
            img.save(out)
            count += 1
            print(f"  {name:30s}  {img.size[0]:4d}x{img.size[1]:4d}")

    print(f"\nGenerated {count} plume sprites in {output_dir}")
