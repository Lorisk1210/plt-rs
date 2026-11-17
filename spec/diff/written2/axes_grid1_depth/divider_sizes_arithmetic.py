# axes_size arithmetic (Fixed, Scaled, AxesX, Fraction, sums and products) inside a Divider; resulting axes positions.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import Divider, Size

fig = plt.figure(figsize=(6, 4), dpi=100)
rect = (0.1, 0.1, 0.8, 0.8)
main_w = Size.Scaled(2)
side_w = Size.Fixed(0.8) + Size.Fixed(0.2)
gap = Size.Fixed(0.15) * 2
h_sizes = [main_w, gap, side_w, gap, Size.Scaled(1)]
v_sizes = [Size.Fixed(0.5), Size.Scaled(1), Size.Fraction(0.25, Size.Fixed(2))]
div = Divider(fig, rect, h_sizes, v_sizes, aspect=False)
axes = []
for nx, ny in [(0, 1), (2, 1), (4, 1), (0, 0), (0, 2)]:
    ax = fig.add_axes(rect, axes_locator=div.new_locator(nx=nx, ny=ny))
    axes.append(ax)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for s in h_sizes + v_sizes:
    print(type(s).__name__, s.get_size(r))
for ax in axes:
    print([round(v, 4) for v in ax.get_position().bounds])
div.set_aspect(True)
div.set_anchor("NE")
fig.canvas.draw()
print([round(v, 4) for v in axes[0].get_position().bounds], div.get_anchor(), div.get_aspect())
ax_x = Size.AxesX(axes[0], aspect=0.5)
print(ax_x.get_size(r))
print(div.get_horizontal_sizes(r).tolist(), div.get_vertical_sizes(r).tolist())
fig.savefig("divider.png")
