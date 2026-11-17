# Smoothly shaded triangles via renderer.draw_gouraud_triangles from a custom Artist, saved under Agg and PDF.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import Artist
plt.rcParams["svg.hashsalt"] = "written2"


class Shaded(Artist):
    def draw(self, renderer):
        tri = np.array([[[0.1, 0.1], [0.9, 0.1], [0.5, 0.9]], [[0.1, 0.9], [0.9, 0.9], [0.5, 0.5]]])
        colors = np.array([[[1, 0, 0, 1], [0, 1, 0, 1], [0, 0, 1, 1]], [[1, 1, 0, 1], [0, 1, 1, 1], [1, 0, 1, 1]]], dtype=float)
        gc = renderer.new_gc()
        renderer.draw_gouraud_triangles(gc, tri, colors, self.axes.transAxes.frozen())
        gc.restore()


fig, ax = plt.subplots(figsize=(3, 3), dpi=60)
ax.add_artist(Shaded())
fig.canvas.draw()
img = np.asarray(fig.canvas.buffer_rgba())
x, y = ax.transAxes.transform((0.5, 0.35)).astype(int)
print(img[img.shape[0] - y, x].tolist())
fig.savefig("gouraud.png")
fig.savefig("gouraud.pdf", metadata={"CreationDate": None})
for xf, yf in [(0.2, 0.15), (0.5, 0.85), (0.8, 0.85)]:
    px, py = ax.transAxes.transform((xf, yf)).astype(int)
    print((xf, yf), img[img.shape[0] - py, px].tolist())
fig.savefig("gouraud.svg", metadata={"Date": None})
print(len(ax.artists), ax.artists[0].get_zorder())
