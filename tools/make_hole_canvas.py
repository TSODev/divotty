#!/usr/bin/env python3
"""Generate a printable A4-landscape PDF canvas (100x60 grid + header) for
hand-drafting a Divotty hole before transcribing it into a .course file.

Requires pycairo (`pip install pycairo` or your distro's python3-cairo
package). Run from anywhere:

    python3 tools/make_hole_canvas.py

Regenerates tools/hole_design_canvas.pdf next to this script.
"""

import os

import cairo
import math

MM = 72.0 / 25.4  # points per mm

# A4 landscape
PAGE_W = 297 * MM
PAGE_H = 210 * MM

MARGIN = 10 * MM
GRID_COLS = 100
GRID_ROWS = 60

HEADER_H = 26 * MM

grid_x0 = MARGIN
grid_y0 = MARGIN + HEADER_H
grid_w = PAGE_W - 2 * MARGIN
grid_h = PAGE_H - 2 * MARGIN - HEADER_H
cell_w = grid_w / GRID_COLS
cell_h = grid_h / GRID_ROWS

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "hole_design_canvas.pdf")

surface = cairo.PDFSurface(OUT, PAGE_W, PAGE_H)
ctx = cairo.Context(surface)

def set_gray(v):
    ctx.set_source_rgb(v, v, v)

# --- Background ---
ctx.set_source_rgb(1, 1, 1)
ctx.paint()

# --- Header ---
ctx.set_source_rgb(0, 0, 0)
ctx.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
ctx.set_font_size(15)
ctx.move_to(MARGIN, MARGIN + 5 * MM)
ctx.show_text("DIVOTTY — Hole Design Canvas")

ctx.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_NORMAL)
ctx.set_font_size(8)
info = "Grid: 100 (x, left-to-right) × 60 (y, top-to-bottom) — origin (0,0) at top-left, matching the .course file format"
te = ctx.text_extents(info)
ctx.move_to(PAGE_W - MARGIN - te.width, MARGIN + 4.5 * MM)
ctx.show_text(info)

info2 = "Dashed arcs: approximate ideal tee-to-green distance per par, for a tee near the origin (see tools/make_hole_canvas.py)"
te2 = ctx.text_extents(info2)
ctx.move_to(PAGE_W - MARGIN - te2.width, MARGIN + 8 * MM)
ctx.show_text(info2)

# Fields row (Name / Par)
field_y = MARGIN + 11 * MM
ctx.set_font_size(10)
ctx.move_to(MARGIN, field_y)
ctx.show_text("Name: ")
name_x = MARGIN + ctx.text_extents("Name: ").width
ctx.set_line_width(0.6)
ctx.move_to(name_x, field_y + 1 * MM)
ctx.line_to(name_x + 90 * MM, field_y + 1 * MM)
ctx.stroke()

par_label_x = name_x + 100 * MM
ctx.move_to(par_label_x, field_y)
ctx.show_text("Par: ")
par_x = par_label_x + ctx.text_extents("Par: ").width
ctx.move_to(par_x, field_y + 1 * MM)
ctx.line_to(par_x + 20 * MM, field_y + 1 * MM)
ctx.stroke()

diff_label_x = par_x + 30 * MM
ctx.move_to(diff_label_x, field_y)
ctx.show_text("Difficulty (1-4, set at course.yaml level): ")
diff_x = diff_label_x + ctx.text_extents("Difficulty (1-4, set at course.yaml level): ").width
ctx.move_to(diff_x, field_y + 1 * MM)
ctx.line_to(diff_x + 15 * MM, field_y + 1 * MM)
ctx.stroke()

# Description row
desc_y = MARGIN + 17 * MM
ctx.move_to(MARGIN, desc_y)
ctx.show_text("Description: ")
desc_x = MARGIN + ctx.text_extents("Description: ").width
ctx.move_to(desc_x, desc_y + 1 * MM)
ctx.line_to(PAGE_W - MARGIN, desc_y + 1 * MM)
ctx.stroke()

# Legend row
legend_y = MARGIN + 23 * MM
ctx.set_font_size(8)
legend_items = [
    ("D", "tee"), (".", "fairway"), ("=", "rough"), ("B", "bunker"),
    ("~", "water"), ("T", "tree"), ("G", "green"), ("H", "hole"),
    ("X", "penalty area"), ("(blank)", "out of bounds"),
]
x = MARGIN
ctx.move_to(x, legend_y)
ctx.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
ctx.show_text("Legend:")
x += ctx.text_extents("Legend:").width + 3 * MM
for char, meaning in legend_items:
    label = f"{char}={meaning}"
    ctx.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_NORMAL)
    ctx.move_to(x, legend_y)
    ctx.show_text(label)
    x += ctx.text_extents(label).width + 5 * MM

# Separator line under header
ctx.set_line_width(0.8)
ctx.set_source_rgb(0, 0, 0)
ctx.move_to(MARGIN, grid_y0 - 2 * MM)
ctx.line_to(PAGE_W - MARGIN, grid_y0 - 2 * MM)
ctx.stroke()

# --- Grid ---
# Minor lines every 1 cell
ctx.set_line_width(0.15)
set_gray(0.75)
for c in range(GRID_COLS + 1):
    x = grid_x0 + c * cell_w
    ctx.move_to(x, grid_y0)
    ctx.line_to(x, grid_y0 + grid_h)
for r in range(GRID_ROWS + 1):
    y = grid_y0 + r * cell_h
    ctx.move_to(grid_x0, y)
    ctx.line_to(grid_x0 + grid_w, y)
