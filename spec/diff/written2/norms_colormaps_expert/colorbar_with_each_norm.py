# Colorbars for each norm: tick locations, scales, extend triangles, boundaries/values and spacing proportional vs uniform.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import colors

data = np.linspace(0.5, 50, 100).reshape(10, 10)
norms = [colors.Normalize(0, 50), colors.LogNorm(0.5, 50), colors.SymLogNorm(1, vmin=-50, vmax=50),
         colors.PowerNorm(2, 0, 50), colors.BoundaryNorm([0, 1, 5, 20, 50], 256), colors.TwoSlopeNorm(10, 0, 50)]
fig, axs = plt.subplots(2, 3, figsize=(9, 5), dpi=50)
for ax, norm in zip(axs.flat, norms):
    im = ax.imshow(data, norm=norm)
    cb = fig.colorbar(im, ax=ax, extend="both")
    fig.canvas.draw()
    print(type(norm).__name__, cb.ax.get_yscale(), [round(t, 3) for t in cb.get_ticks()][:5], cb.spacing,
          [round(v, 3) for v in cb.ax.get_ylim()])
bn = colors.BoundaryNorm([0, 1, 5, 20, 50], 256)
fig2, ax2 = plt.subplots(figsize=(3, 3))
im = ax2.imshow(data, norm=bn)
for spacing in ("uniform", "proportional"):
    cb = fig2.colorbar(im, ax=ax2, spacing=spacing)
    fig2.canvas.draw()
    print(spacing, [round(v, 3) for v in cb.ax.get_ylim()], [round(t, 3) for t in cb.get_ticks()])
    cb.remove()
cb = fig2.colorbar(im, ax=ax2, boundaries=[0, 10, 30, 50], values=[5, 20, 40])
fig2.canvas.draw()
print(cb._boundaries.tolist(), cb._values.tolist())
fig.savefig("norm_cbars.png")
cb.set_ticks([5, 20, 40])
cb.minorticks_on()
fig2.canvas.draw()
print([round(t, 2) for t in cb.get_ticks()], len(cb.ax.yaxis.get_minorticklocs()))
