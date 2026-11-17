# imshow extent/origin/aspect combinations: resulting axes limits, aspect, and which data row lands at the top of the PNG.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

data = np.arange(12).reshape(3, 4)
for origin in ("upper", "lower"):
    for extent in (None, (0, 8, 0, 3), (8, 0, 3, 0)):
        for aspect in ("equal", "auto", 2.0):
            fig, ax = plt.subplots(figsize=(3, 3), dpi=30)
            im = ax.imshow(data, origin=origin, extent=extent, aspect=aspect, cmap="gray", interpolation="nearest")
            fig.canvas.draw()
            print(origin, extent, aspect, [round(v, 2) for v in ax.get_xlim()], [round(v, 2) for v in ax.get_ylim()],
                  ax.get_aspect(), [round(v, 2) for v in im.get_extent()])
            plt.close(fig)
fig, ax = plt.subplots(figsize=(3, 3), dpi=30)
im = ax.imshow(data, cmap="gray")
im.set_extent((10, 20, 30, 40))
fig.canvas.draw()
print([round(v, 2) for v in ax.get_xlim()], [round(v, 2) for v in ax.get_ylim()], ax.get_autoscale_on())
im.set_extent((0, 1, 0, 1))
fig.canvas.draw()
print([round(v, 2) for v in ax.get_xlim()], [round(v, 2) for v in ax.get_ylim()])
png = io.BytesIO()
fig2, ax2 = plt.subplots(figsize=(1, 1), dpi=30)
ax2.set_axis_off()
ax2.set_position([0, 0, 1, 1])
ax2.imshow(data, origin="lower", cmap="gray", interpolation="nearest", aspect="auto")
fig2.savefig(png, format="png")
png.seek(0)
img = plt.imread(png)
print(img.shape, round(float(img[0, 0, 0]), 3), round(float(img[-1, 0, 0]), 3))
