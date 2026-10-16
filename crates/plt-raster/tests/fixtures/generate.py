"""Render the plt-raster reference corpus with matplotlib's RendererAgg.

Reads cases.txt (next to this script) and writes agg_reference.bin, which
tests/agg_reference.rs compares plt-raster's output against. Run with the
reference interpreter:

    .venv/bin/python -I crates/plt-raster/tests/fixtures/generate.py

Every op drives RendererAgg directly with dpi=72 (1 pt = 1 px) and
snapping disabled. Paths are given in device pixels (y down); passing
``Affine2D().scale(1, -1).translate(0, H)`` as the transform makes Agg's
own y flip cancel exactly, so Agg sees the very same coordinates.

Output format (little endian):
    b"PLTAGG1\\n", u32 case count, then per case:
    u16 name length, name (utf-8), u32 width, u32 height,
    u32 byte length, RLE data = repeated (u8 run length 1..255, r, g, b, a).
"""

import pathlib
import struct

import matplotlib as mpl
import numpy as np
from matplotlib.backends.backend_agg import RendererAgg
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, Bbox, TransformedPath

HERE = pathlib.Path(__file__).resolve().parent
CODES = {"M": (Path.MOVETO, 1), "L": (Path.LINETO, 1), "Q": (Path.CURVE3, 2), "C": (Path.CURVE4, 3)}


def parse_path(tokens):
    vertices, codes, start = [], [], (0.0, 0.0)
    i = 0
    while i < len(tokens):
        cmd = tokens[i]
        i += 1
        if cmd == "Z":
            vertices.append(start)
            codes.append(Path.CLOSEPOLY)
            continue
        code, count = CODES[cmd]
        for _ in range(count):
            pt = (float(tokens[i]), float(tokens[i + 1]))
            i += 2
            vertices.append(pt)
            codes.append(code)
        if cmd == "M":
            start = vertices[-1]
    return Path(np.array(vertices, float), codes)


def floats(text):
    return [float(v) for v in text.split(",")]


def image_pixels(w, h):
    """The deterministic RGBA test image (rows top to bottom)."""
    j, i = np.mgrid[0:h, 0:w]
    rgba = np.stack(
        [(i * 37 + j * 11) % 256, (i * 5 + j * 53 + 7) % 256, (i * i + j * 3) % 256, (i * 19 + j * 29 + 40) % 256],
        axis=-1,
    )
    return rgba.astype(np.uint8)


def coverage_pixels(w, h):
    """The deterministic gray8 coverage test bitmap (rows top to bottom)."""
    j, i = np.mgrid[0:h, 0:w]
    return ((i * 47 + j * 31 + i * j * 3) % 256).astype(np.uint8)


