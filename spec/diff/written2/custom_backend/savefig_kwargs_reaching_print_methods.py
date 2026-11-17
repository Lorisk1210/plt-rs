# Which savefig keyword arguments reach a third-party canvas's print method: recorded by a subclass accepting **kwargs.
import io
import sys
import types
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import register_backend
from matplotlib.backends.backend_agg import FigureCanvasAgg

received = []


class RecordingCanvas(FigureCanvasAgg):
    def print_rec(self, filename_or_obj, **kwargs):
        received.append(sorted(kwargs))
        self.print_png(filename_or_obj)


mod = types.ModuleType("rec_backend")
mod.FigureCanvas = RecordingCanvas
sys.modules["rec_backend"] = mod
register_backend("rec", "rec_backend", "recorded")
fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
ax.plot([0, 1])
calls = [dict(), dict(dpi=80), dict(facecolor="r", edgecolor="b"), dict(transparent=True), dict(bbox_inches="tight"),
         dict(metadata={"Title": "x"}), dict(pad_inches=0.3, bbox_inches="tight"), dict(orientation="portrait"),
         dict(custom_option=5)]
for kw in calls:
    buf = io.BytesIO()
    try:
        fig.savefig(buf, format="rec", **kw)
        print(sorted(kw), received[-1], buf.getvalue()[:4])
    except TypeError as e:
        print(sorted(kw), "TypeError", str(e)[:60])
print(len(received), type(fig.canvas).__name__)
