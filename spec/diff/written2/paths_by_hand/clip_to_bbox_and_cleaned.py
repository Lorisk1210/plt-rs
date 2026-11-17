# Path.clip_to_bbox inside/outside, cleaned() with every option, and interpolated(); prints vertices for a few shapes.
import numpy as np
from matplotlib.path import Path
from matplotlib.transforms import Bbox, Affine2D

tri = Path([(0, 0), (4, 0), (2, 4), (0, 0)], closed=True)
box = Bbox([[1, 1], [3, 3]])
inside = tri.clip_to_bbox(box, inside=True)
outside = tri.clip_to_bbox(box, inside=False)
print(np.round(inside.vertices, 4).tolist())
print(inside.codes.tolist() if inside.codes is not None else None)
print(len(outside.vertices))
wave = Path(np.c_[np.linspace(0, 10, 50), np.sin(np.linspace(0, 10, 50))])
c1 = wave.cleaned()
c2 = wave.cleaned(transform=Affine2D().scale(50), simplify=True)
c3 = wave.cleaned(transform=Affine2D().scale(50), remove_nans=True, clip=(0, -20, 200, 20))
c4 = wave.cleaned(curves=False, snap=True, stroke_width=2.0, transform=Affine2D().scale(50))
for c in (c1, c2, c3, c4):
    print(len(c.vertices), c.codes[-1], np.round(c.vertices[:2], 3).tolist())
nan_path = Path([(0, 0), (1, np.nan), (2, 2), (3, 3), (np.nan, 4), (5, 5)])
cn = nan_path.cleaned(remove_nans=True)
print(cn.codes.tolist(), np.round(cn.vertices, 3).tolist())
sq = Path.unit_rectangle()
interp = sq.interpolated(3)
print(len(interp.vertices), interp.codes.tolist())
print(np.round(interp.vertices, 4).tolist())
circ = Path.unit_circle().interpolated(2)
print(len(circ.vertices), circ.codes[:5].tolist())
print(np.round(Path([(0, 0), (1, 1)]).interpolated(4).vertices, 3).tolist())
curve = Path([(0, 0), (1, 2), (2, 0)], [Path.MOVETO, Path.CURVE3, Path.CURVE3])
print(len(curve.interpolated(5).vertices), curve.cleaned(curves=False).codes.tolist())
