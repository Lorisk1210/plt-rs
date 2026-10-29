"""Render the RasterRenderer reference corpus with matplotlib's RendererAgg.

Reads backend_cases.txt (next to this script) and writes one PNG per case
into agg_backend/, which tests/agg_backend.rs compares RasterRenderer's
output against. Run with the reference interpreter:

    .venv/bin/python -I crates/pltrs-render/tests/fixtures/generate.py

Every op is a single call on matplotlib's RendererAgg with a
GraphicsContextBase configured from the case line, mirroring the
pltrs-render `Renderer` call made on the Rust side.
"""

import pathlib

import matplotlib as mpl
import numpy as np
from matplotlib.backends.backend_agg import RendererAgg
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, Bbox, IdentityTransform, TransformedPath
from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
OUT = HERE / "agg_backend"
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
            vertices.append((float(tokens[i]), float(tokens[i + 1])))
            codes.append(code)
            i += 2
        if cmd == "M":
            start = vertices[-1]
    return Path(np.array(vertices, float), codes)


def floats(text):
    return [float(v) for v in text.split(",")]


def colors(text):
    return [floats(c) for c in text.split(";")]


def image_pixels(w, h):
    """The deterministic RGBA test image, rows top to bottom."""
    j, i = np.mgrid[0:h, 0:w]
    rgba = np.stack(
        [(i * 37 + j * 11) % 256, (i * 5 + j * 53 + 7) % 256, (i * i + j * 3) % 256, (i * 19 + j * 29 + 40) % 256],
        axis=-1,
    )
    return rgba.astype(np.uint8)


class Case:
    def __init__(self, name, width, height, dpi):
        self.name = name
        self.renderer = RendererAgg(width, height, dpi)
        self.paths, self.affines = {}, {}

    def affine(self, args, key):
        return self.affines[args[key]] if key in args else IdentityTransform()

    def gc(self, args):
        gc = self.renderer.new_gc()
        gc.set_foreground(tuple(floats(args.get("color", "0,0,0,1"))), isRGBA=True)
        if "alpha" in args:
            gc.set_alpha(float(args["alpha"]))
        gc.set_linewidth(float(args.get("lw", "1")))
        gc.set_capstyle(args.get("cap", "butt"))
        gc.set_joinstyle(args.get("join", "round"))
        gc.set_antialiased(args.get("aa", "1") == "1")
        gc.set_snap({"auto": None, "true": True, "false": False}[args.get("snap", "auto")])
        if "dashes" in args:
            offset, lengths = args["dashes"].split(":")
            gc.set_dashes(float(offset), floats(lengths))
        if "clip" in args:
            x0, y0, x1, y1 = floats(args["clip"])
            gc.set_clip_rectangle(Bbox([[x0, y0], [x1, y1]]))
        if "clippath" in args:
            gc.set_clip_path(TransformedPath(self.paths[args["clippath"]], self.affine(args, "clipt")))
        if "hatch" in args:
            gc.set_hatch(args["hatch"])
            gc.set_hatch_color(tuple(floats(args["hcolor"])))
            gc.set_hatch_linewidth(float(args["hlw"]))
        if "sketch" in args:
            gc.set_sketch_params(*floats(args["sketch"]))
        return gc

    def face(self, args):
        if "face" not in args:
            return None
        face = tuple(floats(args["face"]))
        if "alpha" in args:
            face = face[:3] + (float(args["alpha"]),)
        return face

    def run(self, op, targets, args):
        r = self.renderer
        gc = self.gc(args)
        if op == "draw_path":
            r.draw_path(gc, self.paths[targets[0]], self.affine(args, "transform"), self.face(args))
        elif op == "draw_markers":
            marker, positions = (self.paths[t] for t in targets)
            r.draw_markers(gc, marker, self.affine(args, "marker_transform"), positions,
                           self.affine(args, "transform"), self.face(args))
        elif op == "collection":
            paths = [self.paths[p] for p in args["paths"].split(",")]
            transforms = [self.affines[t].get_matrix() for t in args.get("transforms", "").split(",") if t]
            offsets = np.array(floats(args["offsets"])).reshape(-1, 2) if "offsets" in args else np.zeros((0, 2))
            faces = np.array(colors(args["faces"])) if "faces" in args else np.zeros((0, 4))
            edges = np.array(colors(args["edges"])) if "edges" in args else np.zeros((0, 4))
            widths = np.array(floats(args.get("lws", "1")))
            aa = np.array([v == "1" for v in args.get("aa", "1").split(",")], dtype=bool)
            r.draw_path_collection(
                gc, self.affine(args, "master").frozen(), paths,
                np.array(transforms).reshape(-1, 3, 3), offsets, IdentityTransform(),
                faces, edges, widths, [(0, None)], aa, None, "screen", hatchcolors=np.zeros((0, 4)))
        elif op == "gouraud":
            tris = np.array([floats(t) for t in args["tris"].split(";")]).reshape(-1, 3, 2)
            cols = np.array([floats(c) for c in args["colors"].split(";")]).reshape(-1, 3, 4)
            r.draw_gouraud_triangles(gc, tris, cols, self.affine(args, "transform"))
        elif op == "image":
            x, y = floats(args["at"])
            w, h = (int(v) for v in floats(args["size"]))
            # RendererAgg reads image rows bottom to top.
            r.draw_image(gc, x, y, np.ascontiguousarray(image_pixels(w, h)[::-1]))
        else:
            raise ValueError(f"unknown op {op!r}")

    def save(self):
        Image.fromarray(np.asarray(self.renderer.buffer_rgba()).copy()).save(OUT / f"{self.name}.png")


def main():
    mpl.rcdefaults()
    OUT.mkdir(exist_ok=True)
    for stale in OUT.glob("*.png"):
        stale.unlink()
    case, count = None, 0
    for raw in (HERE / "backend_cases.txt").read_text().splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        head, *rest = line.split()
        if head == "case":
            case = Case(rest[0], int(rest[1]), int(rest[2]), float(rest[3]))
        elif head == "end":
            case.save()
            count += 1
        elif head == "path":
            case.paths[rest[0]] = parse_path(rest[1:])
        elif head == "affine":
            case.affines[rest[0]] = Affine2D.from_values(*(float(v) for v in rest[1:]))
        else:
            targets = [t for t in rest if "=" not in t]
            args = dict(t.split("=", 1) for t in rest if "=" in t)
            case.run(head, targets, args)
    print(f"wrote {count} cases")


if __name__ == "__main__":
    main()
