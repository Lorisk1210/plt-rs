"""
A program written for matplotlib, run unchanged on pltrs as matplotlib
(the ``matplotlib`` distribution of the tree) by ``check_dist.py
--install``, in a fresh environment without a real matplotlib, from a
directory of its own, where it writes its figures. Its data is
deterministic. ``arithmetic.png`` draws what depends on how the
platform's compiler fused matplotlib's products (contours of a grid and of
a triangulation, a triangulation's plane coefficients, an image and a
patch through rotated transforms), and ``arithmetic-3d.png`` a
triangulated surface and wind barbs, for ``check_dist.py
--compare-drawings`` to compare with matplotlib's own drawing.
"""

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
import pylab  # noqa: E402
from matplotlib import colors, ticker, tri  # noqa: E402
from matplotlib.patches import Rectangle  # noqa: E402
from matplotlib.transforms import Affine2D  # noqa: E402
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

arithmetic, axes = plt.subplots(2, 2, figsize=(8, 6))
axes[0, 0].contourf(grid_x, grid_y, np.sin(3 * grid_x) * np.cos(2 * grid_y) + z, levels=9,
                    cmap="RdBu")
angles = np.linspace(0, 2 * np.pi, 36, endpoint=False)
radii = np.linspace(0.25, 0.95, 8)
angles = np.repeat(angles[..., np.newaxis], 8, axis=1)
angles[:, 1::2] += np.pi / 36
tx, ty = (radii * np.cos(angles)).ravel(), (radii * np.sin(angles)).ravel()
mesh = tri.Triangulation(tx, ty)
field = np.cos(3 * tx) * np.sin(4 * ty) + 0.3 * tx
axes[0, 1].tricontourf(mesh, field, levels=8)
axes[0, 1].tricontour(mesh, field, levels=8, colors="k", linewidths=0.5)
# Flat: matplotlib's own aarch64 build interpolates Gouraud colours
# otherwise than its x86-64 one, for a reason no compiler flag explains
# (see "aarch64 Linux" in python/DESIGN.md), which pltrs does not follow.
axes[1, 0].tripcolor(mesh, field, shading="flat")
axes[1, 0].tricontour(mesh, tri.LinearTriInterpolator(mesh, field)(tx * 0.9, ty * 0.9),
                      levels=5, colors="w")
rotated = Affine2D().rotate_deg_around(0.5, 0.5, 23) + axes[1, 1].transData
picture = axes[1, 1].imshow(z[::4, ::4], extent=(0, 1, 0, 1), interpolation="bicubic")
picture.set_transform(rotated)
axes[1, 1].add_patch(Rectangle((0.2, 0.2), 0.5, 0.3, fill=False, transform=rotated))
axes[1, 1].set(xlim=(-0.3, 1.3), ylim=(-0.3, 1.3))
arithmetic.savefig("arithmetic.png")

spatial = plt.figure(figsize=(8, 3))
surface_axes = spatial.add_subplot(1, 2, 1, projection="3d")
surface_axes.plot_trisurf(tx, ty, field, cmap="viridis", linewidth=0.2)
barb_axes = spatial.add_subplot(1, 2, 2)
bx, by = np.meshgrid(np.linspace(-1, 1, 6), np.linspace(-1, 1, 5))
barb_axes.barbs(bx, by, 30 * np.cos(bx * 2) + 7, 25 * np.sin(by * 3) - 5, length=6,
                barb_increments=dict(half=10, full=20, flag=100), rounding=False)
spatial.savefig("arithmetic-3d.png")

print(f"matplotlib {matplotlib.__version__}: {len(plt.get_fignums())} figures drawn")
