"""``mpl_toolkits.axisartist``: the direction, ticks, tick labels, label
and line of axis artists, curvilinear grids (``grid_finder``,
``angle_helper``, ``grid_helper_curvelinear``) and floating axes
(``floating_axes``), after matplotlib's axisartist gallery."""

import numpy as np
from matplotlib.projections import PolarAxes
from matplotlib.transforms import Affine2D
from mpl_toolkits import axisartist
from mpl_toolkits.axisartist import angle_helper, floating_axes, grid_finder
from mpl_toolkits.axisartist.grid_helper_curvelinear import GridHelperCurveLinear

from spec_support import case, rng


def _ticked(fig, pos):
    ax = fig.add_subplot(pos, axes_class=axisartist.Axes)
    ax.set_yticks([0.2, 0.8])
    ax.set_xticks([0.2, 0.8])
    return ax


def _polar_grid_helper(grid_locator2=None):
    """``demo_curvelinear_grid``'s polar grid in degrees in a rectangular box."""
    return GridHelperCurveLinear(
        Affine2D().scale(np.pi / 180.0, 1.0) + PolarAxes.PolarTransform(),
        extreme_finder=angle_helper.ExtremeFinderCycle(
            20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None, lat_minmax=(0, np.inf)),
        grid_locator1=angle_helper.LocatorDMS(12),
        grid_locator2=grid_locator2,
        tick_formatter1=angle_helper.FormatterDMS(),
    )


# --- direction and styling of axis artists


@case("axisartist/simple_axis_direction01", figsize=(4, 2.5))
def _(fig):
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    fig.subplots_adjust(right=0.8)
    ax.axis["left"].major_ticklabels.set_axis_direction("top")
    ax.axis["left"].label.set_text("Left label")
    ax.axis["right"].label.set_visible(True)
    ax.axis["right"].label.set_text("Right label")
    ax.axis["right"].label.set_axis_direction("left")


@case("axisartist/simple_axis_direction03", figsize=(5, 2))
def _(fig):
    fig.subplots_adjust(wspace=0.4, bottom=0.3)
    ax1 = _ticked(fig, 121)
    ax1.set_xlabel("ax1 X-label")
    ax1.set_ylabel("ax1 Y-label")
    ax1.axis[:].invert_ticklabel_direction()
    ax2 = _ticked(fig, 122)
    ax2.set_xlabel("ax2 X-label")
    ax2.set_ylabel("ax2 Y-label")
    ax2.axis[:].major_ticks.set_tick_out(False)


@case("axisartist/ticklabel_alignment", figsize=(3, 5))
def _(fig):
    fig.subplots_adjust(left=0.5, hspace=0.7)
    axes = []
    for pos in [311, 312, 313]:
        ax = fig.add_subplot(pos, axes_class=axisartist.Axes)
        ax.set_yticks([0.2, 0.8], labels=["short", "loooong"])
        ax.set_xticks([0.2, 0.8], labels=[r"$\frac{1}{2}\pi$", r"$\pi$"])
        axes.append(ax)
    axes[0].set_ylabel("ha=right")
    axes[0].set_xlabel("va=baseline")
    axes[1].axis["left"].major_ticklabels.set_ha("center")
    axes[1].axis["bottom"].major_ticklabels.set_va("top")
    axes[1].set_ylabel("ha=center")
    axes[1].set_xlabel("va=top")
    axes[2].axis["left"].major_ticklabels.set_ha("left")
    axes[2].axis["bottom"].major_ticklabels.set_va("bottom")
    axes[2].set_ylabel("ha=left")
    axes[2].set_xlabel("va=bottom")


