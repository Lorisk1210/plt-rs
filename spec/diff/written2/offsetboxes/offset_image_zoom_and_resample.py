# OffsetImage at many zooms with dpi_cor on and off, saved at two dpi values; extents and image data shape read back.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.offsetbox import OffsetImage, AnnotationBbox

data = np.add.outer(np.arange(10), np.arange(12)) % 5
fig, ax = plt.subplots(figsize=(6, 3))
ax.set_xlim(0, 6)
ax.set_ylim(0, 2)
items = []
for k, zoom in enumerate([0.5, 1, 2, 3.5]):
    for j, dpi_cor in enumerate((True, False)):
        im = OffsetImage(data, zoom=zoom, dpi_cor=dpi_cor, cmap="Set2", interpolation="nearest")
        ab = AnnotationBbox(im, (k * 1.4 + 0.6, 0.5 + j), frameon=False)
        ax.add_artist(ab)
        items.append((zoom, dpi_cor, im, ab))
for dpi in (72, 144):
    fig.set_dpi(dpi)
    fig.canvas.draw()
    r = fig.canvas.get_renderer()
    for zoom, dpi_cor, im, ab in items:
        print(dpi, zoom, dpi_cor, [round(v, 1) for v in im.get_bbox(r).bounds])
im0 = items[0][2]
im0.set_zoom(1.5)
im0.set_data(data[:5, :5])
fig.canvas.draw()
print(im0.get_zoom(), im0.get_data().shape, [round(v, 1) for v in im0.get_bbox(fig.canvas.get_renderer()).bounds])
print(type(im0.get_children()[0]).__name__, im0.get_children()[0].get_interpolation())
fig.savefig("zooms.png", dpi=72)
fig.savefig("zooms_hi.png", dpi=144)
