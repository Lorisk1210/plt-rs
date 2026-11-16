# A custom legend handler class drawing a small rectangle with an inner line for a "band" entry.
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
from matplotlib.lines import Line2D


class BandHandle:
    pass


class BandHandler:
    def legend_artist(self, legend, orig_handle, fontsize, handlebox):
        x0, y0 = handlebox.xdescent, handlebox.ydescent
        w, h = handlebox.width, handlebox.height
        rect = Rectangle((x0, y0), w, h, facecolor="C0", alpha=0.3, transform=handlebox.get_transform())
        line = Line2D([x0, x0 + w], [y0 + h / 2] * 2, color="C0", transform=handlebox.get_transform())
        handlebox.add_artist(rect)
        handlebox.add_artist(line)
        return rect


fig, ax = plt.subplots(figsize=(5, 3.5))
ax.fill_between([0, 1, 2], [0, 1, 0], [1, 2, 1], alpha=0.3)
ax.plot([0, 1, 2], [0.5, 1.5, 0.5])
ax.legend([BandHandle()], ["mean and range"], handler_map={BandHandle: BandHandler()})
fig.savefig("band_legend.png")
print(len(ax.get_legend().legend_handles))