@case("axisartist/ticklabel_direction", figsize=(6, 3))
def _(fig):
    fig.subplots_adjust(bottom=0.2)
    ax = _ticked(fig, 131)
    for axis in ax.axis.values():
        axis.major_ticks.set_tick_out(True)
    ax = _ticked(fig, 132)
    ax.axis["left"].set_axis_direction("right")
    ax.axis["bottom"].set_axis_direction("top")
    ax.axis["right"].set_axis_direction("left")
    ax.axis["top"].set_axis_direction("bottom")
    ax = _ticked(fig, 133)
    ax.axis["left"].set_axis_direction("right")
    ax.axis[:].major_ticks.set_tick_out(True)
    ax.axis["left"].label.set_text("Long Label Left")
    ax.axis["bottom"].label.set_text("Label Bottom")
    ax.axis["right"].label.set_text("Long Label Right")
    ax.axis["right"].label.set_visible(True)
    ax.axis["left"].label.set_pad(0)
    ax.axis["bottom"].label.set_pad(10)


@case("axisartist/axis_styling", figsize=(6, 4))
def _(fig):
    fig.subplots_adjust(left=0.15, right=0.85, bottom=0.2)
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    x = np.linspace(0.0, 10.0, 60)
    ax.plot(x, 40.0 * np.sin(x) + 50.0)
    ax.minorticks_on()
    ax.set_xlabel("time [s]")
    ax.set_ylabel("signal")
    ax.grid(True, which="both", axis="x", color="0.8", linestyle=":")
    bottom = ax.axis["bottom"]
    bottom.major_ticks.set_color("tab:red")
    bottom.major_ticks.set_ticksize(8)
    bottom.major_ticks.set_tick_direction("inout")
    bottom.minor_ticks.set_color("tab:blue")
    bottom.minor_ticks.set_tick_out(True)
    bottom.major_ticklabels.set_color("tab:red")
    bottom.major_ticklabels.set_rotation(30)
    bottom.major_ticklabels.set_pad(6)
    bottom.label.set_color("tab:green")
    bottom.label.set_pad(12)
    bottom.line.set_color("tab:purple")
    bottom.line.set_linewidth(2)
    left = ax.axis["left"]
    left.major_ticks.set_markeredgewidth(2)
    left.major_ticks.set_ticksize(6)
    left.major_ticklabels.set_size(8)
    left.minor_ticks.set_visible(False)
    left.label.set_axis_direction("right")
    right = ax.axis["right"]
    right.toggle(all=True)
    right.label.set_text("right label")
    right.invert_ticklabel_direction()
    ax.axis["top"].toggle(all=False, ticks=True)
    ax.axis["top"].major_ticks.set_tick_out(True)
    ax.axis["top"].line.set_visible(False)


@case("axisartist/axis_direction", figsize=(8, 4))
def _(fig):
    fig.subplots_adjust(left=0.04, right=0.98, bottom=0.06, top=0.92, wspace=0.35, hspace=0.45)
    for row, (nth, value, name, text) in enumerate(
            [(0, 30, "lat", r"$\theta = 30^{\circ}$"), (1, 6, "lon", r"$r = 6$")]):
        for i, direction in enumerate(["bottom", "left", "top", "right"]):
            ax = fig.add_subplot(
                2, 4, 4 * row + i + 1, axes_class=axisartist.Axes,
                grid_helper=_polar_grid_helper(grid_finder.MaxNLocator(5)),
                aspect=1, xlim=(-5, 12), ylim=(-5, 10))
            ax.axis[:].toggle(ticklabels=False)
            ax.grid(color=".9")
            ax.axis[name] = axis = ax.new_floating_axis(nth, value)
            axis.label.set_text(text)
            axis.label.set_visible(True)
            axis.set_axis_direction(direction)
            ax.set(title=direction)


