# A tiny projection: an Axes subclass registered by name that sets defaults (grid, no top/right spines).
import matplotlib.pyplot as plt
from matplotlib.axes import Axes
from matplotlib.projections import register_projection


class CleanAxes(Axes):
    name = "clean"

    def clear(self):
        super().clear()
        self.grid(True, alpha=0.3)
        self.spines[["top", "right"]].set_visible(False)


register_projection(CleanAxes)
fig, axs = plt.subplots(1, 2, figsize=(8, 3), subplot_kw={"projection": "clean"})
axs[0].plot([1, 3, 2])
axs[1].bar(["a", "b"], [2, 3])
print([type(ax).__name__ for ax in axs], axs[0].name)
fig.savefig("clean_axes.png")
