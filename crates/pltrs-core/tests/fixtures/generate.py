"""Generate pltrs-core's matplotlib comparison fixtures.

Run from the repository root (needs the project's matplotlib venv)::

    .venv/bin/python -I crates/pltrs-core/tests/fixtures/generate.py

Writes ``convert.json`` (path conditioning: ``cleanup_path``,
``convert_to_string``) and ``geometry.json`` (containment, picking,
intersections, clipping, polygons, extents, hatches) next to this script.
Every expected value comes straight from matplotlib.  Long inputs are not
stored: they are regenerated on both sides from a SplitMix64 stream with
plain arithmetic, so they are bit-identical.

The fixtures next to this script are matplotlib's on Linux x86-64.
matplotlib's wheel for arm64 macOS fuses products into sums, which pltrs
follows there: run there, this writes that platform's fixtures, into
``aarch64-apple-darwin/``, which the tests built for it read instead.

``--scale K`` multiplies the number of random cases and ``--seed`` picks
another random stream (for local fuzzing; the committed fixtures use the
defaults).
"""

import argparse
import json
import math
import pathlib
import platform
import random
import sys

import matplotlib
import numpy as np
from matplotlib import _path, hatch
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, Bbox

HERE = pathlib.Path(__file__).resolve().parent
#: Where this machine's fixtures go: arm64 macOS has its own.
OUT = (HERE / "aarch64-apple-darwin"
       if sys.platform == "darwin" and platform.machine() == "arm64" else HERE)
M, L, C3, C4, CL = 1, 2, 3, 4, 79
MASK64 = (1 << 64) - 1


class SplitMix64:
    def __init__(self, seed):
        self.state = seed & MASK64

    def next_u64(self):
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK64
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK64
        return z ^ (z >> 31)

    def uniform(self):
        return (self.next_u64() >> 11) * (1.0 / 9007199254740992.0)


def noisy_line(n, seed, nan_gaps):
    """Mirror of ``generated`` in ``tests/common/mod.rs``."""
    rng = SplitMix64(seed)
    amplitudes = [0.01, 0.05, 0.3, 2.0]
    amplitude = trend = slope = 0.0
    vertices = []
    for i in range(n):
        if i % 64 == 0:
            amplitude = amplitudes[rng.next_u64() % 4]
            slope = (rng.uniform() - 0.5) * 0.8
        trend += slope
        y = trend + amplitude * (2.0 * rng.uniform() - 1.0)
        gap = nan_gaps and rng.next_u64() % 97 == 0
        vertices.append([i * 0.5, math.nan if gap else y])
    return Path(np.array(vertices)), {"generator": {
        "kind": "noisy", "n": n, "seed": seed, "nan_gaps": nan_gaps}}


def path_json(p):
    return {"vertices": np.asarray(p.vertices, float).reshape(-1, 2).tolist(),
            "codes": None if p.codes is None else [int(c) for c in p.codes]}


def make_path(vertices, codes):
    return Path(np.asarray(vertices, float).reshape(-1, 2),
                None if codes is None else np.asarray(codes, np.uint8))


def random_path(rng, n_max=24, curves=True, nans=True, grid=None):
    """Random moves/lines/curves/closures; coordinates on a grid or not."""
    n = rng.randint(1, n_max)
    grid = rng.random() < 0.4 if grid is None else grid

    def coord():
        if grid:
            return float(rng.randint(-3, 12)) + (0.5 if rng.random() < 0.2 else 0.0)
        return round(rng.uniform(-5, 15), 6)

    vertices, codes = [], []
    while len(vertices) < n:
        r = rng.random()
        if not vertices or r < 0.1:
            code, k = M, 1
        elif r < 0.7 or not curves:
            code, k = (CL, 1) if (r > 0.92 and not curves) else (L, 1)
        elif r < 0.8:
            code, k = C3, 2
        elif r < 0.9:
            code, k = C4, 3
        else:
            code, k = CL, 1
        for _ in range(k):
            x, y = coord(), coord()
            if nans and rng.random() < 0.06:
                x = math.nan
            if nans and rng.random() < 0.04:
                y = rng.choice([math.inf, -math.inf, math.nan])
            vertices.append([x, y])
            codes.append(code)
    plain = all(c in (M, L) for c in codes) and rng.random() < 0.4
    return make_path(vertices, None if plain else codes)


