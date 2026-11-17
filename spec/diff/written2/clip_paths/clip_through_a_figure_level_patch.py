# Clip artists in two axes to one figure-level patch (transform=fig.transFigure) and save at several dpi values.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch

fig, axs = plt.subplots(1, 2, figsize=(6, 3))
window = FancyBboxPatch((0.25, 0.2), 0.5, 0.6, boxstyle="round,pad=0.02,rounding_size=0.08",
                        transform=fig.transFigure, fc="none", ec="k", lw=2)
fig.add_artist(window)
for k, ax in enumerate(axs):
    img = ax.imshow(np.add.outer(np.arange(30), np.arange(30) * (k + 1)) % 11, cmap="tab20")
    img.set_clip_path(window)
    ax.plot(np.arange(30), np.arange(30)[::-1], "w-", lw=3, clip_path=window)
for dpi in (50, 100, 150):
    fig.savefig(f"window_{dpi}.png", dpi=dpi)
    fig.canvas.draw()
    print(dpi, [round(v, 2) for v in window.get_window_extent().bounds])
print(type(axs[0].images[0].get_clip_path()).__name__, type(axs[1].lines[0].get_clip_path()).__name__)
print([round(v, 3) for v in window.get_path().get_extents().bounds])
print(window.get_boxstyle().pad, window.get_boxstyle().rounding_size)
fig.set_dpi(80)
fig.canvas.draw()
buf = np.asarray(fig.canvas.buffer_rgba())
print(buf.shape)
print(int((buf[..., :3] == 255).all(axis=2).sum()) > 0)
window.set_boxstyle("square", pad=0)
fig.canvas.draw()
print([round(v, 2) for v in window.get_window_extent().bounds])
