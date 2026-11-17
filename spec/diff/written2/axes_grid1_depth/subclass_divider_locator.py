# A custom axes locator (callable class) and an AxesLocator from a Divider, used with set_axes_locator and constrained layout off.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Bbox, TransformedBbox
from mpl_toolkits.axes_grid1 import Divider, Size


class CornerLocator:
    """Place an axes in a corner of a parent axes, sized in inches."""

    def __init__(self, parent, w, h, corner="ne"):
        self.parent, self.w, self.h, self.corner = parent, w, h, corner
        self.calls = 0

    def __call__(self, ax, renderer):
        self.calls += 1
        fig = ax.get_figure(root=True)
        pb = self.parent.get_position()
        w = self.w / fig.get_figwidth()
        h = self.h / fig.get_figheight()
        x0 = pb.x1 - w if "e" in self.corner else pb.x0
        y0 = pb.y1 - h if "n" in self.corner else pb.y0
        return Bbox.from_bounds(x0, y0, w, h)


fig, ax = plt.subplots(figsize=(5, 4))
locs = {c: CornerLocator(ax, 1.0, 0.6, c) for c in ("ne", "sw")}
small = {c: fig.add_axes([0, 0, 1, 1], axes_locator=l) for c, l in locs.items()}
fig.canvas.draw()
for c, a in small.items():
    print(c, [round(v, 4) for v in a.get_position().bounds], locs[c].calls > 0)
fig.set_size_inches(7, 3)
fig.canvas.draw()
for c, a in small.items():
    print(c, [round(v, 4) for v in a.get_position().bounds])
div = Divider(fig, (0.05, 0.05, 0.4, 0.3), [Size.Fixed(1), Size.Fixed(0.5)], [Size.Fixed(0.7)], aspect=False)
loc = div.new_locator(nx=0, nx1=2, ny=0)
extra = fig.add_axes([0, 0, 1, 1], axes_locator=loc)
fig.canvas.draw()
print([round(v, 4) for v in extra.get_position().bounds], type(loc).__name__)
small["ne"].set_axes_locator(None)
fig.canvas.draw()
print([round(v, 4) for v in small["ne"].get_position().bounds], small["ne"].get_axes_locator())
fig.savefig("locators.png")
