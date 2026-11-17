# figimage at pixel offsets with resize, BboxImage tied to a text's bbox, and both saved at two dpi values.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.image import BboxImage
from matplotlib.transforms import Bbox, TransformedBbox

fig = plt.figure(figsize=(3, 2), dpi=100)
data = np.add.outer(np.arange(20), np.arange(30)) % 9
fi = fig.figimage(data, xo=10, yo=20, cmap="viridis", origin="upper")
fi2 = fig.figimage(data[:5, :5], xo=200, yo=100, resize=False, alpha=0.5)
ax = fig.add_axes([0.5, 0.1, 0.4, 0.4])
txt = ax.text(0.5, 0.5, "boxed image", ha="center", fontsize=12)
fig.canvas.draw()
bi = BboxImage(txt.get_window_extent, data=np.linspace(0, 1, 16).reshape(4, 4), cmap="coolwarm")
ax.add_artist(bi)
bi2 = BboxImage(TransformedBbox(Bbox([[0.1, 0.1], [0.3, 0.3]]), ax.transAxes), data=np.eye(3))
ax.add_artist(bi2)
for dpi in (100, 50):
    fig.set_dpi(dpi)
    fig.canvas.draw()
    r = fig.canvas.get_renderer()
    print(dpi, [round(v, 1) for v in fi.get_extent()], [round(v, 1) for v in bi.get_window_extent(r).bounds],
          [round(v, 1) for v in bi2.get_window_extent(r).bounds])
print(fi.ox, fi.oy, fi.get_array().shape, len(fig.images), type(bi).__name__)
buf = io.BytesIO()
fig.savefig(buf, format="png", dpi=100)
buf.seek(0)
img = plt.imread(buf)
print(img.shape, np.round(img[-25, 15], 3).tolist())
fig2 = plt.figure(figsize=(1, 1), dpi=50)
fig2.figimage(np.zeros((200, 300)), resize=True)
print(tuple(fig2.get_size_inches()))
