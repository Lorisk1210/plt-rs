# A custom Artist that draws a rounded frame with draw_path and a configured GraphicsContext, saved under Agg.
from __future__ import annotations

import numpy as np
from matplotlib.artist import Artist
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, IdentityTransform


class Frame(Artist):
    """A frame around the axes, drawn in display coordinates."""

    def __init__(self, ax, pad_points: float = 6.0) -> None:
        super().__init__()
        self.ax = ax
        self.pad_points = pad_points
        self.calls: list[tuple] = []

    def draw(self, renderer) -> None:
        if not self.get_visible():
            return
        pad = renderer.points_to_pixels(self.pad_points)
        box = self.ax.bbox.padded(pad)
        verts = [(box.x0, box.y0), (box.x1, box.y0), (box.x1, box.y1), (box.x0, box.y1), (box.x0, box.y0)]
        path = Path(verts, closed=True)
        gc = renderer.new_gc()
        gc.set_foreground("darkred")
        gc.set_linewidth(renderer.points_to_pixels(2.0))
        gc.set_dashes(0, [6, 3])
        gc.set_joinstyle("round")
        gc.set_capstyle("butt")
        gc.set_alpha(0.8)
        renderer.draw_path(gc, path, IdentityTransform(), rgbFace=(1.0, 0.9, 0.8, 0.3))
        self.calls.append((round(float(pad), 6), gc.get_linewidth(), gc.get_joinstyle(), gc.get_capstyle(), gc.get_alpha()))
        gc.restore()


def main() -> None:
    fig = Figure(figsize=(4, 3), dpi=100)
    FigureCanvasAgg(fig)
    ax = fig.add_axes((0.2, 0.2, 0.6, 0.6))
    ax.plot(np.linspace(0, 1, 20), np.linspace(0, 1, 20) ** 2)
    frame = Frame(ax)
    fig.add_artist(frame)
    fig.savefig("frame.png")
    fig.savefig("frame_150.png", dpi=150)
    for call in frame.calls:
        print(call)
    print(repr(Affine2D().scale(2).transform_point((1.5, 2.5))))


if __name__ == "__main__":
    main()
