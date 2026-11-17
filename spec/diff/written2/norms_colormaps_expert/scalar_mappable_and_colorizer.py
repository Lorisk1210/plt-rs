# ScalarMappable without an artist (for colorbars), the Colorizer object shared between artists, and set_array/clim callbacks.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import cm, colors

sm = cm.ScalarMappable(norm=colors.Normalize(0, 10), cmap="plasma")
print(sm.get_array(), sm.get_clim(), sm.cmap.name, np.round(sm.to_rgba(5), 4).tolist())
sm.set_array([])
fig, ax = plt.subplots()
cb = fig.colorbar(sm, ax=ax, label="free mappable")
fig.canvas.draw()
print([round(t, 2) for t in cb.get_ticks()], cb.ax.get_ylabel())
calls = []
sm.callbacks.connect("changed", lambda s: calls.append(s.get_clim()))
sm.set_clim(2, 8)
sm.set_norm(colors.LogNorm(1, 100))
sm.set_cmap("gray")
print(calls, type(cb.norm).__name__)
print(np.round(sm.to_rgba(np.array([1, 10, 100]), alpha=0.5), 3).tolist())
print(np.round(sm.to_rgba(np.array([[0.2, 0.4, 0.6]]), bytes=True), 3).tolist())
rgba_in = np.zeros((2, 2, 4))
print(sm.to_rgba(rgba_in).shape, sm.to_rgba(np.zeros((2, 2, 3), np.uint8)).dtype)
if hasattr(matplotlib, "colorizer"):
    cz = matplotlib.colorizer.Colorizer(cmap="viridis", norm=colors.Normalize(0, 1))
    a = ax.scatter([0, 1], [0, 1], c=[0.2, 0.8], colorizer=cz)
    b = ax.imshow(np.array([[0.0, 1.0]]), colorizer=cz)
    cz.vmax = 2
    fig.canvas.draw()
    print(a.get_clim(), b.get_clim(), a.colorizer is b.colorizer)
print(sm.get_alpha(), sm.norm.vmin, sm.norm.vmax)
fig.savefig("mappable.png")
