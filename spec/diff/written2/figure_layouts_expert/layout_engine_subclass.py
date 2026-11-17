# A custom LayoutEngine subclass that stacks axes in a column with fixed gaps; set_layout_engine, execute counts and positions.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.layout_engine import LayoutEngine


class ColumnLayout(LayoutEngine):
    _adjust_compatible = False
    _colorbar_gridspec = False

    def __init__(self, gap=0.04, margin=0.08, **kw):
        super().__init__(**kw)
        self.gap, self.margin = gap, margin
        self.calls = 0

    def execute(self, fig):
        self.calls += 1
        axes = [a for a in fig.axes if a.get_in_layout()]
        n = len(axes)
        h = (1 - 2 * self.margin - self.gap * (n - 1)) / n
        for k, ax in enumerate(axes):
            ax.set_position([self.margin, 1 - self.margin - (k + 1) * h - k * self.gap, 1 - 2 * self.margin, h])


fig = plt.figure(figsize=(4, 5))
for k in range(3):
    fig.add_subplot(1, 3, k + 1).plot([0, k + 1])
eng = ColumnLayout(gap=0.05)
fig.set_layout_engine(eng)
fig.canvas.draw()
print(type(fig.get_layout_engine()).__name__, eng.calls, [[round(v, 3) for v in a.get_position().bounds] for a in fig.axes])
print(eng.adjust_compatible, eng.colorbar_gridspec, eng.get())
fig.add_subplot(2, 2, 4)
fig.savefig("column.png")
print(eng.calls, len(fig.axes), [round(a.get_position().y0, 3) for a in fig.axes])
try:
    fig.set_layout_engine("constrained")
    print(type(fig.get_layout_engine()).__name__)
except RuntimeError as e:
    print("RuntimeError", str(e)[:60])
fig.set_layout_engine("none")
print(type(fig.get_layout_engine()).__name__)
