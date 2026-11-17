# A HandlerBase subclass drawing a two-colour band for a custom proxy class, registered via handler_map and Legend.update_default_handler_map.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.legend import Legend
from matplotlib.legend_handler import HandlerBase
from matplotlib.patches import Rectangle


class Band:
    def __init__(self, low, high):
        self.low, self.high = low, high


class HandlerBand(HandlerBase):
    def create_artists(self, legend, orig_handle, xdescent, ydescent, width, height, fontsize, trans):
        top = Rectangle((-xdescent, -ydescent + height / 2), width, height / 2, fc=orig_handle.high, transform=trans)
        bottom = Rectangle((-xdescent, -ydescent), width, height / 2, fc=orig_handle.low, transform=trans)
        return [bottom, top]


fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([0, 1], [0, 1], label="line")
band = Band("lightblue", "navy")
leg = ax.legend([band, ax.lines[0]], ["band", "line"], handler_map={Band: HandlerBand()})
fig.canvas.draw()
print([type(h).__name__ for h in leg.legend_handles])
print(len(leg.get_children()), type(leg.get_legend_handler_map()[Band]).__name__)
default = Legend.get_default_handler_map()
Legend.update_default_handler_map({Band: HandlerBand()})
leg2 = ax.legend([band], ["registered"])
print(type(leg2.legend_handles[0]).__name__, Band in Legend.get_default_handler_map())
Legend.set_default_handler_map({k: v for k, v in Legend.get_default_handler_map().items() if k is not Band})
print(Band in Legend.get_default_handler_map())
fig.savefig("band_legend.png")
print([round(v, 2) for v in leg2.get_window_extent().bounds])
