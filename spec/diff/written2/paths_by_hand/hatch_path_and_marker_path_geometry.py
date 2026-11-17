# Generate hatch paths with matplotlib.hatch.get_path for each pattern and density; count vertices and segment codes.
import numpy as np
from matplotlib import hatch
from matplotlib.path import Path

patterns = ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*", "//", "xx", "oo", "/.", "*-"]
for pat in patterns:
    for density in (6, 12):
        p = hatch.get_path(pat, density)
        codes = p.codes if p.codes is not None else np.array([])
        n_move = int(np.sum(codes == Path.MOVETO))
        print(pat.ljust(3), density, len(p.vertices), n_move, np.round(p.get_extents().bounds, 3).tolist())
print([c.__name__ for c in hatch._hatch_types])
for cls in hatch._hatch_types:
    obj = cls("xx//", 6)
    print(cls.__name__, obj.num_vertices, getattr(obj, "num_lines", None))
empty = hatch.get_path("", 6)
print(len(empty.vertices), empty.codes)
print(np.round(hatch.get_path("o", 4).vertices[:4], 4).tolist())
for dens in (4, 6, 9, 15):
    p = hatch.get_path("x+", dens)
    print(dens, len(p.vertices), np.round(p.vertices.max(axis=0), 3).tolist())
counts = {pat: len(hatch.get_path(pat, 6).vertices) for pat in "/\\|-+xoO.*"}
print(counts)
print(sorted(counts, key=counts.get)[:3])
mixed = hatch.get_path("/" * 3 + "o" * 2, 6)
print(len(mixed.vertices), np.unique(mixed.codes).tolist())
dots = hatch.get_path("." * 4, 6)
print(len(dots.vertices), round(float(np.abs(dots.vertices).max()), 4))
print(len(hatch.get_path("|-", 6).vertices), len(hatch.get_path("|-").vertices))
