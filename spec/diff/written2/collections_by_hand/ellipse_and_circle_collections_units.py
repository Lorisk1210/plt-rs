# EllipseCollection with units xy/x/y/inches/points/dots/width, CircleCollection sizes; extents as the figure is resized.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import EllipseCollection, CircleCollection

fig, ax = plt.subplots(figsize=(4, 4), dpi=72)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
offsets = np.c_[np.arange(1, 8), np.full(7, 5)]
colls = {}
for k, units in enumerate(("xy", "x", "y", "inches", "points", "dots", "width")):
    ec = EllipseCollection([0.8], [0.4], [30], units=units, offsets=offsets[k:k + 1], offset_transform=ax.transData,
                           facecolors=f"C{k}")
    ax.add_collection(ec)
    colls[units] = ec
cc = CircleCollection([20, 80, 320], offsets=[(2, 2), (5, 2), (8, 2)], offset_transform=ax.transData)
ax.add_collection(cc)
r = fig.canvas.get_renderer()
for size in [(4, 4), (8, 4)]:
    fig.set_size_inches(*size)
    fig.canvas.draw()
    print(size, {u: [round(v, 1) for v in c.get_window_extent(r).bounds][2:] for u, c in colls.items()})
    print(size, [round(v, 1) for v in cc.get_window_extent(r).bounds])
print(colls["xy"].get_widths().tolist(), colls["xy"].get_heights().tolist(), colls["xy"].get_angles().tolist())
colls["xy"].set_widths([2])
colls["xy"].set_angles([90])
fig.canvas.draw()
print(colls["xy"].get_widths().tolist(), [round(v, 1) for v in colls["xy"].get_window_extent(r).bounds][2:])
print(cc.get_sizes().tolist(), type(cc).__mro__[1].__name__)
fig.savefig("ellipses.png")
