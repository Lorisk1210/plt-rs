# An Artist drawing a small RGBA image with renderer.draw_image and a caption with draw_text, measured first.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import Artist
from matplotlib.font_manager import FontProperties


class Badge(Artist):
    def __init__(self, text, xy_axes=(0.05, 0.05)):
        super().__init__()
        self.text = text
        self.xy_axes = xy_axes
        self.measured = None

    def draw(self, renderer):
        x, y = self.axes.transAxes.transform(self.xy_axes)
        h, w = 24, 24
        yy, xx = np.mgrid[0:h, 0:w]
        rgba = np.zeros((h, w, 4), dtype=np.uint8)
        rgba[..., 0] = (xx * 255 // (w - 1)).astype(np.uint8)
        rgba[..., 2] = (yy * 255 // (h - 1)).astype(np.uint8)
        rgba[..., 3] = 255
        gc = renderer.new_gc()
        renderer.draw_image(gc, round(x), round(y), rgba[::-1])
        prop = FontProperties(family="DejaVu Sans", size=10, weight="bold")
        tw, th, td = renderer.get_text_width_height_descent(self.text, prop, ismath=False)
        self.measured = (round(tw, 3), round(th, 3), round(td, 3))
        gc.set_foreground("navy")
        renderer.draw_text(gc, x + w + 4, y + td, self.text, prop, 0.0, ismath=False)
        gc.restore()


fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
ax.plot([0, 1], [1, 0])
badge = Badge("calibrated")
ax.add_artist(badge)
fig.savefig("badge.png")
print(badge.measured)
r = fig.canvas.get_renderer()
print(r.get_text_width_height_descent("calibrated", FontProperties(size=10, weight="bold"), ismath=False))
print(r.get_text_width_height_descent(r"$\alpha_i^2$", FontProperties(size=12), ismath=True))