@case("axisartist/simple_axis_pad", figsize=(9, 3))
def _(fig):
    fig.subplots_adjust(left=0.01, right=0.99, bottom=0.01, top=0.99, wspace=0.01, hspace=0.01)
    adjustments = [
        ("default", lambda axis: None),
        ("ticklabels.set_pad(10)", lambda axis: axis.major_ticklabels.set_pad(10)),
        ("label.set_pad(20)", lambda axis: axis.label.set_pad(20)),
        ("ticks.set_tick_out(True)", lambda axis: axis.major_ticks.set_tick_out(True)),
    ]
    for i, (note, adjust) in enumerate(adjustments):
        ax = fig.add_subplot(1, 4, i + 1, axes_class=axisartist.Axes,
                             grid_helper=_polar_grid_helper(grid_finder.MaxNLocator(5)))
        ax.axis[:].set_visible(False)
        ax.set_aspect(1.0)
        ax.set_xlim(-5, 12)
        ax.set_ylim(-5, 10)
        ax.axis["lat"] = axis = ax.new_floating_axis(0, 30)
        axis.label.set_text(r"$\theta = 30^{\circ}$")
        axis.label.set_visible(True)
        adjust(axis)
        ax.annotate(note, (0.5, 1), (5, -5), xycoords="axes fraction",
                    textcoords="offset points", va="top", ha="center")


# --- curvilinear grids


@case("axisartist/curvelinear_grid", figsize=(7, 4))
def _(fig):
    def tr(x, y):
        return x, y - x

    def inv_tr(x, y):
        return x, y + x

    ax1 = fig.add_subplot(1, 2, 1, axes_class=axisartist.Axes,
                          grid_helper=GridHelperCurveLinear((tr, inv_tr)))
    xx, yy = tr(np.array([3, 6]), np.array([5, 10]))
    ax1.plot(xx, yy)
    ax1.set_aspect(1)
    ax1.set_xlim(0, 10)
    ax1.set_ylim(0, 10)
    ax1.axis["t"] = ax1.new_floating_axis(0, 3)
    ax1.axis["t2"] = ax1.new_floating_axis(1, 7)
    ax1.grid(True, zorder=0)

    polar = Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform()
    ax2 = fig.add_subplot(1, 2, 2, axes_class=axisartist.HostAxes,
                          grid_helper=_polar_grid_helper())
    ax2.axis["right"].major_ticklabels.set_visible(True)
    ax2.axis["top"].major_ticklabels.set_visible(True)
    ax2.axis["right"].get_helper().nth_coord_ticks = 0
    ax2.axis["bottom"].get_helper().nth_coord_ticks = 1
    ax2.set_aspect(1)
    ax2.set_xlim(-5, 12)
    ax2.set_ylim(-5, 10)
    ax2.grid(True, zorder=0)
    aux = ax2.get_aux_axes(polar)
    aux.plot(np.linspace(0, 30, 51), np.linspace(10, 10, 51), linewidth=2)
    aux.pcolor(np.linspace(0, 90, 4), np.linspace(0, 10, 4), np.arange(9).reshape((3, 3)))
    aux.contour(np.linspace(0, 90, 4), np.linspace(0, 10, 4), np.arange(16).reshape((4, 4)),
                colors="k")


@case("axisartist/curvelinear_grid2", figsize=(7, 4))
def _(fig):
    def tr(x, y):
        return np.sign(x) * abs(x) ** 0.5, y

    def inv_tr(x, y):
        return np.sign(x) * x ** 2, y

    grid_helper = GridHelperCurveLinear(
        (tr, inv_tr), extreme_finder=grid_finder.ExtremeFinderSimple(20, 20),
        grid_locator1=grid_finder.MaxNLocator(nbins=6),
        grid_locator2=grid_finder.MaxNLocator(nbins=6))
    ax = fig.add_subplot(axes_class=axisartist.Axes, grid_helper=grid_helper)
    ax.imshow(np.arange(25).reshape(5, 5), vmax=50, cmap="gray_r", origin="lower")


# --- floating axes


