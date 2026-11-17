# An Artist that stamps a custom marker path at data points with renderer.draw_markers, in data coordinates.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import Artist
from matplotlib.path import Path
from matplotlib.transforms import Affine2D
plt.rcParams["svg.hashsalt"] = "written2"


class Stamps(Artist):
    def __init__(self, xy, size_points=10.0, color="teal"):
        super().__init__()
        self.xy = np.asarray(xy, dtype=float)
        self.size_points = size_points
        self.color = color
        star = Path.unit_regular_star(5, 0.4)
        self.marker = star
        self.drawn = 0

    def draw(self, renderer):
        scale = renderer.points_to_pixels(self.size_points) / 2
        marker_trans = Affine2D().scale(scale).rotate_deg(15)
        gc = renderer.new_gc()
        gc.set_foreground("k")
        gc.set_linewidth(0.5)
        self._set_gc_clip(gc)
        trans = self.axes.transData
        positions = trans.transform_path_non_affine(Path(self.xy))
        renderer.draw_markers(gc, self.marker, marker_trans.frozen(), positions, trans.get_affine().frozen(),
                              matplotlib.colors.to_rgba(self.color))
        gc.restore()
        self.drawn += 1


fig, ax = plt.subplots(figsize=(4, 3))
xy = np.column_stack([np.linspace(0, 10, 12), np.sin(np.linspace(0, 10, 12))])
stamps = Stamps(xy)
stamps.set_clip_box(ax.bbox)
ax.add_artist(stamps)
ax.set_xlim(-1, 11)
ax.set_ylim(-1.5, 1.5)
fig.savefig("stamps.png")
fig.savefig("stamps.svg", metadata={"Date": None})
print(stamps.drawn, stamps.axes is ax, stamps.get_figure() is fig)
print(repr(Path.unit_regular_star(5, 0.4).vertices[1]))