class Case:
    def __init__(self, name, width, height):
        self.name, self.width, self.height = name, width, height
        self.paths = {}
        self.renderer = RendererAgg(width, height, 72)
        self.flip = Affine2D().scale(1, -1).translate(0, height)

    def gc(self, args):
        gc = self.renderer.new_gc()
        gc.set_snap(False)
        gc.set_antialiased(args.get("aa", "1") == "1")
        if "clip" in args:
            x0, y0, x1, y1 = floats(args["clip"])
            gc.set_clip_rectangle(Bbox([[x0, self.height - y1], [x1, self.height - y0]]))
        if "clippath" in args:
            gc.set_clip_path(TransformedPath(self.paths[args["clippath"]], self.flip))
        return gc

    def stroke_gc(self, args, color_key):
        gc = self.gc(args)
        gc.set_foreground(tuple(floats(args[color_key])), isRGBA=True)
        gc.set_linewidth(float(args.get("lw", "1")))
        gc.set_capstyle(args.get("cap", "butt"))
        gc.set_joinstyle(args.get("join", "round"))
        if "dashes" in args:
            offset, lengths = args["dashes"].split(":")
            gc.set_dashes(float(offset), floats(lengths))
        return gc

    def run(self, op, target, args):
        r = self.renderer
        if op == "fill":
            gc = self.gc(args)
            gc.set_linewidth(0)
            r.draw_path(gc, self.paths[target], self.flip, tuple(floats(args["color"])))
        elif op == "stroke":
            gc = self.stroke_gc(args, "color")
            r.draw_path(gc, self.paths[target], self.flip, None)
        elif op == "hatch":
            gc = self.gc(args)
            gc.set_linewidth(0)
            hatch = self.paths[args["hatch"]]
            gc.get_hatch_path = lambda density=6.0: hatch
            gc.set_hatch_color(tuple(floats(args["hcolor"])))
            gc.set_hatch_linewidth(float(args["hlw"]))
            face = tuple(floats(args["face"])) if "face" in args else None
            r.draw_path(gc, self.paths[target], self.flip, face)
        elif op == "markers":
            gc = self.stroke_gc(args, "edge")
            at = floats(args["at"])
            positions = Path(np.array(at, float).reshape(-1, 2))
            face = tuple(floats(args["face"])) if "face" in args else None
            r.draw_markers(gc, self.paths[target], Affine2D().scale(1, -1), positions, self.flip, face)
        elif op == "gouraud":
            gc = self.gc(args)
            points = np.array(floats(args["tri"]), float).reshape(1, 3, 2)
            colors = np.array([floats(args[k]) for k in ("c0", "c1", "c2")], float).reshape(1, 3, 4)
            r.draw_gouraud_triangles(gc, points, colors, self.flip)
        elif op == "image":
            gc = self.gc(args)
            x, y = (int(v) for v in floats(args["at"]))
            w, h = (int(v) for v in floats(args["size"]))
            # RendererAgg reads image rows bottom to top.
            im = np.ascontiguousarray(image_pixels(w, h)[::-1])
            r.draw_image(gc, x, self.height - (y + h), im)
        elif op == "text":
            gc = self.gc(args)
            gc.set_foreground(tuple(floats(args["color"])), isRGBA=True)
            x, y = (int(v) for v in floats(args["at"]))
            w, h = (int(v) for v in floats(args["size"]))
            r._renderer.draw_text_image(coverage_pixels(w, h), x, y + h, 0, gc)
        else:
            raise ValueError(f"unknown op {op!r}")

    def pixels(self):
        return np.asarray(self.renderer.buffer_rgba()).copy()


def rle(pixels):
    flat = pixels.reshape(-1, 4)
    out = bytearray()
    i = 0
    while i < len(flat):
        run = 1
        while i + run < len(flat) and run < 255 and (flat[i + run] == flat[i]).all():
            run += 1
        out.append(run)
        out.extend(flat[i].tobytes())
        i += run
    return bytes(out)


def main():
    # Long paths must reach Agg unsimplified, as plt-raster receives them.
    mpl.rcParams["path.simplify"] = False
    cases, case = [], None
    for raw in (HERE / "cases.txt").read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        head, *rest = line.split()
        if head == "case":
            case = Case(rest[0], int(rest[1]), int(rest[2]))
        elif head == "end":
            cases.append((case.name, case.pixels()))
            case = None
        elif head == "path":
            case.paths[rest[0]] = parse_path(rest[1:])
        else:
            target = rest[0] if rest and "=" not in rest[0] else None
            args = dict(token.split("=", 1) for token in rest if "=" in token)
            case.run(head, target, args)

    out = bytearray(b"PLTAGG1\n")
    out += struct.pack("<I", len(cases))
    for name, px in cases:
        data = rle(px)
        encoded = name.encode()
        out += struct.pack("<H", len(encoded)) + encoded
        out += struct.pack("<III", px.shape[1], px.shape[0], len(data)) + data
    (HERE / "agg_reference.bin").write_bytes(bytes(out))
    print(f"wrote {len(cases)} cases, {len(out)} bytes")


if __name__ == "__main__":
    main()
