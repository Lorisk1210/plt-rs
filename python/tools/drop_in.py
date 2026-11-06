"""
A program written for matplotlib, run unchanged on pltrs as matplotlib
(the ``matplotlib`` distribution of the tree) by ``check_dist.py
--install``, in a fresh environment without a real matplotlib, from a
directory of its own, where it writes its figures. Its data is
deterministic.
"""

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
import pylab  # noqa: E402
from matplotlib import colors, ticker  # noqa: E402
from mpl_toolkits.axes_grid1 import make_axes_locatable  # noqa: E402

x = np.linspace(0, 2 * np.pi, 200)
grid_x, grid_y = np.meshgrid(np.linspace(-2, 2, 60), np.linspace(-2, 2, 60))
z = np.exp(-(grid_x ** 2 + grid_y ** 2)) + 1e-3

fig, axes = plt.subplots(2, 2, figsize=(8, 6), layout="constrained")
axes[0, 0].plot(x, np.sin(x), label="sin")
axes[0, 0].plot(x, np.cos(x), "--", label="cos")
axes[0, 0].legend()
axes[0, 0].set_title(r"$\sin x$ and $\cos x$")
axes[0, 1].hist(np.sin(np.arange(500) * 1.7) * 3, bins=20, color="C2")
image = axes[1, 0].imshow(z, cmap="viridis", norm=colors.LogNorm())
divider = make_axes_locatable(axes[1, 0])
fig.colorbar(image, cax=divider.append_axes("right", size="5%", pad=0.05))
axes[1, 1].bar(["a", "b", "c"], [0.2, 0.5, 0.3])
axes[1, 1].yaxis.set_major_formatter(ticker.PercentFormatter(1.0))
for suffix in ("png", "pdf", "svg"):
    fig.savefig(f"figure.{suffix}")

surface = plt.figure()
ax = surface.add_subplot(projection="3d")
ax.plot_surface(grid_x, grid_y, z, cmap="coolwarm")
surface.savefig("surface.png")

pylab.figure()
pylab.plot([1, 2, 3], [3, 1, 2], "o-")
pylab.savefig("pylab.png")

print(f"matplotlib {matplotlib.__version__}: {len(plt.get_fignums())} figures drawn")
