# A vector-like custom renderer driven through MixedModeRenderer: rasterized artists reach draw_image, the rest reach draw_path.
import sys
import types
import numpy as np
import matplotlib
from matplotlib.backend_bases import FigureCanvasBase, RendererBase, GraphicsContextBase
from matplotlib.backends.backend_mixed import MixedModeRenderer

log = []


class LogRenderer(RendererBase):
    def __init__(self, width, height, dpi):
        super().__init__()
        self.width, self.height, self.dpi = width, height, dpi

    def draw_path(self, gc, path, transform, rgbFace=None):
        log.append(("path", len(path.vertices)))

    def draw_image(self, gc, x, y, im):
        log.append(("image", im.shape))

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        log.append(("text", s))

    def get_canvas_width_height(self):
        return self.width, self.height

    def finalize(self):
        log.append(("finalize", None))

    def points_to_pixels(self, p):
        return p * self.dpi / 72

    def new_gc(self):
        return GraphicsContextBase()


class LogCanvas(FigureCanvasBase):
    def print_log(self, filename, *, dpi=72, bbox_inches_restore=None, **kw):
        w, h = self.figure.get_size_inches()
        self.figure.dpi = 72
        renderer = MixedModeRenderer(self.figure, w, h, dpi, LogRenderer(w * 72, h * 72, 72),
                                     bbox_inches_restore=bbox_inches_restore)
        self.figure.draw(renderer)
        renderer.finalize()


mod = types.ModuleType("log_backend")
mod.FigureCanvas = LogCanvas
sys.modules["log_backend"] = mod
matplotlib.use("module://log_backend")
import matplotlib.pyplot as plt
fig, ax = plt.subplots(figsize=(3, 2))
ax.plot(np.arange(100), np.sin(np.arange(100)), rasterized=True)
ax.plot([0, 99], [0, 0])
ax.text(10, 0.5, "label")
fig.savefig("out.log", dpi=100)
kinds = sorted({k for k, _ in log})
print(kinds, [v for k, v in log if k == "image"], sum(1 for k, _ in log if k == "path") > 5, [v for k, v in log if k == "text"][:3])
log.clear()
fig.savefig("out.log", dpi=200)
print([v for k, v in log if k == "image"])
plt.close("all")