@case("axisartist/floating_axes", figsize=(8, 4))
def _(fig):
    fig.subplots_adjust(wspace=0.3, left=0.05, right=0.95)
    data = rng(19680801)

    tr1 = Affine2D().scale(2, 1).rotate_deg(30)
    helper1 = floating_axes.GridHelperCurveLinear(
        tr1, extremes=(-0.5, 3.5, 0, 4),
        grid_locator1=grid_finder.MaxNLocator(nbins=4),
        grid_locator2=grid_finder.MaxNLocator(nbins=4))
    ax1 = fig.add_subplot(131, axes_class=floating_axes.FloatingAxes, grid_helper=helper1)
    ax1.grid()
    aux1 = ax1.get_aux_axes(tr1)
    aux1.bar([0, 1, 2, 3], [3, 2, 1, 3])

    tr2 = PolarAxes.PolarTransform()
    angle_ticks = [(0, r"$0$"), (0.25 * np.pi, r"$\frac{1}{4}\pi$"),
                   (0.5 * np.pi, r"$\frac{1}{2}\pi$")]
    helper2 = floating_axes.GridHelperCurveLinear(
        tr2, extremes=(0.5 * np.pi, 0, 2, 1),
        grid_locator1=grid_finder.FixedLocator([v for v, s in angle_ticks]),
        grid_locator2=grid_finder.MaxNLocator(2),
        tick_formatter1=grid_finder.DictFormatter(dict(angle_ticks)),
        tick_formatter2=None)
    ax2 = fig.add_subplot(132, axes_class=floating_axes.FloatingAxes, grid_helper=helper2)
    ax2.grid()
    aux2 = ax2.get_aux_axes(tr2)
    aux2.patch = ax2.patch
    ax2.patch.zorder = 0.9
    aux2.scatter(data.uniform(10) * 0.5 * np.pi, data.uniform(10) + 1.0)

    tr3 = Affine2D().translate(-95, 0) + Affine2D().scale(np.pi / 180.0, 1.0) \
        + PolarAxes.PolarTransform()
    helper3 = floating_axes.GridHelperCurveLinear(
        tr3, extremes=(8.0 * 15, 14.0 * 15, 0, 14000),
        grid_locator1=angle_helper.LocatorHMS(4),
        tick_formatter1=angle_helper.FormatterHMS(),
        grid_locator2=grid_finder.MaxNLocator(3),
        tick_formatter2=None)
    ax3 = fig.add_subplot(133, axes_class=floating_axes.FloatingAxes, grid_helper=helper3)
    ax3.axis["left"].set_axis_direction("bottom")
    ax3.axis["right"].set_axis_direction("top")
    ax3.axis["bottom"].set_visible(False)
    ax3.axis["top"].set_axis_direction("bottom")
    ax3.axis["top"].toggle(ticklabels=True, label=True)
    ax3.axis["top"].major_ticklabels.set_axis_direction("top")
    ax3.axis["top"].label.set_axis_direction("top")
    ax3.axis["left"].label.set_text(r"cz [km$^{-1}$]")
    ax3.axis["top"].label.set_text(r"$\alpha_{1950}$")
    ax3.grid()
    aux3 = ax3.get_aux_axes(tr3)
    aux3.patch = ax3.patch
    ax3.patch.zorder = 0.9
    aux3.scatter((8 + data.uniform(10) * (14 - 8)) * 15.0, data.uniform(10) * 14000.0)


@case("axisartist/floating_axis", figsize=(5, 5))
def _(fig):
    ax = fig.add_subplot(axes_class=axisartist.HostAxes, grid_helper=_polar_grid_helper())
    ax.axis["lat"] = axis = ax.new_floating_axis(0, 60)
    axis.label.set_text(r"$\theta = 60^{\circ}$")
    axis.label.set_visible(True)
    ax.axis["lon"] = axis = ax.new_floating_axis(1, 6)
    axis.label.set_text(r"$r = 6$")
    ax.set_aspect(1.0)
    ax.set_xlim(-5, 12)
    ax.set_ylim(-5, 10)
    ax.grid(True)
