# Call renderer.draw_path_collection directly from an Artist to draw many squares with per-item colours and offsets.
import numpy as np
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.artist import Artist
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, IdentityTransform
import matplotlib.colors as mcolors


class Tiles(Artist):
    def __init__(self, n=8):
        super().__init__()
        self.n = n

    def draw(self, renderer):
        n = self.n
        square = Path.unit_rectangle()
        size = renderer.points_to_pixels(12)
        trans = [Affine2D().translate(-0.5, -0.5).scale(size).get_matrix()]
        grid = np.array([(i, j) for i in range(n) for j in range(n)], dtype=float) / (n - 1)
        offsets = grid
        offset_trans = self.axes.transAxes
        faces = mcolors.to_rgba_array([(i / n, j / n, 0.5, 1.0) for i in range(n) for j in range(n)])
        edges = mcolors.to_rgba_array(["k"])
        gc = renderer.new_gc()
        renderer.draw_path_collection(gc, IdentityTransform().frozen(), [square], trans, offsets, offset_trans,
                                      faces, edges, [0.5], [(0, None)], [True], [None], "screen")
        gc.restore()


fig = Figure(figsize=(4, 4), dpi=80)
canvas = FigureCanvasAgg(fig)
ax = fig.add_subplot()
ax.set_xlim(0, 1)
ax.set_ylim(0, 1)
tiles = Tiles()
ax.add_artist(tiles)
canvas.draw()
img = np.asarray(canvas.buffer_rgba())
print(img.shape, int((img[..., 3] > 0).sum()))
center = img[img.shape[0] // 2, img.shape[1] // 2]
print(center.tolist())
fig.savefig("tiles.png")
fig.savefig("tiles.pdf", metadata={"CreationDate": None})