def random_affine(rng):
    if rng.random() < 0.4:
        return [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
    return [round(v, 4) for v in (rng.uniform(1, 20), rng.uniform(-3, 3), rng.uniform(-3, 3),
                                  rng.uniform(1, 20), rng.uniform(-2, 2), rng.uniform(-2, 2))]


def has_nonfinite(p):
    return not np.all(np.isfinite(p.vertices))


def cleanup(p, affine, remove_nans, clip, snap, stroke_width, simplify, curves, sketch):
    vertices, codes = _path.cleanup_path(
        p, Affine2D.from_values(*affine), remove_nans, clip, snap, stroke_width,
        simplify, curves, sketch)
    assert codes[-1] == 0
    return {"vertices": vertices[:-1].tolist(), "codes": [int(c) for c in codes[:-1]]}


def cleanup_case(p, encoded, affine, remove_nans, clip, snap, stroke_width, simplify,
                 curves, sketch, threshold=0.111111111111):
    p.simplify_threshold = threshold
    return {"path": encoded, "transform": affine, "remove_nans": remove_nans,
            "clip": clip, "snap": snap, "stroke_width": stroke_width,
            "simplify": threshold if simplify else None, "curves": curves,
            "sketch": sketch,
            "expected": cleanup(p, affine, remove_nans, clip, snap, stroke_width,
                                simplify, curves, sketch)}


def random_cleanup_cases(rng, count):
    cases = []
    while len(cases) < count:
        p = random_path(rng)
        affine = random_affine(rng)
        remove_nans = rng.random() < 0.8
        clip = None if rng.random() < 0.5 else [
            round(rng.uniform(-2, 40), 3), round(rng.uniform(-2, 40), 3),
            round(rng.uniform(5, 150), 3), round(rng.uniform(5, 150), 3)]
        snap = rng.choice([None, True, False])
        stroke_width = rng.choice([0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0])
        has_curves = p.codes is not None and bool(np.any((p.codes == C3) | (p.codes == C4)))
        simplify = rng.random() < (0.15 if has_curves else 0.5)
        curves = rng.random() < 0.5
        sketch = None
        if rng.random() < 0.05 and (remove_nans or not has_nonfinite(p)):
            sketch = [rng.choice([0.0, 1.0, 3.0]), rng.choice([10.0, 128.0, 5.5]),
                      rng.choice([16.0, 2.0, 1.5])]
        flattened = not curves or (sketch is not None and sketch[0] != 0.0)
        if flattened and has_curves and not remove_nans and has_nonfinite(p):
            continue  # Agg would subdivide non-finite curves forever
        if sketch is not None and np.abs(np.nan_to_num(p.vertices)).max() * 20 > 150:
            continue  # keep sketched outputs small
        cases.append(cleanup_case(p, path_json(p), affine, remove_nans, clip, snap,
                                  stroke_width, simplify, curves, sketch))
    return cases


def long_line_cases(scale):
    cases = []
    identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
    for seed in range(1, 1 + scale):
        p, encoded = noisy_line(1000, seed, False)
        cases.append(cleanup_case(p, encoded, identity, True, None, None, 1.0, True, True, None))
        # Agg's canvas pipeline: device transform, clipping to the canvas.
        p, encoded = noisy_line(1200, seed + 100, True)
        device = [4.0, 0.0, 0.0, -3.0, 20.0, 400.0]
        cases.append(cleanup_case(p, encoded, device, True, [0.0, 0.0, 640.0, 480.0], None,
                                  1.0, True, True, None))
        cases.append(cleanup_case(p, encoded, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0], True, None,
                                  False, 1.0, True, True, None, 1.0))
    return cases


def sketch_cases():
    cases = []
    shapes = [make_path([[0, 0], [30, 0], [30, 20]], None),
              make_path([[0, 0], [15, 0], [15, 15], [0, 15], [0, 0]], [M, L, L, L, CL]),
              Path.circle((10, 10), 8)]
    for p in shapes:
        for sketch in ([2.0, 128.0, 16.0], [1.0, 10.0, 2.0]):
            cases.append(cleanup_case(p, path_json(p), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], True,
                                      None, False, 1.0, False, True, sketch))
    return cases


