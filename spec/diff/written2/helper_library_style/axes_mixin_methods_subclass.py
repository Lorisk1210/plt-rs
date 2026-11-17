# A user Axes subclass adding domain methods (bands, markers), registered as a projection and used through add_subplot and subplots.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.axes import Axes
from matplotlib.projections import register_projection


class TimelineAxes(Axes):
    name = "timeline"

    def clear(self):
        super().clear()
        self.events = []
        self.set_yticks([])
        self.spines[["left", "right", "top"]].set_visible(False)

    def band(self, start, end, label, color="C0"):
        patch = self.axvspan(start, end, 0.3, 0.7, color=color, alpha=0.5)
        self.text((start + end) / 2, 0.5, label, ha="center", va="center", transform=self.get_xaxis_transform())
        self.events.append(("band", start, end, label))
        return patch

    def milestone(self, at, label):
        ln = self.axvline(at, color="k", lw=1)
        self.annotate(label, (at, 0.8), xycoords=("data", "axes fraction"), rotation=45)
        self.events.append(("milestone", at, label))
        return ln


register_projection(TimelineAxes)
fig = plt.figure(figsize=(6, 2), dpi=60)
ax = fig.add_subplot(projection="timeline")
ax.band(0, 3, "design")
ax.band(3, 7, "build", color="C1")
ax.milestone(7, "release")
ax.set_xlim(-1, 9)
fig.canvas.draw()
print(type(ax).__name__, ax.name, ax.events, len(ax.patches), len(ax.texts), ax.get_yticks().tolist())
fig2, axs = plt.subplots(2, 1, subplot_kw={"projection": "timeline"})
axs[0].band(1, 2, "x")
axs[1].clear()
print([type(a).__name__ for a in axs], [len(a.events) for a in axs])
print([s.get_visible() for s in ax.spines.values()])
ax.cla()
print(ax.events, len(ax.patches))
fig.savefig("timeline.png")
