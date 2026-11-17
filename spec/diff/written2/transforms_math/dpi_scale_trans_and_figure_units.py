# Place artists in inches with fig.dpi_scale_trans and in figure fraction with transFigure; check positions across dpi.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
from matplotlib.transforms import blended_transform_factory

fig = plt.figure(figsize=(4, 3), dpi=100)
r_in = fig.add_artist(Rectangle((0.5, 0.5), 1, 0.5, transform=fig.dpi_scale_trans, fc="C0"))
r_fr = fig.add_artist(Rectangle((0.6, 0.6), 0.3, 0.3, transform=fig.transFigure, fc="C1"))
mixed = blended_transform_factory(fig.dpi_scale_trans, fig.transFigure)
r_mx = fig.add_artist(Rectangle((2.5, 0.1), 0.5, 0.2, transform=mixed, fc="C2"))
for dpi in (100, 50, 200):
    fig.set_dpi(dpi)
    fig.canvas.draw()
    print(dpi, [[round(v, 3) for v in r.get_window_extent().bounds] for r in (r_in, r_fr, r_mx)])
print(np.round(fig.dpi_scale_trans.get_matrix(), 6).tolist())
print(np.round(fig.transFigure.transform([[1, 1]]), 6).tolist(), fig.transSubfigure is fig.transFigure)
fig.set_size_inches(2, 2)
fig.canvas.draw()
print([[round(v, 3) for v in r.get_window_extent().bounds] for r in (r_in, r_fr, r_mx)])
fig.savefig("units.png")
print(len(fig.patches), len(fig.artists) if hasattr(fig, "artists") else None)
fig.set_size_inches(4, 3)
fig.set_dpi(72)
fig.canvas.draw()
print([[round(v, 3) for v in r.get_window_extent().bounds] for r in (r_in, r_fr, r_mx)])
print(np.round(mixed.transform([[1, 0.5]]), 6).tolist(), mixed.is_separable)
print(r_mx.get_transform() is mixed, type(r_in.get_transform()).__name__)