def string_cases(rng, count):
    codes_sets = {"svg": ([b"M", b"L", b"Q", b"C", b"z"], False),
                  "pdf": ([b"m", b"l", b"", b"c", b"h"], True),
                  "ps": ([b"m", b"l", b"", b"c", b"cl"], True)}
    cases = []
    for i in range(count):
        p = random_path(rng)
        affine = random_affine(rng)
        clip = None if rng.random() < 0.6 else [0.0, 0.0, round(rng.uniform(5, 60), 2),
                                                round(rng.uniform(5, 60), 2)]
        flavour = ["svg", "pdf", "ps"][i % 3]
        precision = rng.choice([6, 6, 3, 1, 0])
        codes, postfix = codes_sets[flavour]
        text = _path.convert_to_string(p, Affine2D.from_values(*affine), clip, False, None,
                                       precision, codes, postfix)
        cases.append({"path": path_json(p), "transform": affine, "clip": clip,
                      "flavour": flavour, "precision": precision,
                      "expected": text.decode("ascii")})
    return cases


def flatten_rect(b):
    return [float(v) for v in np.asarray(b).ravel()]


def containment_cases(rng, count):
    cases = []
    while len(cases) < count:
        p = random_path(rng, n_max=14, nans=rng.random() < 0.3)
        radius = rng.choice([0.0, 0.0, 0.5, -0.5, 1.5, -1.0, 0.1])
        affine = random_affine(rng) if rng.random() < 0.3 else None
        pts = [[round(rng.uniform(-6, 16), 3), round(rng.uniform(-6, 16), 3)] for _ in range(24)]
        if affine is not None:
            pts = [[x * 5, y * 5] for x, y in pts]
        trans = None if affine is None else Affine2D.from_values(*affine)
        inside = _path.points_in_path(np.array(pts), radius, p, trans).astype(bool).tolist()
        cases.append({"path": path_json(p), "radius": radius, "transform": affine,
                      "points": pts, "expected": inside})
    return cases


def picking_cases(rng, count):
    cases = []
    for _ in range(count):
        paths = [random_path(rng, n_max=10, nans=False) for _ in range(rng.randint(1, 3))]
        transforms = [random_affine(rng) for _ in range(rng.randint(0, 2))]
        offsets = [[round(rng.uniform(-3, 3), 2), round(rng.uniform(-3, 3), 2)]
                   for _ in range(rng.randint(0, 3))]
        master = random_affine(rng)
        offset_trans = random_affine(rng)
        radius = rng.choice([0.0, 0.5, 2.0, 5.0])
        filled = rng.random() < 0.5
        mats = np.array([Affine2D.from_values(*t).get_matrix() for t in transforms]).reshape(-1, 3, 3)
        offs = np.array(offsets, float).reshape(-1, 2)
        queries = []
        for _ in range(8):
            x, y = round(rng.uniform(-40, 120), 2), round(rng.uniform(-40, 120), 2)
            hits = _path.point_in_path_collection(
                x, y, radius, Affine2D.from_values(*master), paths, mats, offs,
                Affine2D.from_values(*offset_trans), filled)
            queries.append({"point": [x, y], "hits": [int(i) for i in hits]})
        extents, minpos = _path.get_path_collection_extents(
            Affine2D.from_values(*master), paths, mats, offs, Affine2D.from_values(*offset_trans))
        cases.append({"paths": [path_json(p) for p in paths], "transforms": transforms,
                      "offsets": offsets, "master": master, "offset_transform": offset_trans,
                      "radius": radius, "filled": filled, "queries": queries,
                      "extents": flatten_rect(extents), "minpos": [float(v) for v in minpos]})
    return cases


def intersection_cases(rng, count):
    cases = []
    for _ in range(count):
        a = random_path(rng, n_max=10, nans=rng.random() < 0.2)
        b = random_path(rng, n_max=10, nans=rng.random() < 0.2)
        rect = sorted([round(rng.uniform(-4, 14), 2) for _ in range(2)]) + \
            sorted([round(rng.uniform(-4, 14), 2) for _ in range(2)])
        x0, x1, y0, y1 = rect
        cases.append({
            "a": path_json(a), "b": path_json(b), "rect": [x0, y0, x1, y1],
            "intersects": [bool(_path.path_intersects_path(a, b, f)) for f in (False, True)],
            "intersects_rect": [bool(_path.path_intersects_rectangle(a, x0, y0, x1, y1, f))
                                for f in (False, True)],
            "contains": bool(_path.path_in_path(a, None, b, None)),
        })
    return cases


