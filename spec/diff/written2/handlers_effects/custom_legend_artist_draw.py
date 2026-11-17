# A legend for a custom Artist subclass: legend_artist method on the handler, get_legend_handles_labels picking it up from the axes.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import Artist
from matplotlib.lines import Line2D
from matplotlib.patches import Rectangle


class RangeBar(Artist):
    """A min-max bar with a midpoint tick, drawn with two sub-artists."""

    def __init__(self, x, lo, hi, color="C0", label=None):
        super().__init__()
        self.rect = Rectangle((x - 0.2, lo), 0.4, hi - lo, fc=color, alpha=0.5)
        self.mid = Line2D([x - 0.25, x + 0.25], [(lo + hi) / 2] * 2, color="k")
        self.color = color
        self.set_label(label)

    def set_figure(self, fig):
        super().set_figure(fig)
        self.rect.set_figure(fig)
        self.mid.set_figure(fig)

    def draw(self, renderer):
        for a in (self.rect, self.mid):
            a.set_transform(self.axes.transData)
            a.draw(renderer)


class RangeHandler:
    def legend_artist(self, legend, orig_handle, fontsize, handlebox):
        x0, y0 = handlebox.xdescent, handlebox.ydescent
        w, h = handlebox.width, handlebox.height
        patch = Rectangle((x0, y0), w, h, fc=orig_handle.color, alpha=0.5, transform=handlebox.get_transform())
        handlebox.add_artist(patch)
        return patch


fig, ax = plt.subplots(figsize=(4, 3))
for k, (lo, hi) in enumerate([(1, 3), (0.5, 2), (2, 4)]):
    ax.add_artist(RangeBar(k, lo, hi, color=f"C{k}", label=f"range {k}"))
ax.set_xlim(-1, 3)
ax.set_ylim(0, 5)
handles, labels = ax.get_legend_handles_labels()
print(len(handles), labels)
handles = [a for a in ax.artists if isinstance(a, RangeBar)]
leg = ax.legend(handles, [h.get_label() for h in handles], handler_map={RangeBar: RangeHandler()})
fig.canvas.draw()
print([type(h).__name__ for h in leg.legend_handles], [round(v, 1) for v in leg.get_window_extent().bounds])
fig.savefig("range_legend.png")
