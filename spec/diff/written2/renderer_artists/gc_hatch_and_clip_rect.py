# A custom Artist using gc.set_hatch, set_hatch_color and set_clip_rectangle, drawn in a 2x2 grid under Agg.
import numpy as np
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.artist import Artist
from matplotlib.path import Path
from matplotlib.transforms import IdentityTransform, Bbox


class HatchedBand(Artist):
    def __init__(self, hatch, color):
        super().__init__()
        self.hatch = hatch
        self.color = color

    def draw(self, renderer):
        bb = self.axes.bbox
        band = Path([(bb.x0 - 20, bb.y0 + 0.3 * bb.height), (bb.x1 + 20, bb.y0 + 0.3 * bb.height),
                     (bb.x1 + 20, bb.y0 + 0.7 * bb.height), (bb.x0 - 20, bb.y0 + 0.7 * bb.height)], closed=True)
        gc = renderer.new_gc()
        gc.set_clip_rectangle(bb)
        gc.set_hatch(self.hatch)
        gc.set_hatch_color(self.color)
        gc.set_foreground(self.color)
        renderer.draw_path(gc, band, IdentityTransform(), rgbFace=None)
        print(self.hatch, gc.get_hatch(), [round(v, 3) for v in gc.get_clip_rectangle().bounds])
        gc.restore()


fig = Figure(figsize=(5, 4), dpi=72)
canvas = FigureCanvasAgg(fig)
axs = fig.subplots(2, 2)
for ax, (h, c) in zip(axs.flat, [("//", "C0"), ("xx", "C1"), ("..", "C2"), ("o", "C3")]):
    ax.add_artist(HatchedBand(h, c))
canvas.draw()
buf = np.asarray(canvas.buffer_rgba())
outside = buf[2, 2]
print(outside.tolist(), int((buf[..., :3].sum(axis=2) < 600).sum()) > 100)
fig.savefig("bands.png")
