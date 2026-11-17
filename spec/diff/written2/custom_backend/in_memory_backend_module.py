# A backend defined as an in-memory module (FigureCanvas + RendererBase subclass counting calls), selected with module:// names.
import sys
import types
import matplotlib
from matplotlib.backend_bases import FigureCanvasBase, FigureManagerBase, RendererBase, GraphicsContextBase

mod = types.ModuleType("counting_backend")


class CountingRenderer(RendererBase):
    def __init__(self, dpi):
        super().__init__()
        self.dpi = dpi
        self.counts = {}

    def _count(self, name):
        self.counts[name] = self.counts.get(name, 0) + 1

    def draw_path(self, gc, path, transform, rgbFace=None):
        self._count("path")

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        self._count("text")

    def draw_image(self, gc, x, y, im):
        self._count("image")

    def get_text_width_height_descent(self, s, prop, ismath):
        return len(s) * prop.get_size_in_points() * 0.6, prop.get_size_in_points(), 2.0

    def get_canvas_width_height(self):
        return 640, 480

    def flipy(self):
        return True

    def points_to_pixels(self, points):
        return points * self.dpi / 72

    def new_gc(self):
        return GraphicsContextBase()


class CountingCanvas(FigureCanvasBase):
    filetypes = {**FigureCanvasBase.filetypes, "count": "Draw-call counts"}

    def draw(self):
        self.renderer = CountingRenderer(self.figure.dpi)
        self.figure.draw(self.renderer)
        return self.renderer.counts

    def print_count(self, filename, **kwargs):
        counts = self.draw()
        with open(filename, "w") as f:
            f.write(repr(sorted(counts.items())))

    def get_default_filetype(self):
        return "count"


mod.FigureCanvas = CountingCanvas
mod.FigureManager = FigureManagerBase
sys.modules["counting_backend"] = mod
matplotlib.use("module://counting_backend")
import matplotlib.pyplot as plt
print(matplotlib.get_backend())
fig, ax = plt.subplots()
ax.plot([0, 1], [0, 1])
ax.set_title("counted")
ax.imshow([[0, 1], [1, 0]], extent=(0, 1, 0, 1))
print(type(fig.canvas).__name__, sorted(fig.canvas.draw().items()))
fig.savefig("calls.count")
fig.savefig("calls_default")
import os
print(sorted(p for p in os.listdir(".") if p.startswith("calls")), open("calls.count").read()[:60])
plt.close("all")
