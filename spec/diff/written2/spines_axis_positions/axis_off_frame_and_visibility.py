# axis('off'), set_frame_on, set_axis_off, xaxis.set_visible and spine visibility: what is drawn (counted by a renderer wrapper).
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backends.backend_agg import RendererAgg

calls = {"path": 0, "text": 0, "markers": 0}
orig_path, orig_text, orig_markers = RendererAgg.draw_path, RendererAgg.draw_text, RendererAgg.draw_markers


def draw_path(self, *a, **k):
    calls["path"] += 1
    return orig_path(self, *a, **k)


def draw_text(self, *a, **k):
    calls["text"] += 1
    return orig_text(self, *a, **k)


def draw_markers(self, *a, **k):
    calls["markers"] += 1
    return orig_markers(self, *a, **k)


RendererAgg.draw_path, RendererAgg.draw_text, RendererAgg.draw_markers = draw_path, draw_text, draw_markers
fig, ax = plt.subplots(figsize=(3, 3), dpi=40)
ax.plot([0, 1], [0, 1])
ax.set_title("t")
steps = [("normal", lambda: None), ("frame off", lambda: ax.set_frame_on(False)), ("xaxis hidden", lambda: ax.xaxis.set_visible(False)),
         ("spines hidden", lambda: ax.spines[:].set_visible(False)), ("axis off", lambda: ax.axis("off")),
         ("axis on", lambda: ax.set_axis_on())]
try:
    for name, step in steps:
        step()
        for k in calls:
            calls[k] = 0
        fig.canvas.draw()
        print(name, dict(calls), ax.axison, ax.get_frame_on(), ax.xaxis.get_visible())
finally:
    RendererAgg.draw_path, RendererAgg.draw_text, RendererAgg.draw_markers = orig_path, orig_text, orig_markers
fig.savefig("visibility.png")
