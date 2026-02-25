#!/usr/bin/env python3
"""
Generate sprites for 8 interstellar engines.

Three technology tiers:
  Fission:    Orion Pulse (nuclear pulse propulsion)
  Fusion:     Daedalus S1/S2, Z-Pinch Probe/Advanced
  Antimatter: AM-Cat Fusion, Antimatter Torch, Gamma Conversion

Each engine has a distinctive visual design reflecting its propulsion physics.
Interstellar engines are dramatically larger than chemical engines (11-27 grid
wide vs 1-8 for chemical), with accent colors for magnetic coils (copper),
reactor housings (blue-steel), and antimatter containment (purple).
"""

from PIL import Image, ImageDraw
import math
import os
import random

# ================================================================
# Constants
# ================================================================
PX = 90   # pixels per grid square (matches chemical engines)
PAD = 6

# ================================================================
# Palette — steel base + interstellar accents
# ================================================================

# Base steel (same as chemical engines)
STEEL_DARK = (48, 50, 55)
STEEL_MID = (80, 84, 92)
STEEL_LIGHT = (120, 125, 135)
STEEL_HIGHLIGHT = (155, 160, 170)
STEEL_VERY_DARK = (32, 34, 38)
INTERIOR = (20, 18, 16)
ABLATIVE_TIP = (70, 62, 50)
PIPE_COLOR = (65, 68, 75)
PIPE_HIGHLIGHT = (100, 105, 115)

# Magnetic coil windings (copper/bronze)
COIL_DARK = (90, 60, 35)
COIL_MID = (140, 95, 55)
COIL_LIGHT = (180, 130, 75)
COIL_HIGHLIGHT = (210, 165, 100)

# Reactor housing (blue-tinted steel)
REACTOR_DARK = (40, 50, 65)
REACTOR_MID = (55, 70, 92)
REACTOR_LIGHT = (80, 100, 130)
REACTOR_HIGHLIGHT = (105, 130, 160)

# Antimatter containment (deep purple)
AM_DARK = (55, 38, 75)
AM_MID = (80, 58, 110)
AM_LIGHT = (110, 85, 145)
AM_HIGHLIGHT = (140, 115, 175)

# Energy channel (cyan/teal)
ENERGY_DARK = (35, 65, 85)
ENERGY_MID = (50, 90, 120)
ENERGY_LIGHT = (70, 120, 155)

# Orion pusher plate (heat-darkened steel)
PLATE_DARK = (42, 40, 36)
PLATE_MID = (58, 55, 48)
PLATE_LIGHT = (78, 72, 62)
PLATE_HIGHLIGHT = (95, 88, 75)
PLATE_ABLATION = (48, 42, 34)

# Gamma reflector (warm silver)
GAMMA_DARK = (60, 58, 65)
GAMMA_MID = (90, 86, 95)
GAMMA_LIGHT = (125, 120, 132)
GAMMA_HIGHLIGHT = (160, 155, 168)


# ================================================================
# Drawing primitives
# ================================================================

def trap(d, cx, y, tw, bw, h, fill, outline=None):
    """Trapezoid: tw=top width, bw=bottom width, h=height."""
    d.polygon([(cx - tw / 2, y), (cx + tw / 2, y),
               (cx + bw / 2, y + h), (cx - bw / 2, y + h)],
              fill=fill, outline=outline)

def rect(d, cx, y, w, h, fill, outline=None):
    """Centered rectangle."""
    d.rectangle([cx - w / 2, y, cx + w / 2, y + h], fill=fill, outline=outline)

def circ(d, cx, cy, r, **kw):
    d.ellipse([cx - r, cy - r, cx + r, cy + r], **kw)

def bell_w(frac, throat, exit_w, exp=1.8):
    """Bell/nozzle expansion curve."""
    return throat + (exit_w - throat) * (1 - (1 - frac) ** exp)

def bezier2(p0, p1, p2, n=20):
    pts = []
    for i in range(n + 1):
        t = i / n
        x = (1-t)**2*p0[0] + 2*(1-t)*t*p1[0] + t**2*p2[0]
        y = (1-t)**2*p0[1] + 2*(1-t)*t*p1[1] + t**2*p2[1]
        pts.append((int(x), int(y)))
    return pts

def lerp_color(c1, c2, t):
    """Linearly interpolate between two RGB colors."""
    t = max(0.0, min(1.0, t))
    return tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3))

def clamp_color(c):
    return tuple(max(0, min(255, v)) for v in c)

