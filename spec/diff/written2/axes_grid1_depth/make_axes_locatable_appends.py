# make_axes_locatable with append_axes on every side, pad/size as strings and Size objects, sharex/sharey and colorbars.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import make_axes_locatable, axes_size

fig, ax = plt.subplots(figsize=(6, 5), dpi=80)
im = ax.imshow(np.add.outer(np.arange(20), np.arange(30)) % 9)
div = make_axes_locatable(ax)
right = div.append_axes("right", size="5%", pad=0.1)
top = div.append_axes("top", size=0.6, pad=0.2, sharex=ax)
left = div.append_axes("left", size=axes_size.Fraction(0.1, axes_size.AxesX(ax)), pad="2%")
bottom = div.append_axes("bottom", size="15%", pad=0.3, sharex=ax)
cb = fig.colorbar(im, cax=right)
top.plot(np.arange(30), np.arange(30) % 7)
bottom.plot(np.arange(30), np.sin(np.arange(30)))
fig.canvas.draw()
for name, a in [("main", ax), ("right", right), ("top", top), ("left", left), ("bottom", bottom)]:
    print(name, [round(v, 4) for v in a.get_position().bounds])
print(type(div).__name__, len(div.get_horizontal()), len(div.get_vertical()))
print(top.get_shared_x_axes().joined(top, ax), bottom.get_xlim() == ax.get_xlim())
ax.set_xlim(5, 25)
print([round(v, 2) for v in top.get_xlim()], [round(v, 2) for v in bottom.get_xlim()])
try:
    div.append_axes("diagonal", size="5%")
except ValueError as e:
    print("ValueError", str(e)[:60])
fig.set_size_inches(8, 5)
fig.canvas.draw()
print([round(v, 4) for v in ax.get_position().bounds], [round(v, 4) for v in right.get_position().bounds])
fig.savefig("locatable.png")
