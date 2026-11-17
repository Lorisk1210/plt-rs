# Transform paths through chains of affine and non-affine transforms, inverting them back; compare vertex round trips and codes.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, blended_transform_factory

fig, axs = plt.subplots(1, 3, figsize=(7, 2.5))
axs[1].set_xscale("log")
axs[2].remove()
pax = fig.add_subplot(1, 3, 3, projection="polar")
pax.set_rlim(0, 2)
p = Path([(1, 0.5), (10, 1.5), (100, 0.2)], [Path.MOVETO, Path.CURVE3, Path.CURVE3])
fig.canvas.draw()
for ax in (axs[0], axs[1], pax):
    t = ax.transData
    tp = t.transform_path(p)
    back = t.inverted().transform_path(tp)
    print(ax.name, t.is_affine, np.round(tp.vertices, 2).tolist())
    print(np.allclose(back.vertices, p.vertices, rtol=1e-6), tp.codes.tolist() if tp.codes is not None else None)
blend = blended_transform_factory(axs[0].transData, axs[0].transAxes)
print(np.round(blend.transform_path(p).vertices, 2).tolist(), blend.is_separable)
na = axs[1].transData.transform_path_non_affine(p)
print(np.round(na.vertices, 4).tolist())
aff = Affine2D().rotate_deg(30).translate(5, 5)
print(np.round(p.transformed(aff).vertices, 4).tolist())
print(np.round(aff.transform_path_affine(p).vertices, 4).tolist())
pol = pax.transData.transform_path(Path([(0, 1), (np.pi / 2, 1)]))
print(len(pol.vertices), np.round(pol.vertices[[0, -1]], 2).tolist())