def clipping_cases(rng, count):
    cases = []
    for _ in range(count):
        p = random_path(rng, n_max=14, nans=False)
        rect = [round(rng.uniform(-2, 6), 2), round(rng.uniform(-2, 6), 2),
                round(rng.uniform(4, 14), 2), round(rng.uniform(4, 14), 2)]
        inside = rng.random() < 0.8
        polys = _path.clip_path_to_rect(p, Bbox.from_extents(*rect), inside)
        affine = random_affine(rng)
        width, height = (0.0, 0.0) if rng.random() < 0.5 else (40.0, 30.0)
        closed_only = rng.random() < 0.5
        simplify = rng.random() < 0.3
        polygons = to_polygons(p, affine, width, height, closed_only, simplify)
        cases.append({"path": path_json(p), "rect": rect, "inside": inside,
                      "clipped": [np.asarray(q).tolist() for q in polys],
                      "transform": affine, "width": width, "height": height,
                      "closed_only": closed_only, "simplify": simplify, "polygons": polygons})
    long, encoded = noisy_line(1000, 7, True)
    rect = [100.0, -20.0, 400.0, 20.0]
    clipped = [np.asarray(q).tolist()
               for q in _path.clip_path_to_rect(long, Bbox.from_extents(*rect), True)]
    for width, closed_only in ((200.0, False),):
        affine = [0.5, 0.0, 0.0, 2.0, 5.0, 60.0]
        cases.append({"path": encoded, "rect": rect, "inside": True,
                      "clipped": clipped, "transform": affine, "width": width, "height": 120.0,
                      "closed_only": closed_only, "simplify": True,
                      "polygons": to_polygons(long, affine, width, 120.0, closed_only, True)})
    return cases


def to_polygons(p, affine, width, height, closed_only, simplify):
    """``Path.to_polygons`` with ``should_simplify`` decided by matplotlib."""
    with matplotlib.rc_context({"path.simplify": simplify,
                                "path.simplify_threshold": 0.111111111111}):
        fresh = Path(p.vertices, p.codes)
        polygons = fresh.to_polygons(Affine2D.from_values(*affine), width, height, closed_only)
    return [np.asarray(q).tolist() for q in polygons]


def overlap_cases(rng, count):
    cases = []
    for _ in range(count):
        a = [round(rng.uniform(-5, 5), 1) for _ in range(4)]
        boxes = [[round(rng.uniform(-6, 6), 1) for _ in range(4)] for _ in range(6)]
        arr = np.array(boxes, float).reshape(-1, 2, 2)
        cases.append({"bbox": a, "boxes": boxes,
                      "count": int(_path.count_bboxes_overlapping_bbox(
                          Bbox.from_extents(*a), arr))})
    return cases


def hatch_cases():
    patterns = ["/", "\\", "|", "-", "+", "x", "X", "o", ".", "*", "x*."]
    return [{"pattern": pattern, "density": 2, "expected": path_json(hatch.get_path(pattern, 2))}
            for pattern in patterns]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--scale", type=int, default=1)
    parser.add_argument("--out", type=pathlib.Path, default=OUT)
    parser.add_argument("--seed", type=int, default=20261016)
    args = parser.parse_args()
    k = args.scale
    rng = random.Random(args.seed)
    convert = {
        "cleanup": random_cleanup_cases(rng, 120 * k) + long_line_cases(2 * k) + sketch_cases(),
        "strings": string_cases(rng, 30 * k),
    }
    geometry = {
        "containment": containment_cases(rng, 40 * k),
        "picking": picking_cases(rng, 20 * k),
        "intersections": intersection_cases(rng, 60 * k),
        "clipping": clipping_cases(rng, 30 * k),
        "overlaps": overlap_cases(rng, 10 * k),
        "hatches": hatch_cases(),
    }
    args.out.mkdir(parents=True, exist_ok=True)
    for name, data in (("convert.json", convert), ("geometry.json", geometry)):
        (args.out / name).write_text(json.dumps(data, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()
