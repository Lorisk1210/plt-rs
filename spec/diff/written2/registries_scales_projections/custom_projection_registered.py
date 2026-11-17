# A registered Axes subclass projection ("sketchy") with its own defaults, clear() override and a _gen_axes_patch.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.axes import Axes
from matplotlib.projections import register_projection, get_projection_class, get_projection_names
from matplotlib.patches import FancyBboxPatch


class RoundedAxes(Axes):
    name = "rounded"

    def _gen_axes_patch(self):
        return FancyBboxPatch((0, 0), 1, 1, boxstyle="round,pad=0,rounding_size=0.08")

    def clear(self):
        super().clear()
        self.grid(True, alpha=0.4)
        for spine in self.spines.values():
            spine.set_visible(False)
        self.set_facecolor("#f4f4fb")

    def label_corner(self, text):
        return self.text(0.98, 0.98, text, transform=self.transAxes, ha="right", va="top")


register_projection(RoundedAxes)
print("rounded" in get_projection_names(), get_projection_class("rounded").__name__)
fig = plt.figure(figsize=(6, 3))
a = fig.add_subplot(1, 2, 1, projection="rounded")
b = fig.add_subplot(1, 2, 2, projection="rounded")
a.plot(np.arange(10), np.arange(10) ** 0.5)
b.bar(range(4), [3, 1, 4, 1])
t = a.label_corner("A")
print(type(a).__name__, a.name, type(a.patch).__name__, a.patch.get_boxstyle().__class__.__name__)
print(a.xaxis._major_tick_kw.get("gridOn"), all(not s.get_visible() for s in a.spines.values()))
b.cla()
print(b.get_facecolor(), len(b.patches))
fig.savefig("rounded.png")
print(t.get_text(), a.get_facecolor())
