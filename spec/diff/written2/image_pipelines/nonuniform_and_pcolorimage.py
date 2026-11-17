# NonUniformImage and PcolorImage (from pcolorfast) with irregular coordinates; get_array, extents and interpolation choices.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.image import NonUniformImage

x = np.array([0, 0.5, 1.5, 3.5, 7.5])
y = np.array([0, 1, 3, 6])
z = np.add.outer(y, x)
fig, axs = plt.subplots(1, 3, figsize=(8, 3), dpi=50)
for ax, interp in zip(axs[:2], ("nearest", "bilinear")):
    im = NonUniformImage(ax, interpolation=interp, extent=(0, 7.5, 0, 6), cmap="viridis")
    im.set_data(x, y, z)
    ax.add_image(im)
    ax.set_xlim(0, 7.5)
    ax.set_ylim(0, 6)
    print(interp, im.get_array().shape, im.get_extent(), im.get_interpolation())
xe = np.array([0, 1, 2, 4, 8])
ye = np.array([0, 2, 3])
pf = axs[2].pcolorfast(xe, ye, np.arange(8).reshape(2, 4), cmap="magma")
fig.canvas.draw()
print(type(pf).__name__, pf.get_array().shape, [round(v, 2) for v in axs[2].get_xlim()])
pf2 = axs[2].pcolorfast((0, 8), (3, 6), np.arange(6).reshape(2, 3))
print(type(pf2).__name__, [round(v, 2) for v in pf2.get_extent()])
X, Y = np.meshgrid(xe, ye)
pf3 = axs[2].pcolorfast(X, Y, np.arange(8).reshape(2, 4))
print(type(pf3).__name__)
fig.savefig("nonuniform.png")
try:
    NonUniformImage(axs[0]).set_data(x, y, z.T)
except (ValueError, TypeError) as e:
    print(type(e).__name__, str(e)[:60])
