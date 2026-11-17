# RGBAxes showing three channels plus the composite, imshow_rgb with each channel normalised, and their positions.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1.axes_rgb import RGBAxes, make_rgb_axes

y, x = np.mgrid[0:1:30j, 0:1:40j]
r, g, b = x, y, 1 - x * y
fig = plt.figure(figsize=(5, 4), dpi=60)
rgb = RGBAxes(fig, [0.1, 0.1, 0.8, 0.8], pad=0.05)
ims = rgb.imshow_rgb(r, g, b, origin="lower")
fig.canvas.draw()
print(len(ims), [type(i).__name__ for i in ims])
print([round(v, 3) for v in rgb.RGB.get_position().bounds], [round(v, 3) for v in rgb.R.get_position().bounds])
print([ims[k].get_array().shape for k in range(4)])
print(np.round(ims[0].get_array()[0, :3], 3).tolist())
fig.savefig("rgbaxes.png")
fig2 = plt.figure(figsize=(5, 4), dpi=60)
ax = fig2.add_subplot(111)
ax_r, ax_g, ax_b = make_rgb_axes(ax, pad=0.02)
fig2.canvas.draw()
print([round(v, 3) for v in ax.get_position().bounds], [round(v, 3) for v in ax_b.get_position().bounds])
print(len(fig2.axes))
fig2.savefig("rgbaxes2.png")
try:
    rgb.imshow_rgb(r, g[:5], b)
except ValueError as e:
    print("ValueError", str(e)[:60])
rgb.RGB.set_xlim(5, 20)
fig.canvas.draw()
print([round(v, 2) for v in rgb.R.get_xlim()], [round(v, 2) for v in rgb.B.get_xlim()])