ctx.stroke()

# Medium lines every 5 cells
ctx.set_line_width(0.4)
set_gray(0.45)
for c in range(0, GRID_COLS + 1, 5):
    x = grid_x0 + c * cell_w
    ctx.move_to(x, grid_y0)
    ctx.line_to(x, grid_y0 + grid_h)
for r in range(0, GRID_ROWS + 1, 5):
    y = grid_y0 + r * cell_h
    ctx.move_to(grid_x0, y)
    ctx.line_to(grid_x0 + grid_w, y)
ctx.stroke()

# Bold lines every 10 cells
ctx.set_line_width(0.9)
ctx.set_source_rgb(0, 0, 0)
for c in range(0, GRID_COLS + 1, 10):
    x = grid_x0 + c * cell_w
    ctx.move_to(x, grid_y0)
    ctx.line_to(x, grid_y0 + grid_h)
for r in range(0, GRID_ROWS + 1, 10):
    y = grid_y0 + r * cell_h
    ctx.move_to(grid_x0, y)
    ctx.line_to(grid_x0 + grid_w, y)
ctx.stroke()

# Outer border, thicker
ctx.set_line_width(1.2)
ctx.rectangle(grid_x0, grid_y0, grid_w, grid_h)
ctx.stroke()

# --- Ideal-distance markers per par ---
# Quarter-circle arcs from the origin (a tee near (0,0)): roughly how far a
# green "should" be for a hole to play like the given par, given the club
# distances calibrated elsewhere in the project (Driver averages ~28 cells;
# see src/core/shot.rs). Radii chosen to stay within the grid (<=100 wide,
# <=60 tall) — each arc is clipped to the sweep angle where it's still
# inside the grid, rather than the full 0-90 degrees, since a radius bigger
# than the grid's height/width would otherwise run off the page.
PAR_RADII = {4: 45, 5: 60, 6: 75, 7: 90, 8: 100}

def arc_angle_range(radius):
    t_min = math.acos(min(1.0, GRID_COLS / radius)) if radius > GRID_COLS else 0.0
    t_max = math.asin(min(1.0, GRID_ROWS / radius)) if radius > GRID_ROWS else math.pi / 2
    return t_min, t_max

ctx.set_source_rgb(0.2, 0.2, 0.2)
ctx.set_dash([3, 2])
for par, radius in PAR_RADII.items():
    t_min, t_max = arc_angle_range(radius)
    ctx.set_line_width(1.1)
    steps = 80
    for i in range(steps + 1):
        t = t_min + (t_max - t_min) * i / steps
        px = grid_x0 + radius * math.cos(t) * cell_w
        py = grid_y0 + radius * math.sin(t) * cell_h
        if i == 0:
            ctx.move_to(px, py)
        else:
            ctx.line_to(px, py)
    ctx.stroke()
ctx.set_dash([])

ctx.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
ctx.set_font_size(7.5)
for par, radius in PAR_RADII.items():
    t_min, t_max = arc_angle_range(radius)
    t_mid = (t_min + t_max) / 2
    label = f"Par {par} (~{radius})"
    te = ctx.text_extents(label)
    lx = grid_x0 + radius * math.cos(t_mid) * cell_w
    ly = grid_y0 + radius * math.sin(t_mid) * cell_h
    pad = 0.8 * MM
    ctx.set_source_rgb(1, 1, 1)
    ctx.rectangle(lx - te.width / 2 - pad, ly - te.height - pad, te.width + 2 * pad, te.height + 2 * pad)
    ctx.fill()
    ctx.set_source_rgb(0.2, 0.2, 0.2)
    ctx.move_to(lx - te.width / 2, ly)
    ctx.show_text(label)

# --- Axis labels (every 10 columns/rows) ---
ctx.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_NORMAL)
ctx.set_font_size(6.5)
ctx.set_source_rgb(0, 0, 0)

for c in range(0, GRID_COLS + 1, 10):
    x = grid_x0 + c * cell_w
    label = str(c)
    te = ctx.text_extents(label)
    # above grid
    ctx.move_to(x - te.width / 2, grid_y0 - 1 * MM)
    ctx.show_text(label)
    # below grid
    ctx.move_to(x - te.width / 2, grid_y0 + grid_h + 3.2 * MM)
    ctx.show_text(label)

for r in range(0, GRID_ROWS + 1, 10):
    y = grid_y0 + r * cell_h
    label = str(r)
    te = ctx.text_extents(label)
    # left of grid
    ctx.move_to(grid_x0 - te.width - 1.5 * MM, y + te.height / 2)
    ctx.show_text(label)
    # right of grid
    ctx.move_to(grid_x0 + grid_w + 1.5 * MM, y + te.height / 2)
    ctx.show_text(label)

# Axis direction hints
ctx.set_font_size(7)
ctx.select_font_face("Sans", cairo.FONT_SLANT_ITALIC, cairo.FONT_WEIGHT_NORMAL)
ctx.move_to(grid_x0 + grid_w / 2 - 10 * MM, grid_y0 + grid_h + 7 * MM)
ctx.show_text("x (column, left -> right)")
ctx.save()
ctx.translate(grid_x0 - 7 * MM, grid_y0 + grid_h / 2 + 10 * MM)
ctx.rotate(-math.pi / 2)
ctx.move_to(0, 0)
ctx.show_text("y (row, top -> bottom)")
ctx.restore()

surface.finish()
print("Wrote", OUT)
