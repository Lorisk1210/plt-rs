# set_axes_locator with InsetPosition-like callables and an axes_divider locator; positions after resize and with constrained layout.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Bbox


class RelativeTo:
    def __init__(self, parent, bounds):
        self.parent, self.bounds = parent, bounds

    def __call__(self, ax, renderer):
        p = self.parent.get_position()
        x, y, w, h = self.bounds
        return Bbox.from_bounds(p.x0 + x * p.width, p.y0 + y * p.height, w * p.width, h * p.height)


fig, ax = plt.subplots(figsize=(5, 4), layout="constrained")
ax.set_title("parent")
ins = fig.add_axes([0, 0, 1, 1])
ins.set_axes_locator(RelativeTo(ax, (0.6, 0.6, 0.35, 0.35)))
ins2 = ax.inset_axes([0.05, 0.05, 0.3, 0.3])
fig.canvas.draw()
print([round(v, 4) for v in ax.get_position().bounds], [round(v, 4) for v in ins.get_position().bounds],
      [round(v, 4) for v in ins2.get_position().bounds])
print(type(ins2.get_axes_locator()).__name__, ins.get_in_layout())
fig.set_size_inches(8, 3)
fig.canvas.draw()
print([round(v, 4) for v in ins.get_position().bounds], [round(v, 4) for v in ins2.get_position().bounds])
ax.set_position([0.1, 0.1, 0.5, 0.5])
fig.set_layout_engine("none")
fig.canvas.draw()
print([round(v, 4) for v in ins.get_position().bounds])
ins.set_axes_locator(None)
fig.canvas.draw()
print([round(v, 4) for v in ins.get_position().bounds])
ind = ax.indicate_inset_zoom(ins2)
print(type(ind).__name__, len(ind.connectors))
fig.savefig("locators.png")
