# Axes with a fixed physical size (inches) using Divider and Size, independent of figure size.
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import Divider, Size

fig = plt.figure(figsize=(6, 4))
h = [Size.Fixed(1.0), Size.Fixed(4.0)]
v = [Size.Fixed(0.7), Size.Fixed(2.5)]
divider = Divider(fig, (0, 0, 1, 1), h, v, aspect=False)
ax = fig.add_axes(divider.get_position(), axes_locator=divider.new_locator(nx=1, ny=1))
ax.plot([1, 2, 3], [1, 4, 2])
ax.set_title("4 x 2.5 inch axes")
fig.savefig("fixed_size.png")
print([round(v, 4) for v in ax.get_position().bounds])
