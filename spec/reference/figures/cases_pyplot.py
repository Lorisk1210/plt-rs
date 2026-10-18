"""Figures built only through pyplot's functions acting on the current
figure and axes: the harness's figure is pyplot's current figure."""

import matplotlib.pyplot as plt
import numpy as np

from spec_support import case


def _xyz(n=40):
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    return xx, yy, np.sin(xx) * np.cos(yy * 0.8) * np.exp(-(xx**2 + yy**2) / 12.0)


@case("pyplot/line_decorations")
def _(fig):
    x = np.linspace(0.0, 2.0 * np.pi, 60)
    plt.plot(x, np.sin(x), label="sin")
    plt.plot(x, np.cos(x), "r--", label="cos")
    plt.axhline(0.0, color="0.5", linewidth=0.8)
    plt.axvspan(1.0, 2.0, alpha=0.2)
    plt.title("Waves")
    plt.xlabel("x")
    plt.ylabel("y")
    plt.xlim(0.0, 2.0 * np.pi)
    plt.ylim(-1.5, 1.5)
    plt.grid(True)
    plt.legend()
    plt.text(4.0, 1.2, "note")


@case("pyplot/subplot_grid")
def _(fig):
    plt.subplot(2, 2, 1)
    plt.bar([1, 2, 3], [3.0, 1.0, 2.0])
    plt.title("bar")
    plt.subplot(2, 2, 2)
    plt.barh([1, 2, 3], [2.0, 3.0, 1.0])
    plt.subplot(2, 2, 3)
    plt.hist(np.sin(np.linspace(0.0, 20.0, 400)) * np.linspace(0.0, 1.0, 400), bins=12)
    plt.subplot(2, 2, 4)
    plt.stem([1.0, 2.0, 3.0, 4.0], [1.0, 3.0, 2.0, 4.0])
    plt.suptitle("pyplot subplots")
    plt.subplots_adjust(hspace=0.4)


@case("pyplot/image_cmap_colorbar")
def _(fig):
    plt.imshow(_xyz(30)[2])
    plt.hot()
    plt.clim(-0.3, 0.3)
    plt.colorbar()
    plt.title("hot, clim")


@case("pyplot/contour_colorbar_labels")
def _(fig):
    xx, yy, zz = _xyz()
    plt.contourf(xx, yy, zz, levels=8)
    plt.colorbar()
    cs = plt.contour(xx, yy, zz, levels=8, colors="k", linewidths=0.6)
    plt.clabel(cs)


@case("pyplot/subplot2grid_twin")
def _(fig):
    x = np.linspace(0.0, 3.0, 31)
    plt.subplot2grid((3, 3), (0, 0), colspan=3)
    plt.plot(x, np.exp(x))
    plt.yscale("log")
    plt.subplot2grid((3, 3), (1, 0), rowspan=2, colspan=2)
    plt.scatter(x, np.sin(3.0 * x), c=x)
    plt.xticks([0.0, 1.5, 3.0], ["start", "middle", "end"])
    plt.margins(0.2, 0.1)
    plt.subplot2grid((3, 3), (1, 2), rowspan=2)
    plt.step(x[:8], x[:8] ** 2, where="mid")
    plt.box(False)
    plt.twinx()
    plt.plot(x[:8], -x[:8], color="C3")


@case("pyplot/polar_grids")
def _(fig):
    theta = np.linspace(0.0, 2.0 * np.pi, 100)
    plt.polar(theta, 1.0 + 0.5 * np.sin(3.0 * theta))
    plt.rgrids([0.5, 1.0, 1.5], ["a", "b", "c"], angle=45.0)
    plt.thetagrids([0.0, 90.0, 180.0, 270.0])
    plt.title("polar")


@case("pyplot/contour_clim_cmap_after_colorbar")
def _(fig):
    xx, yy, zz = _xyz()
    plt.contourf(xx, yy, zz, levels=8)
    plt.colorbar()
    plt.clim(-0.2, 0.3)
    plt.magma()
