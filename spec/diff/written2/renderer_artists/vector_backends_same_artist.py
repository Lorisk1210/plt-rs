# One custom Artist (path + text + markers via the renderer) saved to PNG, SVG, PDF and PS; the program checks each output.
import io
import re
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import Artist
from matplotlib.path import Path
from matplotlib.transforms import Affine2D, IdentityTransform
from matplotlib.font_manager import FontProperties


class Glyph(Artist):
    def draw(self, renderer):
        cx, cy = self.axes.transAxes.transform((0.5, 0.5))
        r = renderer.points_to_pixels(30)
        circle = Path.circle((cx, cy), r)
        gc = renderer.new_gc()
        gc.set_foreground("purple")
        gc.set_linewidth(2)
        renderer.draw_path(gc, circle, IdentityTransform(), rgbFace=(0.9, 0.85, 1.0))
        pts = Path(np.array([[cx - r, cy], [cx + r, cy], [cx, cy + r]]))
        renderer.draw_markers(gc, Path.unit_rectangle(), Affine2D().translate(-0.5, -0.5).scale(6), pts,
                              IdentityTransform(), (1.0, 0.5, 0.0, 1.0))
        renderer.draw_text(gc, cx, cy, "center", FontProperties(size=9), 0.0, ismath=False)
        gc.restore()


plt.rcParams.update({"svg.hashsalt": "glyph", "svg.fonttype": "none"})
fig, ax = plt.subplots(figsize=(3, 3))
ax.add_artist(Glyph())
outputs = {}
for fmt in ("png", "svg", "pdf", "ps"):
    buf = io.BytesIO()
    meta = {"svg": {"Date": None}, "pdf": {"CreationDate": None}, "png": {"Software": None}}.get(fmt)
    fig.savefig(buf, format=fmt, metadata=meta)
    outputs[fmt] = buf.getvalue()
print(outputs["png"][:8])
svg = outputs["svg"].decode()
print(svg.count("<path"), ">center</text>" in svg, len(re.findall(r'<use xlink:href="#m', svg)))
print(outputs["pdf"][:8], b"/Type /Page" in outputs["pdf"])
print(outputs["ps"].splitlines()[0], b"%%BoundingBox" in outputs["ps"])
