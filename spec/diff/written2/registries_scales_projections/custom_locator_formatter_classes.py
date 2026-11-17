# Locator and Formatter subclasses (with tick_values, view_limits, format_ticks, get_offset) used on both axes.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import Locator, Formatter, MaxNLocator


class FibonacciLocator(Locator):
    def __call__(self):
        vmin, vmax = self.axis.get_view_interval()
        return self.tick_values(vmin, vmax)

    def tick_values(self, vmin, vmax):
        a, b, out = 0, 1, []
        while a <= vmax:
            if a >= vmin:
                out.append(a)
            a, b = b, a + b
        return self.raise_if_exceeds(np.array(out, dtype=float))

    def view_limits(self, vmin, vmax):
        return 0, max(vmax, 1)


class OrdinalFormatter(Formatter):
    def __call__(self, x, pos=None):
        n = int(round(x))
        suffix = "th" if 10 <= n % 100 <= 20 else {1: "st", 2: "nd", 3: "rd"}.get(n % 10, "th")
        return f"{n}{suffix}"

    def get_offset(self):
        return "rank"


fig, ax = plt.subplots(figsize=(5, 3))
ax.plot(np.arange(0, 60), np.sqrt(np.arange(0, 60)))
ax.xaxis.set_major_locator(FibonacciLocator())
ax.xaxis.set_major_formatter(OrdinalFormatter())
ax.yaxis.set_major_locator(MaxNLocator(4, steps=[1, 2.5, 5]))
fig.canvas.draw()
print(ax.get_xticks().tolist())
print([t.get_text() for t in ax.get_xticklabels()], repr(ax.xaxis.get_offset_text().get_text()))
fmt = ax.xaxis.get_major_formatter()
print(fmt.format_ticks([1, 2, 3, 11, 22]), fmt.format_data(3.2), fmt.format_data_short(3.2))
print(FibonacciLocator().tick_values(10, 100).tolist())
print(ax.get_yticks().tolist())
fig.savefig("fibonacci.png")