def draw_bolts(d, cx, y, w, h, n, r=3):
    """Draw a row of bolts across a horizontal span."""
    hw = int(w / 2)
    for i in range(n):
        if n > 1:
            bx = cx - hw + r * 2 + i * int((w - r * 4) / (n - 1))
        else:
            bx = cx
        circ(d, bx, y + h // 2, r, fill=STEEL_HIGHLIGHT)
        circ(d, bx, y + h // 2, max(1, r - 1), fill=STEEL_DARK)


# ================================================================
# Shared interstellar components
# ================================================================

def draw_mount_ring(d, cx, y, w, h, scale=1.0):
    """Standard mounting interface ring with bolts."""
    bolt_r = max(2, int(3 * scale))
    n_bolts = max(3, int(w / (bolt_r * 8)))
    trap(d, cx, y, w, w, h, STEEL_MID, outline=STEEL_DARK)
    hw = int(w / 2)
    d.line([(cx - hw + 2, y + 1), (cx + hw - 2, y + 1)],
           fill=STEEL_HIGHLIGHT, width=max(1, int(scale)))
    draw_bolts(d, cx, y, w, h, n_bolts, r=bolt_r)
    return y + h


def draw_magnetic_coil(d, cx, y, outer_w, thickness, scale=1.0, wall_th=None):
    """Draw a toroidal magnetic field coil wrapping around a hollow nozzle.

    Renders as two raised copper bumps on the left and right nozzle walls,
    NOT spanning across the interior void. Each bump has 3D rounded
    cross-section shading with specular highlight and cast shadow.
    """
    th = max(4, int(thickness))
    protrude = max(2, int(3 * scale))

    # Wall thickness determines how wide each coil bump is
    if wall_th is None:
        wall_th = max(12, int(14 * scale))

    o_hw = int(outer_w / 2) + protrude          # outer edge (beyond wall)
    i_hw = max(0, int(outer_w / 2) - wall_th)   # inner edge (at void)

    for side in [-1, 1]:
        if side == -1:
            left = cx - o_hw
            right = cx - i_hw
        else:
            left = cx + i_hw
            right = cx + o_hw

        section_w = right - left
        if section_w < 3:
            continue

        # Rounded cross-section shading (sinusoidal brightness)
        for row in range(th):
            t = row / max(1, th - 1)
            roundness = math.sin(t * math.pi)
            top_bias = max(0, 1.0 - t * 1.6)
            shade = 0.15 + 0.55 * roundness + 0.30 * top_bias
            shade = max(0.0, min(1.0, shade))

            r = int(COIL_DARK[0] + (COIL_LIGHT[0] - COIL_DARK[0]) * shade)
            g = int(COIL_DARK[1] + (COIL_LIGHT[1] - COIL_DARK[1]) * shade)
            b = int(COIL_DARK[2] + (COIL_LIGHT[2] - COIL_DARK[2]) * shade)

            ry = y - th // 2 + row
            d.rectangle([left, ry, right, ry + 1],
                        fill=clamp_color((r, g, b)))

        # Specular highlight near top
        spec_y = y - th // 2 + max(1, th // 5)
        d.line([(left + 1, spec_y), (right - 1, spec_y)],
               fill=COIL_HIGHLIGHT, width=1)

        # Cast shadow below
        shadow_y = y + (th + 1) // 2
        d.rectangle([left + 1, shadow_y, right - 1,
                     shadow_y + max(1, int(1 * scale))],
                    fill=(30, 25, 18))

        # Winding marks (only if section is wide enough)
        if section_w > 12:
            wind_color = (min(255, COIL_LIGHT[0] + 15),
                          min(255, COIL_LIGHT[1] + 15),
                          min(255, COIL_LIGHT[2] + 10))
            n_winds = max(3, section_w // max(3, int(4 * scale)))
            spacing = max(2, (section_w - 4) // max(1, n_winds))
            for i in range(n_winds):
                wx = left + 2 + i * spacing
                if wx < right - 2:
                    d.line([(wx, y - th // 2 + 2),
                            (wx + max(1, th // 6), y + th // 2 - 2)],
                           fill=wind_color, width=1)


def draw_reactor_housing(d, cx, y, top_w, bot_w, h, tint="reactor", scale=1.0):
    """Draw a reactor section housing with panel lines and structural bands."""
    colors = {
        "reactor": (REACTOR_DARK, REACTOR_MID, REACTOR_LIGHT, REACTOR_HIGHLIGHT),
        "antimatter": (AM_DARK, AM_MID, AM_LIGHT, AM_HIGHLIGHT),
        "steel": (STEEL_DARK, STEEL_MID, STEEL_LIGHT, STEEL_HIGHLIGHT),
        "gamma": (GAMMA_DARK, GAMMA_MID, GAMMA_LIGHT, GAMMA_HIGHLIGHT),
    }
    dark, mid, light, highlight = colors.get(tint, colors["reactor"])

    # Main housing body
    trap(d, cx, y, top_w, bot_w, h, mid, outline=dark)

    # Left edge highlight
    d.line([(cx - int(top_w / 2) + 2, y + 2),
            (cx - int(bot_w / 2) + 2, y + h - 2)],
           fill=light, width=max(1, int(scale)))

    # Horizontal structural bands
    band_h = max(2, int(3 * scale))
    for frac in [0.25, 0.5, 0.75]:
        by = y + int(h * frac)
        bw_at = top_w + (bot_w - top_w) * frac
        bhw = int(bw_at / 2)
        d.rectangle([cx - bhw, by, cx + bhw, by + band_h], fill=light)
        d.rectangle([cx - bhw, by + band_h, cx + bhw, by + band_h + 1], fill=dark)

    # Vertical panel lines
    n_panels = max(2, int(bot_w / (40 * scale)))
    for i in range(n_panels):
        frac = (i + 1) / (n_panels + 1)
        px = cx - int(bot_w / 2) + int(bot_w * frac)
        d.line([(px, y + 4), (px, y + h - 4)], fill=dark, width=1)

    return y + h


def draw_containment_ring(d, cx, y, w, h, tint="antimatter", scale=1.0):
    """Draw a containment/separator ring with accent color."""
    colors = {
        "antimatter": (AM_DARK, AM_MID, AM_LIGHT, AM_HIGHLIGHT),
        "energy": (ENERGY_DARK, ENERGY_MID, ENERGY_LIGHT),
        "copper": (COIL_DARK, COIL_MID, COIL_LIGHT),
        "steel": (STEEL_DARK, STEEL_MID, STEEL_LIGHT),
    }
    c = colors.get(tint, colors["steel"])
    hw = int(w / 2)
    extra = max(4, int(6 * scale))
    # Ring wider than surrounding structure
    d.rectangle([cx - hw - extra, y, cx + hw + extra, y + h], fill=c[1])
    d.rectangle([cx - hw - extra, y, cx + hw + extra, y + max(1, h // 3)], fill=c[2])
    d.rectangle([cx - hw - extra, y + h - max(1, h // 3), cx + hw + extra, y + h],
                fill=c[0])
    return y + h


def draw_structural_truss(d, cx, y, top_w, bot_w, h, n_bays=4, scale=1.0):
    """Draw an open cross-braced structural truss section."""
    sw = max(2, int(3 * scale))
    # Main longerons (side rails)
    for s in [-1, 1]:
        tx = cx + s * int(top_w / 2)
        bx = cx + s * int(bot_w / 2)
        d.line([(tx, y), (bx, y + h)], fill=STEEL_MID, width=sw + 1)
        d.line([(tx + s, y), (bx + s, y + h)], fill=STEEL_HIGHLIGHT, width=1)

    # Cross braces
    for i in range(n_bays):
        f1 = i / n_bays
        f2 = (i + 1) / n_bays
        y1 = y + int(h * f1)
        y2 = y + int(h * f2)
        w1 = top_w + (bot_w - top_w) * f1
        w2 = top_w + (bot_w - top_w) * f2
        # X-brace
        d.line([(cx - int(w1 / 2), y1), (cx + int(w2 / 2), y2)],
               fill=STEEL_DARK, width=sw)
        d.line([(cx + int(w1 / 2), y1), (cx - int(w2 / 2), y2)],
               fill=STEEL_DARK, width=sw)
        # Horizontal at each bay boundary
        hw = int(w2 / 2)
        d.line([(cx - hw, y2), (cx + hw, y2)], fill=STEEL_MID, width=sw)

    return y + h


def draw_magnetic_nozzle(d, cx, y, throat_w, exit_w, height,
                         coil_fracs=None, bell_exp=2.0,
                         wall_base=8, tint_shift=(0, 0, 0), scale=1.0):
    """Draw a magnetic nozzle with toroidal coils and thin structural walls.

    Unlike chemical nozzles (solid-walled), magnetic nozzles have thin
    structural supports holding field coils that magnetically contain the
    plasma exhaust. The wall is much thinner and the interior is visible.
    """
    if coil_fracs is None:
        coil_fracs = [0.1, 0.25, 0.4, 0.55, 0.7, 0.85]

    n_strips = max(40, int(height / 5))
    sh = height / n_strips

    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        ow1 = bell_w(t, throat_w, exit_w, exp=bell_exp)
        ow2 = bell_w(t1, throat_w, exit_w, exp=bell_exp)
        sy = y + t * height

        # Wall thickness tapers from throat to exit
        wt = wall_base * (1.0 - 0.5 * t)
        iw1 = max(0, ow1 - wt * 2)
        iw2 = max(0, ow2 - wt * 2)

        # Color: gradual darkening from throat to exit (heat tint)
        heat = t ** 1.5
        r = int(STEEL_MID[0] - 5 + heat * 12 + tint_shift[0])
        g = int(STEEL_MID[1] - 5 + heat * 4 + tint_shift[1])
        b = int(STEEL_MID[2] - 5 - heat * 10 + tint_shift[2])

        # Subtle sine banding
        band = math.sin(t * math.pi * 8) * 3
        r += int(band)
        g += int(band)
        b += int(band)

        oc = clamp_color((r, g, b))
        trap(d, cx, int(sy), ow1, ow2, int(sh) + 1, fill=oc)

        # Interior darkness
        if iw1 > 6 and iw2 > 6:
            ir = int(INTERIOR[0] + heat * 8)
            ig = int(INTERIOR[1] + heat * 3)
            ib = int(INTERIOR[2])
            trap(d, cx, int(sy), iw1, iw2, int(sh) + 1, fill=(ir, ig, ib))

    # Stiffening rings between coils
    n_rings = max(3, int(height / (60 * scale)))
    ring_fracs = [(i + 0.5) / n_rings for i in range(n_rings)]
    rh = max(2, int(2 * scale))
    for rf in ring_fracs:
        # Skip if too close to a coil position
        if any(abs(rf - cf) < 0.06 for cf in coil_fracs):
            continue
        ry = int(y + rf * height)
        rw = bell_w(rf, throat_w, exit_w, exp=bell_exp)
        rhw = int(rw / 2)
        d.rectangle([cx - rhw, ry - rh, cx + rhw, ry + rh], fill=STEEL_LIGHT)
        d.rectangle([cx - rhw + 1, ry + rh, cx + rhw - 1, ry + rh + 1], fill=STEEL_DARK)

    # Magnetic coils at specified positions
    coil_th = max(5, int(5 * scale))
    for cf in coil_fracs:
        cy_pos = int(y + cf * height)
        cw = bell_w(cf, throat_w, exit_w, exp=bell_exp)
        draw_magnetic_coil(d, cx, cy_pos, cw, coil_th, scale=scale)

    # Exit lip
    lip_h = max(4, int(5 * scale))
    ey = int(y + height - lip_h)
    ehw = int(exit_w / 2)
    d.rectangle([cx - ehw, ey, cx + ehw, ey + lip_h],
                fill=ABLATIVE_TIP, outline=STEEL_VERY_DARK)

    return y + height


# ================================================================
# Engine generators
# ================================================================

def generate_orion():
    """Orion Pulse Drive — 27w × 30h grid, 7000t

    Nuclear pulse propulsion. The most raw, industrial interstellar engine.
    Distinctive anvil silhouette: massive pusher plate at bottom, shock
    absorber column above, pulse unit magazine, narrower mount at top.
    """
    W, H, TOP = 27, 30, 14
    img_w = int(W * PX) + PAD * 2
    img_h = int(H * PX) + PAD * 2
    img = Image.new("RGBA", (img_w, img_h), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cx = img_w // 2
    total_h = H * PX
    exit_w = W * PX
    top_w = TOP * PX
    scale = exit_w / 400.0
    y = PAD

    # --- MOUNT RING ---
    mount_h = max(10, int(total_h * 0.02))
    ring_w = top_w + 30
    y = draw_mount_ring(d, cx, y, ring_w, mount_h, scale=scale)

    # --- PAYLOAD ADAPTER (taper from mount to magazine) ---
    adapter_h = int(total_h * 0.04)
    magazine_w = exit_w * 0.68
    trap(d, cx, y, ring_w, magazine_w, adapter_h, STEEL_MID, outline=STEEL_DARK)
    # Panel lines on adapter
    for frac in [0.3, 0.6]:
        ly = y + int(adapter_h * frac)
        lw = ring_w + (magazine_w - ring_w) * frac
        lhw = int(lw / 2)
        d.rectangle([cx - lhw, ly, cx + lhw, ly + 2], fill=STEEL_LIGHT)
    y += adapter_h

    # --- PULSE UNIT MAGAZINE ---
    mag_h = int(total_h * 0.13)
    mag_w = magazine_w
    # Main housing
    rect(d, cx, y, mag_w, mag_h, STEEL_MID, outline=STEEL_DARK)
    # Left highlight
    d.line([(cx - int(mag_w / 2) + 3, y + 3),
            (cx - int(mag_w / 2) + 3, y + mag_h - 3)],
           fill=STEEL_LIGHT, width=2)
    # Horizontal structural bands
    for frac in [0.33, 0.66]:
        by = y + int(mag_h * frac)
        mhw = int(mag_w / 2)
        d.rectangle([cx - mhw, by, cx + mhw, by + max(3, int(3 * scale))],
                    fill=STEEL_LIGHT)
        d.rectangle([cx - mhw, by + 3, cx + mhw, by + 4], fill=STEEL_DARK)
    # Access hatches (evenly spaced rectangular outlines)
    n_hatches = max(4, int(mag_w / (60 * scale)))
    hatch_w = max(20, int(30 * scale))
    hatch_h = max(15, int(mag_h * 0.25))
    for row in [0.2, 0.55]:
        hy = y + int(mag_h * row)
        for i in range(n_hatches):
            hx = cx - int(mag_w / 2) + int((i + 0.5) * mag_w / n_hatches)
            d.rectangle([hx - hatch_w // 2, hy, hx + hatch_w // 2, hy + hatch_h],
                        outline=STEEL_DARK)
            d.rectangle([hx - hatch_w // 2 + 1, hy + 1,
                         hx + hatch_w // 2 - 1, hy + 2], fill=STEEL_HIGHLIGHT)
    y += mag_h

    # --- STRUCTURAL TRUSS (open frame connecting magazine to absorbers) ---
    truss_h = int(total_h * 0.10)
    absorber_col_w = exit_w * 0.30
    y = draw_structural_truss(d, cx, y, mag_w, absorber_col_w + 40,
                               truss_h, n_bays=5, scale=scale)

    # --- SHOCK ABSORBER ASSEMBLY ---
    absorber_h = int(total_h * 0.27)
    abs_w = absorber_col_w
    # Outer housing
    trap(d, cx, y, abs_w + 40, abs_w + 60, absorber_h, STEEL_DARK, outline=STEEL_VERY_DARK)
    trap(d, cx, y + 3, abs_w + 20, abs_w + 40, absorber_h - 3, STEEL_MID)

    # Piston assemblies (4 visible)
    n_pistons = 4
    piston_w = max(30, int(abs_w * 0.18))
    piston_spacing = (abs_w - piston_w) / max(1, n_pistons - 1)
    for i in range(n_pistons):
        px = cx - int(abs_w / 2) + int(piston_w / 2) + int(i * piston_spacing)
        # Piston cylinder
        cyl_h = int(absorber_h * 0.85)
        cyl_w = piston_w
        rect(d, px, y + int(absorber_h * 0.08), cyl_w, cyl_h,
             STEEL_VERY_DARK, outline=STEEL_DARK)
        # Piston rod (lighter, narrower, extending partway)
        rod_w = cyl_w * 0.4
        rod_h = cyl_h * 0.6
        rect(d, px, y + int(absorber_h * 0.08) + int(cyl_h * 0.35),
             rod_w, rod_h, STEEL_HIGHLIGHT)
        # Piston rings (horizontal bands on the rod)
        for rf in [0.4, 0.55, 0.7]:
            ry = y + int(absorber_h * 0.08) + int(cyl_h * rf)
            d.rectangle([px - int(rod_w / 2) - 2, ry,
                         px + int(rod_w / 2) + 2, ry + 2], fill=STEEL_DARK)

    # Spring coils between pistons
    spring_w = max(10, int(piston_spacing * 0.4))
    for i in range(n_pistons - 1):
        sx = cx - int(abs_w / 2) + int(piston_w) + int(i * piston_spacing)
        n_coils = max(8, int(absorber_h / (15 * scale)))
        for j in range(n_coils):
            sy = y + int(absorber_h * 0.1) + int(j * absorber_h * 0.78 / n_coils)
            # Zigzag spring pattern
            if j % 2 == 0:
                d.line([(sx, sy), (sx + spring_w, sy + int(absorber_h * 0.78 / n_coils / 2))],
                       fill=STEEL_LIGHT, width=max(1, int(2 * scale)))
            else:
                d.line([(sx + spring_w, sy),
                        (sx, sy + int(absorber_h * 0.78 / n_coils / 2))],
                       fill=STEEL_LIGHT, width=max(1, int(2 * scale)))

    # Structural bands on absorber housing
    for frac in [0.15, 0.45, 0.75]:
        by = y + int(absorber_h * frac)
        ahw = int((abs_w + 50) / 2)
        d.rectangle([cx - ahw, by, cx + ahw, by + max(3, int(4 * scale))],
                    fill=STEEL_LIGHT)
    y += absorber_h

    # --- PLATE ADAPTER (widens from absorber column to full plate width) ---
    plate_adapter_h = int(total_h * 0.06)
    trap(d, cx, y, abs_w + 60, exit_w, plate_adapter_h, PLATE_MID, outline=PLATE_DARK)
    # Structural ribbing
    n_ribs = max(6, int(exit_w / (50 * scale)))
    for i in range(n_ribs):
        rx = cx - int(exit_w / 2) + int((i + 0.5) * exit_w / n_ribs)
        top_x = cx - int((abs_w + 60) / 2) + int((i + 0.5) * (abs_w + 60) / n_ribs)
        d.line([(top_x, y + 2), (rx, y + plate_adapter_h - 2)],
               fill=PLATE_DARK, width=max(2, int(2 * scale)))
    y += plate_adapter_h

    # --- PUSHER PLATE (the massive flat bottom) ---
    plate_h = int(total_h * 0.38)
    plate_w = exit_w

    # Main plate body - layered for depth
    rect(d, cx, y, plate_w, plate_h, PLATE_MID, outline=PLATE_DARK)

    # Concentric reinforcement rings (viewed edge-on, appear as horizontal bands)
    n_rings = max(8, int(plate_h / (20 * scale)))
    for i in range(n_rings):
        ry = y + int((i + 0.5) * plate_h / n_rings)
        phw = int(plate_w / 2)
        ring_h = max(2, int(3 * scale))
        # Alternating lighter/darker bands for depth
        if i % 2 == 0:
            d.rectangle([cx - phw + 4, ry, cx + phw - 4, ry + ring_h], fill=PLATE_LIGHT)
        else:
            d.rectangle([cx - phw + 4, ry, cx + phw - 4, ry + ring_h], fill=PLATE_DARK)

    # Ablation texture (random darker spots on the bottom half)
    random.seed(42)  # reproducible
    n_spots = max(30, int(plate_w * plate_h / (200 * scale)))
    for _ in range(n_spots):
        sx = cx + random.randint(-int(plate_w / 2) + 8, int(plate_w / 2) - 8)
        # Concentrate spots toward the bottom (most ablation)
        sy_frac = random.random() ** 0.5  # biased toward 1.0 (bottom)
        sy = y + int(plate_h * 0.3) + int(plate_h * 0.65 * sy_frac)
        sr = max(3, int(random.randint(4, 12) * scale))
        circ(d, sx, sy, sr, fill=PLATE_ABLATION)

    # Gas channel ports around the plate edge
    n_ports = max(12, int(plate_w / (30 * scale)))
    port_r = max(4, int(6 * scale))
    port_y = y + int(plate_h * 0.85)
    for i in range(n_ports):
        px = cx - int(plate_w / 2) + int((i + 0.5) * plate_w / n_ports)
        circ(d, px, port_y, port_r + 1, fill=PLATE_DARK)
        circ(d, px, port_y, port_r - 1, fill=INTERIOR)

    # Bottom face of the plate (thick ablative lip)
    lip_h = max(6, int(10 * scale))
    phw = int(plate_w / 2)
    d.rectangle([cx - phw, y + plate_h - lip_h, cx + phw, y + plate_h],
                fill=PLATE_DARK, outline=STEEL_VERY_DARK)
    # Chamfered edges
    chamfer = max(10, int(15 * scale))
    d.polygon([(cx - phw, y + plate_h - lip_h),
               (cx - phw + chamfer, y + plate_h),
               (cx - phw, y + plate_h)], fill=PLATE_DARK)
    d.polygon([(cx + phw, y + plate_h - lip_h),
               (cx + phw - chamfer, y + plate_h),
               (cx + phw, y + plate_h)], fill=PLATE_DARK)

    # Left edge highlight on the whole plate
    d.line([(cx - phw + 3, y + 3), (cx - phw + 3, y + plate_h - 3)],
           fill=PLATE_HIGHLIGHT, width=max(2, int(3 * scale)))

    return img


def generate_daedalus(stage=1):
    """Daedalus ICF Fusion Engine — S1: 20w×26h, S2: 14w×19h

    Inertial confinement fusion with magnetic nozzle. Beautiful parabolic
    bell with prominent copper toroidal field coils. Bulging reactor
    chamber houses the fusion pellet injection and laser/beam systems.
    """
    if stage == 1:
        W, H, TOP = 20, 26, 12
    else:
        W, H, TOP = 14, 19, 9

    img_w = int(W * PX) + PAD * 2
    img_h = int(H * PX) + PAD * 2
    img = Image.new("RGBA", (img_w, img_h), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cx = img_w // 2
    total_h = H * PX
    exit_w = W * PX
    top_w = TOP * PX
    scale = exit_w / 300.0
    y = PAD

    # --- MOUNT RING ---
    mount_h = max(10, int(total_h * 0.025))
    ring_w = top_w + 20
    y = draw_mount_ring(d, cx, y, ring_w, mount_h, scale=scale)

    # --- REACTOR HOUSING (bulging section above nozzle) ---
    reactor_h = int(total_h * 0.22)
    reactor_top_w = ring_w + 10
    reactor_bot_w = exit_w * 0.55  # bulges wider than mount
    y = draw_reactor_housing(d, cx, y, reactor_top_w, reactor_bot_w,
                              reactor_h, tint="reactor", scale=scale)

    # Laser/beam injection ports on reactor sides
    port_r = max(6, int(10 * scale))
    for s in [-1, 1]:
        for frac in [0.3, 0.6]:
            py = y - int(reactor_h * (1 - frac))
            pw = reactor_top_w + (reactor_bot_w - reactor_top_w) * frac
            px = cx + s * int(pw / 2 - port_r)
            circ(d, px, py, port_r + 2, fill=REACTOR_DARK)
            circ(d, px, py, port_r, fill=ENERGY_DARK)
            circ(d, px, py, port_r - 3, fill=ENERGY_MID)

    # --- TRANSITION TO NOZZLE ---
    trans_h = int(total_h * 0.04)
    throat_w = exit_w * 0.18
    trap(d, cx, y, reactor_bot_w, throat_w + 40, trans_h, STEEL_DARK, outline=STEEL_VERY_DARK)
    # Containment ring at transition
    draw_containment_ring(d, cx, y + trans_h // 2, reactor_bot_w, max(6, int(8 * scale)),
                          tint="energy", scale=scale)
    y += trans_h

    # --- MAGNETIC NOZZLE ---
    nozzle_h = int(total_h * 0.72)
    n_coils = 7 if stage == 1 else 5
    coil_fracs = [(i + 0.5) / n_coils for i in range(n_coils)]
    y = draw_magnetic_nozzle(d, cx, y, throat_w, exit_w, nozzle_h,
                              coil_fracs=coil_fracs, bell_exp=2.0,
                              wall_base=max(6, int(10 * scale)),
                              scale=scale)

    return img


def generate_zpinch(variant="probe"):
    """Z-Pinch Fusion Engine — Probe: 11w×15h, Advanced: 18w×24h

    Z-pinch confinement compresses plasma with intense magnetic fields.
    Distinctive: nearly cylindrical body studded with prominent pinch coil
    rings along the full length. Short flared nozzle. Industrial, utilitarian.
    """
    if variant == "probe":
        W, H, TOP = 11, 15, 7
    else:
        W, H, TOP = 18, 24, 12

    img_w = int(W * PX) + PAD * 2
    img_h = int(H * PX) + PAD * 2
    img = Image.new("RGBA", (img_w, img_h), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cx = img_w // 2
    total_h = H * PX
    exit_w = W * PX
    top_w = TOP * PX
    scale = exit_w / 300.0
    y = PAD

    # --- MOUNT RING ---
    mount_h = max(10, int(total_h * 0.025))
    ring_w = top_w + 16
    y = draw_mount_ring(d, cx, y, ring_w, mount_h, scale=scale)

    # --- REACTOR/DRIVER SECTION ---
    reactor_h = int(total_h * 0.18)
    reactor_w = top_w + 30
    y = draw_reactor_housing(d, cx, y, ring_w, reactor_w, reactor_h,
                              tint="reactor", scale=scale)

    # --- Z-PINCH COIL BODY (the main feature — cylindrical with coil rings) ---
    body_h = int(total_h * 0.55)
    body_top_w = reactor_w
    body_bot_w = exit_w * 0.75  # slight taper outward

    # Draw the body as strips with coil rings overlaid
    n_coils = 5 if variant == "probe" else 8
    coil_spacing = body_h / (n_coils + 1)
    coil_th = max(5, int(5 * scale))

    # Body shell
    n_strips = max(30, int(body_h / 6))
    sh = body_h / n_strips
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = body_top_w + (body_bot_w - body_top_w) * t
        w2 = body_top_w + (body_bot_w - body_top_w) * t1
        sy = y + t * body_h

        # Subtle color variation
        band = math.sin(t * math.pi * n_coils * 2) * 5
        r = int(STEEL_MID[0] + band)
        g = int(STEEL_MID[1] + band)
        b = int(STEEL_MID[2] + band + 5)  # slight blue tint
        trap(d, cx, int(sy), w1, w2, int(sh) + 1, fill=clamp_color((r, g, b)))

    # Interior visible as dark center stripe
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = body_top_w + (body_bot_w - body_top_w) * t
        w2 = body_top_w + (body_bot_w - body_top_w) * t1
        iw1 = w1 * 0.55
        iw2 = w2 * 0.55
        sy = y + t * body_h
        trap(d, cx, int(sy), iw1, iw2, int(sh) + 1, fill=INTERIOR)

    # Left edge highlight
    d.line([(cx - int(body_top_w / 2) + 3, y),
            (cx - int(body_bot_w / 2) + 3, y + body_h)],
           fill=STEEL_LIGHT, width=max(2, int(2 * scale)))

    # Pinch coils — the star of the show
    for i in range(n_coils):
        cy_pos = y + int((i + 1) * coil_spacing)
        frac = (i + 1) / (n_coils + 1)
        cw = body_top_w + (body_bot_w - body_top_w) * frac
        draw_magnetic_coil(d, cx, cy_pos, cw, coil_th, scale=scale)

    # Structural ribs between coils
    rib_h = max(2, int(3 * scale))
    for i in range(n_coils + 1):
        if i == 0:
            ry = y + int(coil_spacing * 0.5)
        elif i == n_coils:
            ry = y + body_h - int(coil_spacing * 0.5)
        else:
            ry = y + int((i + 0.5) * coil_spacing)
        frac = ry / (y + body_h) if body_h > 0 else 0
        rw = body_top_w + (body_bot_w - body_top_w) * min(1, frac)
        rhw = int(rw / 2)
        d.rectangle([cx - rhw, ry - rib_h, cx + rhw, ry + rib_h], fill=STEEL_LIGHT)

    y += body_h

    # --- NOZZLE FLARE (short, opening to full width) ---
    flare_h = int(total_h * 0.20)
    throat_w = body_bot_w * 0.6

    # Converging section
    conv_h = int(flare_h * 0.25)
    trap(d, cx, y, body_bot_w, throat_w, conv_h, STEEL_DARK, outline=STEEL_VERY_DARK)
    y += conv_h

    # Flared nozzle
    nozzle_h = flare_h - conv_h
    n_strips = max(20, int(nozzle_h / 5))
    sh = nozzle_h / n_strips
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = bell_w(t, throat_w, exit_w, exp=1.5)
        w2 = bell_w(t1, throat_w, exit_w, exp=1.5)
        sy = y + t * nozzle_h
        heat = t ** 1.5
        r = int(STEEL_MID[0] - 3 + heat * 15)
        g = int(STEEL_MID[1] - 3 + heat * 5)
        b = int(STEEL_MID[2] - 3 - heat * 8)
        trap(d, cx, int(sy), w1, w2, int(sh) + 1, fill=clamp_color((r, g, b)))

    # Coil at nozzle throat
    draw_magnetic_coil(d, cx, y + int(nozzle_h * 0.05), throat_w + 10,
                       max(5, int(5 * scale)), scale=scale)
    # Coil at nozzle mid
    mid_w = bell_w(0.5, throat_w, exit_w, exp=1.5)
    draw_magnetic_coil(d, cx, y + int(nozzle_h * 0.5), mid_w,
                       max(5, int(5 * scale)), scale=scale)

    # Exit lip
    lip_h = max(4, int(5 * scale))
    ehw = int(exit_w / 2)
    ey = y + nozzle_h - lip_h
    d.rectangle([cx - ehw, int(ey), cx + ehw, int(ey + lip_h)],
                fill=ABLATIVE_TIP, outline=STEEL_VERY_DARK)

    return img


def generate_amcat():
    """Antimatter-Catalyzed Fusion Engine — 16w × 21h, 2500t

    Hybrid: a fusion reactor with trace antimatter injection to catalyze
    ignition. Two-section silhouette: bulging reactor section with a
    distinctive antimatter injection collar, feeding into a magnetic nozzle.
    """
    W, H, TOP = 16, 21, 10
    img_w = int(W * PX) + PAD * 2
    img_h = int(H * PX) + PAD * 2
    img = Image.new("RGBA", (img_w, img_h), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cx = img_w // 2
    total_h = H * PX
    exit_w = W * PX
    top_w = TOP * PX
    scale = exit_w / 300.0
    y = PAD

    # --- MOUNT RING ---
    mount_h = max(10, int(total_h * 0.025))
    ring_w = top_w + 20
    y = draw_mount_ring(d, cx, y, ring_w, mount_h, scale=scale)

    # --- ANTIMATTER INJECTION COLLAR (distinctive purple ring) ---
    collar_h = max(12, int(total_h * 0.035))
    collar_w = ring_w + 10
    y = draw_containment_ring(d, cx, y, collar_w, collar_h,
                               tint="antimatter", scale=scale)
    # Injection port nozzles on the collar
    port_r = max(4, int(6 * scale))
    for s in [-1, 1]:
        px = cx + s * int(collar_w / 2 + 4)
        circ(d, px, y - collar_h // 2, port_r + 2, fill=AM_DARK)
        circ(d, px, y - collar_h // 2, port_r, fill=AM_LIGHT)

    # --- FUSION REACTOR (wider bulging section) ---
    reactor_h = int(total_h * 0.22)
    reactor_top_w = collar_w + 10
    reactor_max_w = exit_w * 0.62  # widest point
    reactor_bot_w = reactor_max_w - 20

    # Upper half: expanding
    half_h = reactor_h // 2
    trap(d, cx, y, reactor_top_w, reactor_max_w, half_h, REACTOR_MID, outline=REACTOR_DARK)
    # Lower half: contracting slightly
    trap(d, cx, y + half_h, reactor_max_w, reactor_bot_w, half_h,
         REACTOR_MID, outline=REACTOR_DARK)

    # Left highlight
    d.line([(cx - int(reactor_top_w / 2) + 2, y + 2),
            (cx - int(reactor_max_w / 2) + 2, y + half_h)],
           fill=REACTOR_LIGHT, width=max(1, int(2 * scale)))

    # Structural bands
    band_h = max(3, int(3 * scale))
    for frac in [0.25, 0.5, 0.75]:
        by = y + int(reactor_h * frac)
        if frac < 0.5:
            bw = reactor_top_w + (reactor_max_w - reactor_top_w) * (frac / 0.5)
        else:
            bw = reactor_max_w + (reactor_bot_w - reactor_max_w) * ((frac - 0.5) / 0.5)
        bhw = int(bw / 2)
        d.rectangle([cx - bhw, by, cx + bhw, by + band_h], fill=REACTOR_LIGHT)
        d.rectangle([cx - bhw, by + band_h, cx + bhw, by + band_h + 1], fill=REACTOR_DARK)

    # Panel lines
    n_panels = max(3, int(reactor_max_w / (50 * scale)))
    for i in range(n_panels):
        frac = (i + 1) / (n_panels + 1)
        px = cx - int(reactor_max_w / 2) + int(reactor_max_w * frac)
        d.line([(px, y + 4), (px, y + reactor_h - 4)], fill=REACTOR_DARK, width=1)

    # Viewport/sensor windows on reactor
    vp_r = max(5, int(8 * scale))
    for s in [-1, 1]:
        vpx = cx + s * int(reactor_max_w * 0.3)
        vpy = y + int(reactor_h * 0.5)
        circ(d, vpx, vpy, vp_r + 2, fill=REACTOR_DARK)
        circ(d, vpx, vpy, vp_r, fill=ENERGY_MID)
        circ(d, vpx, vpy, vp_r - 2, fill=ENERGY_LIGHT)
    y += reactor_h

    # --- CONTAINMENT RING (at reactor/nozzle junction) ---
    cont_h = max(10, int(total_h * 0.03))
    y = draw_containment_ring(d, cx, y, reactor_bot_w, cont_h,
                               tint="copper", scale=scale)

    # --- TRANSITION TO NOZZLE ---
    trans_h = int(total_h * 0.04)
    throat_w = exit_w * 0.16
    trap(d, cx, y, reactor_bot_w, throat_w + 30, trans_h,
         STEEL_DARK, outline=STEEL_VERY_DARK)
    y += trans_h

    # --- MAGNETIC NOZZLE ---
    nozzle_h = int(total_h * 0.65)
    coil_fracs = [0.08, 0.22, 0.38, 0.55, 0.72, 0.88]
    y = draw_magnetic_nozzle(d, cx, y, throat_w, exit_w, nozzle_h,
                              coil_fracs=coil_fracs, bell_exp=1.8,
                              wall_base=max(6, int(9 * scale)),
                              tint_shift=(5, -2, -5),  # slightly warmer
                              scale=scale)

    return img


def generate_am_torch():
    """Antimatter Torch — 12w × 16h, 1200t

    Pure matter-antimatter annihilation engine. Sleek and narrow with
    magnetic confinement rings. Nearly cylindrical profile. Minimal
    bell flare — magnetic nozzle is so efficient it barely needs
    physical expansion. The most refined-looking engine.
    """
    W, H, TOP = 12, 16, 7
    img_w = int(W * PX) + PAD * 2
    img_h = int(H * PX) + PAD * 2
    img = Image.new("RGBA", (img_w, img_h), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cx = img_w // 2
    total_h = H * PX
    exit_w = W * PX
    top_w = TOP * PX
    scale = exit_w / 300.0
    y = PAD

    # --- MOUNT RING ---
    mount_h = max(10, int(total_h * 0.025))
    ring_w = top_w + 16
    y = draw_mount_ring(d, cx, y, ring_w, mount_h, scale=scale)

    # --- ANTIMATTER CONTAINMENT MODULE ---
    # Narrow cylindrical section with purple-tinted housing
    contain_h = int(total_h * 0.30)
    contain_top_w = ring_w + 8
    contain_bot_w = exit_w * 0.55

    # Body with AM tint
    y = draw_reactor_housing(d, cx, y, contain_top_w, contain_bot_w,
                              contain_h, tint="antimatter", scale=scale)

    # Superconducting containment rings (3 rings)
    for frac in [0.25, 0.50, 0.75]:
        ry = y - int(contain_h * (1 - frac))
        rw = contain_top_w + (contain_bot_w - contain_top_w) * frac
        ring_h = max(6, int(8 * scale))
        draw_containment_ring(d, cx, ry - ring_h // 2, rw, ring_h,
                               tint="antimatter", scale=scale)

    # --- MAGNETIC CONFINEMENT CHANNEL ---
    # Nearly uniform diameter tube — the annihilation zone
    channel_h = int(total_h * 0.35)
    channel_w = contain_bot_w
    channel_bot_w = channel_w + 10  # barely tapers

    # Draw as strips with glow
    n_strips = max(25, int(channel_h / 5))
    sh = channel_h / n_strips
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = channel_w + (channel_bot_w - channel_w) * t
        w2 = channel_w + (channel_bot_w - channel_w) * t1
        sy = y + t * channel_h

        # AM-tinted steel
        r = int(AM_MID[0] - 10 + t * 15)
        g = int(AM_MID[1] - 10 + t * 10)
        b = int(AM_MID[2] - 5 + t * 8)
        band = math.sin(t * math.pi * 6) * 4
        trap(d, cx, int(sy), w1, w2, int(sh) + 1,
             fill=clamp_color((int(r + band), int(g + band), int(b + band))))

    # Interior — bright energy channel
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = channel_w + (channel_bot_w - channel_w) * t
        w2 = channel_w + (channel_bot_w - channel_w) * t1
        iw1 = w1 * 0.4
        iw2 = w2 * 0.4
        sy = y + t * channel_h
        # Deep interior with faint purple glow
        ir = int(25 + t * 8)
        ig = int(20 + t * 5)
        ib = int(35 + t * 12)
        trap(d, cx, int(sy), iw1, iw2, int(sh) + 1, fill=(ir, ig, ib))

    # Superconducting rings along channel
    n_rings = 4
    for i in range(n_rings):
        frac = (i + 0.5) / n_rings
        ry = y + int(channel_h * frac)
        rw = channel_w + (channel_bot_w - channel_w) * frac
        ring_h = max(5, int(7 * scale))
        # These are thinner, more delicate rings than the containment ones
        hw = int(rw / 2)
        extra = max(3, int(5 * scale))
        d.rectangle([cx - hw - extra, ry - ring_h // 2,
                     cx + hw + extra, ry + ring_h // 2], fill=AM_LIGHT)
        d.rectangle([cx - hw - extra, ry - ring_h // 2,
                     cx + hw + extra, ry - ring_h // 2 + 2], fill=AM_HIGHLIGHT)
        d.rectangle([cx - hw - extra, ry + ring_h // 2 - 2,
                     cx + hw + extra, ry + ring_h // 2], fill=AM_DARK)

    # Left edge highlight
    d.line([(cx - int(channel_w / 2) + 2, y),
            (cx - int(channel_bot_w / 2) + 2, y + channel_h)],
           fill=AM_LIGHT, width=max(1, int(2 * scale)))
    y += channel_h

    # --- NOZZLE (minimal flare) ---
    nozzle_h = int(total_h * 0.30)
    throat_w = channel_bot_w * 0.7

    # Converging
    conv_h = int(nozzle_h * 0.15)
    trap(d, cx, y, channel_bot_w, throat_w, conv_h, STEEL_DARK, outline=STEEL_VERY_DARK)
    y += conv_h

    # Short magnetic nozzle with just 3 coils
    bell_h = nozzle_h - conv_h
    coil_fracs = [0.15, 0.5, 0.85]
    y = draw_magnetic_nozzle(d, cx, y, throat_w, exit_w, bell_h,
                              coil_fracs=coil_fracs, bell_exp=1.6,
                              wall_base=max(5, int(7 * scale)),
                              tint_shift=(8, -3, -8),  # warm AM tint
                              scale=scale)

    return img


def generate_gamma_conversion():
    """Gamma Conversion Engine — 13w × 20h, 1800t

    The most exotic engine. Converts antimatter annihilation gamma rays
    into directed thrust via pair production and gamma reflection.

    Three visually distinct sections:
    1. Annihilation reactor (top, slight bulge, purple)
    2. Pair production chamber (middle, narrow, blue-steel)
    3. Gamma reflector array (bottom, wider, angled panels)

    Segmented appearance with visible separation rings between sections.
    """
    W, H, TOP = 13, 20, 7
    img_w = int(W * PX) + PAD * 2
    img_h = int(H * PX) + PAD * 2
    img = Image.new("RGBA", (img_w, img_h), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    cx = img_w // 2
    total_h = H * PX
    exit_w = W * PX
    top_w = TOP * PX
    scale = exit_w / 300.0
    y = PAD

    # --- MOUNT RING ---
    mount_h = max(10, int(total_h * 0.02))
    ring_w = top_w + 16
    y = draw_mount_ring(d, cx, y, ring_w, mount_h, scale=scale)

    # === SECTION 1: ANNIHILATION REACTOR (top, purple, slight bulge) ===
    s1_h = int(total_h * 0.22)
    s1_top_w = ring_w + 10
    s1_max_w = exit_w * 0.65
    s1_bot_w = s1_max_w - 10

    # Upper expanding
    s1_upper_h = int(s1_h * 0.45)
    trap(d, cx, y, s1_top_w, s1_max_w, s1_upper_h, AM_MID, outline=AM_DARK)
    # Lower contracting
    s1_lower_h = s1_h - s1_upper_h
    trap(d, cx, y + s1_upper_h, s1_max_w, s1_bot_w, s1_lower_h, AM_MID, outline=AM_DARK)

    # Panel lines and details
    d.line([(cx - int(s1_top_w / 2) + 2, y + 2),
            (cx - int(s1_max_w / 2) + 2, y + s1_upper_h)],
           fill=AM_LIGHT, width=max(1, int(2 * scale)))

    # Structural bands
    for frac in [0.3, 0.6]:
        by = y + int(s1_h * frac)
        if frac < 0.45:
            bw = s1_top_w + (s1_max_w - s1_top_w) * (frac / 0.45)
        else:
            bw = s1_max_w + (s1_bot_w - s1_max_w) * ((frac - 0.45) / 0.55)
        bhw = int(bw / 2)
        d.rectangle([cx - bhw, by, cx + bhw, by + max(2, int(3 * scale))],
                    fill=AM_LIGHT)

    # Containment rings
    for frac in [0.2, 0.5, 0.8]:
        ry = y + int(s1_h * frac)
        if frac < 0.45:
            rw = s1_top_w + (s1_max_w - s1_top_w) * (frac / 0.45)
        else:
            rw = s1_max_w + (s1_bot_w - s1_max_w) * ((frac - 0.45) / 0.55)
        ring_h = max(5, int(6 * scale))
        hw = int(rw / 2)
        extra = max(3, int(4 * scale))
        d.rectangle([cx - hw - extra, ry - ring_h // 2,
                     cx + hw + extra, ry + ring_h // 2], fill=AM_HIGHLIGHT)
        d.rectangle([cx - hw - extra, ry + ring_h // 2 - 1,
                     cx + hw + extra, ry + ring_h // 2], fill=AM_DARK)
    y += s1_h

    # --- SEPARATION RING 1 ---
    sep_h = max(8, int(total_h * 0.02))
    y = draw_containment_ring(d, cx, y, s1_bot_w, sep_h, tint="steel", scale=scale)

    # === SECTION 2: PAIR PRODUCTION CHAMBER (middle, narrow, blue-steel) ===
    s2_h = int(total_h * 0.28)
    s2_w = s1_bot_w * 0.75  # narrower than reactor
    s2_bot_w = s2_w + 15

    # Cylindrical body with energy tint
    n_strips = max(20, int(s2_h / 5))
    sh = s2_h / n_strips
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = s2_w + (s2_bot_w - s2_w) * t
        w2 = s2_w + (s2_bot_w - s2_w) * t1
        sy = y + t * s2_h
        # Blue-steel color with subtle variation
        r = int(ENERGY_DARK[0] + 10 + math.sin(t * math.pi * 4) * 5)
        g = int(ENERGY_DARK[1] + 15 + math.sin(t * math.pi * 4) * 5)
        b = int(ENERGY_DARK[2] + 20 + math.sin(t * math.pi * 4) * 5)
        trap(d, cx, int(sy), w1, w2, int(sh) + 1, fill=clamp_color((r, g, b)))

    # Interior
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = s2_w + (s2_bot_w - s2_w) * t
        w2 = s2_w + (s2_bot_w - s2_w) * t1
        iw1 = w1 * 0.45
        iw2 = w2 * 0.45
        sy = y + t * s2_h
        trap(d, cx, int(sy), iw1, iw2, int(sh) + 1, fill=INTERIOR)

    # Left edge highlight
    d.line([(cx - int(s2_w / 2) + 2, y + 2),
            (cx - int(s2_bot_w / 2) + 2, y + s2_h - 2)],
           fill=ENERGY_LIGHT, width=max(1, int(2 * scale)))

    # Structural rings
    n_rings = 4
    ring_h = max(3, int(4 * scale))
    for i in range(n_rings):
        frac = (i + 0.5) / n_rings
        ry = y + int(s2_h * frac)
        rw = s2_w + (s2_bot_w - s2_w) * frac
        rhw = int(rw / 2)
        d.rectangle([cx - rhw - 3, ry - ring_h, cx + rhw + 3, ry + ring_h],
                    fill=ENERGY_LIGHT)
        d.rectangle([cx - rhw - 3, ry + ring_h, cx + rhw + 3, ry + ring_h + 1],
                    fill=ENERGY_DARK)
    y += s2_h

    # --- SEPARATION RING 2 ---
    y = draw_containment_ring(d, cx, y, s2_bot_w, sep_h, tint="steel", scale=scale)

    # === SECTION 3: GAMMA REFLECTOR ARRAY (bottom, wider, angled panels) ===
    s3_h = int(total_h * 0.42)
    s3_top_w = s2_bot_w + 10
    s3_bot_w = exit_w

    # Main reflector body — wider, with distinctive angled panels
    n_strips = max(30, int(s3_h / 5))
    sh = s3_h / n_strips
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = s3_top_w + (s3_bot_w - s3_top_w) * (t ** 0.8)  # aggressive flare
        w2 = s3_top_w + (s3_bot_w - s3_top_w) * (t1 ** 0.8)
        sy = y + t * s3_h
        # Warm silver color
        heat = t ** 1.2
        r = int(GAMMA_MID[0] + heat * 10)
        g = int(GAMMA_MID[1] + heat * 6)
        b = int(GAMMA_MID[2] + heat * 3 - heat * 8)
        band = math.sin(t * math.pi * 6) * 4
        trap(d, cx, int(sy), w1, w2, int(sh) + 1,
             fill=clamp_color((int(r + band), int(g + band), int(b + band))))

    # Interior
    for i in range(n_strips):
        t = i / n_strips
        t1 = (i + 1) / n_strips
        w1 = s3_top_w + (s3_bot_w - s3_top_w) * (t ** 0.8)
        w2 = s3_top_w + (s3_bot_w - s3_top_w) * (t1 ** 0.8)
        iw1 = w1 * 0.5
        iw2 = w2 * 0.5
        sy = y + t * s3_h
        heat = t ** 1.5
        ir = int(INTERIOR[0] + heat * 10)
        ig = int(INTERIOR[1] + heat * 3)
        ib = int(INTERIOR[2] + heat * 5)
        trap(d, cx, int(sy), iw1, iw2, int(sh) + 1, fill=(ir, ig, ib))

    # Angled reflector panel lines (diagonal hatching — the distinctive feature)
    n_panels = max(8, int(s3_bot_w / (25 * scale)))
    for i in range(n_panels):
        frac = (i + 0.5) / n_panels
        # Panel line goes from top-left to bottom-right at an angle
        top_x = cx - int(s3_top_w / 2) + int(s3_top_w * frac)
        bot_frac = min(1.0, frac + 0.08)
        bot_x = cx - int(s3_bot_w / 2) + int(s3_bot_w * bot_frac)
        d.line([(top_x, y + 4), (bot_x, y + s3_h - 4)],
               fill=GAMMA_DARK, width=max(1, int(2 * scale)))

    # Left highlight
    d.line([(cx - int(s3_top_w / 2) + 2, y + 2),
            (cx - int(s3_bot_w / 2) + 2, y + s3_h - 2)],
           fill=GAMMA_HIGHLIGHT, width=max(2, int(3 * scale)))

    # Structural rings on reflector
    for frac in [0.2, 0.45, 0.7]:
        ry = y + int(s3_h * frac)
        rw = s3_top_w + (s3_bot_w - s3_top_w) * (frac ** 0.8)
        rhw = int(rw / 2)
        ring_h = max(3, int(4 * scale))
        d.rectangle([cx - rhw, ry - ring_h, cx + rhw, ry + ring_h], fill=GAMMA_LIGHT)
        d.rectangle([cx - rhw, ry + ring_h, cx + rhw, ry + ring_h + 1], fill=GAMMA_DARK)

    # Magnetic coils near exit
    for frac in [0.6, 0.85]:
        cy_pos = y + int(s3_h * frac)
        cw = s3_top_w + (s3_bot_w - s3_top_w) * (frac ** 0.8)
        draw_magnetic_coil(d, cx, cy_pos, cw, max(5, int(5 * scale)), scale=scale)

    # Exit lip
    lip_h = max(5, int(6 * scale))
    ehw = int(exit_w / 2)
    d.rectangle([cx - ehw, y + s3_h - lip_h, cx + ehw, y + s3_h],
                fill=ABLATIVE_TIP, outline=STEEL_VERY_DARK)

    return img


# ================================================================
# Main
# ================================================================

ENGINES = {
    "engine_orion_pulse": generate_orion,
    "engine_daedalus_s1": lambda: generate_daedalus(stage=1),
    "engine_daedalus_s2": lambda: generate_daedalus(stage=2),
    "engine_zpinch_probe": lambda: generate_zpinch(variant="probe"),
    "engine_zpinch_advanced": lambda: generate_zpinch(variant="advanced"),
    "engine_amcat_fusion": generate_amcat,
    "engine_am_torch": generate_am_torch,
    "engine_gamma_conversion": generate_gamma_conversion,
}

if __name__ == "__main__":
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.abspath(os.path.join(script_dir, "..", ".."))
    output_dir = os.path.join(project_root, "data", "sprites", "engines")
    os.makedirs(output_dir, exist_ok=True)

    for name, gen_fn in ENGINES.items():
        img = gen_fn()
        out = os.path.join(output_dir, f"{name}.png")
        img.save(out)
        print(f"  {name:30s}  {img.size[0]:5d}x{img.size[1]:5d}")

    print(f"\nGenerated {len(ENGINES)} interstellar engine sprites in {output_dir}")
