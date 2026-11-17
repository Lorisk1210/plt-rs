# Subclass FigureCanvasAgg overriding print_png to stamp a watermark and count saves; installed per figure with set_canvas.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backends.backend_agg import FigureCanvasAgg


class StampedCanvas(FigureCanvasAgg):
    saves = 0

    def print_png(self, filename_or_obj, *, metadata=None, pil_kwargs=None, **ignored):
        StampedCanvas.saves += 1
        stamp = self.figure.text(0.99, 0.01, f"#{StampedCanvas.saves}", ha="right", va="bottom", fontsize=6)
        try:
            return super().print_png(filename_or_obj, metadata=metadata, pil_kwargs=pil_kwargs)
        finally:
            stamp.remove()


fig, ax = plt.subplots(figsize=(3, 2), dpi=60)
ax.plot([0, 1, 0])
original = fig.canvas
canvas = StampedCanvas(fig)
print(fig.canvas is canvas, type(original).__name__, canvas.manager)
for k in range(3):
    fig.savefig(f"stamped_{k}.png")
print(StampedCanvas.saves, len(fig.texts))
buf = io.BytesIO()
fig.savefig(buf, format="png")
buf.seek(0)
img = plt.imread(buf)
print(img.shape, round(float(img[-5:, -20:, 0].mean()), 3) < 1.0)
fig.savefig("vector.pdf", metadata={"CreationDate": None})
print(StampedCanvas.saves, type(fig.canvas).__name__)
FigureCanvasAgg(fig)
fig.savefig("plain.png")
print(StampedCanvas.saves, type(fig.canvas).__name__)
fig2 = plt.figure(figsize=(1, 1))
StampedCanvas(fig2)
fig2.canvas.print_figure("fig2.png")
print(StampedCanvas.saves, plt.get_fignums())
