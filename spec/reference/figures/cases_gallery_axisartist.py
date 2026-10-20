"""matplotlib's gallery sections "axisartist" and "Widgets"
(``galleries/examples/{axisartist,widgets}``), one case per figure
(``<example>_<n>`` for examples drawing several).

Widgets are drawn as their figures first appear, before any interaction:
callbacks only run when a widget triggers them while the figure is built
(``textbox``'s ``set_val``), event connections are left out, and so is
the mouse move ``annotated_cursor`` simulates, whose cursor lines and
text are animated and never saved. The cursor of that example is a plain
``Cursor`` plus its hidden text, and ``multicursor``'s cursor spans the
axes of one figure at a time. Random data comes from ``spec_support.rng``
seeded with the example's seed.

Covered by ``cases_axisartist.py``: ``simple_axis_direction01``,
``simple_axis_direction03``, ``demo_ticklabel_alignment``,
``demo_ticklabel_direction``, ``demo_curvelinear_grid``,
``demo_curvelinear_grid2``, ``demo_floating_axes`` and
``demo_floating_axis``. Left out: ``menu``, whose items are custom
artists appended to ``fig.artists`` (each drawing its label a second
time).
"""

import numpy as np
from matplotlib import widgets
from matplotlib.backend_tools import Cursors
from matplotlib.projections import PolarAxes
from matplotlib.transforms import Affine2D
from mpl_toolkits import axisartist
from mpl_toolkits.axes_grid1 import host_subplot
from mpl_toolkits.axisartist import angle_helper, grid_finder
from mpl_toolkits.axisartist.grid_helper_curvelinear import GridHelperCurveLinear
from mpl_toolkits.axisartist.parasite_axes import HostAxes

from spec_support import case, rng

GROUP = "gallery_axisartist"
SEED = 19680801


def gallery(name, **options):
    """Registers a case of this group."""
    return case(f"{GROUP}/{name}", **options)


def _noop(*args):
    """A selector callback that does nothing."""


# --- axisartist: axis directions ---------------------------------------------------


def _arrow_axis(fig, pos):
    """``axis_direction``'s ``setup_axes``: only a vertical axis line
    with an arrow, through x = 0.5."""
    ax = fig.add_subplot(pos, axes_class=axisartist.Axes)
    ax.set_ylim(-0.1, 1.5)
    ax.set_yticks([0, 1])
    ax.axis[:].set_visible(False)
    ax.axis["x"] = ax.new_floating_axis(1, 0.5)
    ax.axis["x"].set_axisline_style("->", size=1.5)
    return ax


@gallery("axis_direction", figsize=(10, 4),
         rc={"axes.titlesize": "medium", "axes.titley": 1.1})
def _(fig):
    fig.subplots_adjust(bottom=0.1, top=0.9, left=0.05, right=0.95)
    ax1 = _arrow_axis(fig, 251)
    ax1.axis["x"].set_axis_direction("left")
    for pos, direction in [(252, "+"), (253, "-")]:
        ax = _arrow_axis(fig, pos)
        ax.axis["x"].label.set_text("Label")
        ax.axis["x"].toggle(ticklabels=False)
        ax.axis["x"].set_axislabel_direction(direction)
        ax.set_title(f"label direction=${direction}$")
    for pos, direction in [(254, "+"), (255, "-")]:
        ax = _arrow_axis(fig, pos)
        ax.axis["x"].set_ticklabel_direction(direction)
        ax.set_title(f"ticklabel direction=${direction}$")
    ax7 = _arrow_axis(fig, 257)
    ax7.axis["x"].label.set_text("rotation=10")
    ax7.axis["x"].label.set_rotation(10)
    ax7.axis["x"].toggle(ticklabels=False)
    ax8 = _arrow_axis(fig, 258)
    ax8.axis["x"].set_axislabel_direction("-")
    ax8.axis["x"].label.set_text("rotation=10")
    ax8.axis["x"].label.set_rotation(10)
    ax8.axis["x"].toggle(ticklabels=False)


def _polar_box(fig, rect):
    """``demo_axis_direction``'s and ``simple_axis_pad``'s
    ``setup_axes``: a polar grid in degrees in a rectangular box."""
    grid_helper = GridHelperCurveLinear(
        Affine2D().scale(np.pi / 180., 1.) + PolarAxes.PolarTransform(),
        extreme_finder=angle_helper.ExtremeFinderCycle(
            20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None, lat_minmax=(0, np.inf)),
        grid_locator1=angle_helper.LocatorDMS(12),
        grid_locator2=grid_finder.MaxNLocator(5),
        tick_formatter1=angle_helper.FormatterDMS(),
    )
    return fig.add_subplot(rect, axes_class=axisartist.Axes, grid_helper=grid_helper,
                           aspect=1, xlim=(-5, 12), ylim=(-5, 10))


def _floating_lat(ax):
    """``add_floating_axis1``: the labelled axis line θ = 30°."""
    ax.axis["lat"] = axis = ax.new_floating_axis(0, 30)
    axis.label.set_text(r"$\theta = 30^{\circ}$")
    axis.label.set_visible(True)
    return axis


def _floating_lon(ax):
    """``add_floating_axis2``: the labelled axis line r = 6."""
    ax.axis["lon"] = axis = ax.new_floating_axis(1, 6)
    axis.label.set_text(r"$r = 6$")
    axis.label.set_visible(True)
    return axis


@gallery("demo_axis_direction", figsize=(8, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    for row, add_floating_axis in enumerate([_floating_lat, _floating_lon]):
        for i, direction in enumerate(["bottom", "left", "top", "right"]):
            ax = _polar_box(fig, 241 + 4 * row + i)
            ax.axis[:].toggle(ticklabels=False)
            ax.grid(color=".9")
            axis = add_floating_axis(ax)
            axis.set_axis_direction(direction)
            ax.set(title=direction)


@gallery("simple_axis_pad", figsize=(9, 3))
def _(fig):
    fig.subplots_adjust(left=0.01, right=0.99, bottom=0.01, top=0.99, wspace=0.01, hspace=0.01)
    adjustments = [
        ("default", lambda axis: None),
        ("ticklabels.set_pad(10)", lambda axis: axis.major_ticklabels.set_pad(10)),
        ("label.set_pad(20)", lambda axis: axis.label.set_pad(20)),
        ('ticks.set_tick_direction("in")', lambda axis: axis.major_ticks.set_tick_direction("in")),
        ('ticks.set_tick_direction("out")',
         lambda axis: axis.major_ticks.set_tick_direction("out")),
        ('ticks.set_tick_direction("inout")',
         lambda axis: axis.major_ticks.set_tick_direction("inout")),
    ]
    for i, (note, adjust) in enumerate(adjustments):
        ax = _polar_box(fig, 231 + i)
        ax.axis[:].set_visible(False)
        adjust(_floating_lat(ax))
        ax.annotate(note, (0.5, 1), (5, -5), xycoords="axes fraction",
                    textcoords="offset points", va="top", ha="center")


# --- axisartist: axis lines ----------------------------------------------------------


@gallery("demo_axisline_style")
def _(fig):
    ax = fig.add_subplot(axes_class=axisartist.axislines.AxesZero)
    for direction in ["xzero", "yzero"]:
        ax.axis[direction].set_axisline_style("-|>")
        ax.axis[direction].set_visible(True)
    for direction in ["left", "right", "bottom", "top"]:
        ax.axis[direction].set_visible(False)
    x = np.linspace(-0.5, 1., 100)
    ax.plot(x, np.sin(x * np.pi))


@gallery("simple_axisartist1", figsize=(6, 3))
def _(fig):
    fig.set_layout_engine("constrained")
    gs = fig.add_gridspec(1, 2)
    ax0 = fig.add_subplot(gs[0, 0], axes_class=axisartist.Axes)
    ax0.axis["y=0"] = ax0.new_floating_axis(nth_coord=0, value=0, axis_direction="bottom")
    ax0.axis["y=0"].toggle(all=True)
    ax0.axis["y=0"].label.set_text("y = 0")
    ax0.axis["bottom", "top", "right"].set_visible(False)
    ax1 = fig.add_subplot(gs[0, 1], axes_class=axisartist.axislines.AxesZero)
    ax1.axis["xzero"].set_visible(True)
    ax1.axis["xzero"].label.set_text("Axis Zero")
    ax1.axis["bottom", "top", "right"].set_visible(False)
    x = np.arange(0, 2 * np.pi, 0.01)
    ax0.plot(x, np.sin(x))
    ax1.plot(x, np.sin(x))


@gallery("simple_axisline")
def _(fig):
    fig.subplots_adjust(right=0.85)
    ax = fig.add_subplot(axes_class=axisartist.axislines.AxesZero)
    ax.axis["right"].set_visible(False)
    ax.axis["top"].set_visible(False)
    ax.axis["xzero"].set_visible(True)
    ax.axis["xzero"].label.set_text("Axis Zero")
    ax.set_ylim(-2, 4)
    ax.set_xlabel("Label X")
    ax.set_ylabel("Label Y")
    ax.axis["right2"] = ax.new_fixed_axis(loc="right", offset=(20, 0))
    ax.axis["right2"].label.set_text("Label Y2")
    ax.plot([-2, 3, 2])


@gallery("simple_axisline3", figsize=(3, 3))
def _(fig):
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.axis["right"].set_visible(False)
    ax.axis["top"].set_visible(False)


# --- axisartist: parasite axes -------------------------------------------------------


def _three_datasets(host, par1, par2):
    """The plots, limits, labels and legend shared by both parasite axes
    demos; returns the three lines."""
    p1, = host.plot([0, 1, 2], [0, 1, 2], label="Density")
    p2, = par1.plot([0, 1, 2], [0, 3, 2], label="Temperature")
    p3, = par2.plot([0, 1, 2], [50, 30, 15], label="Velocity")
    host.set(xlim=(0, 2), ylim=(0, 2), xlabel="Distance", ylabel="Density")
    par1.set(ylim=(0, 4), ylabel="Temperature")
    par2.set(ylim=(1, 65), ylabel="Velocity")
    host.legend()
    return p1, p2, p3


@gallery("demo_parasite_axes")
def _(fig):
    host = fig.add_axes((0.15, 0.1, 0.65, 0.8), axes_class=HostAxes)
    par1 = host.get_aux_axes(viewlim_mode=None, sharex=host)
    par2 = host.get_aux_axes(viewlim_mode=None, sharex=host)
    host.axis["right"].set_visible(False)
    par1.axis["right"].set_visible(True)
    par1.axis["right"].major_ticklabels.set_visible(True)
    par1.axis["right"].label.set_visible(True)
    par2.axis["right2"] = par2.new_fixed_axis(loc="right", offset=(60, 0))
    p1, p2, p3 = _three_datasets(host, par1, par2)
    host.axis["left"].label.set_color(p1.get_color())
    par1.axis["right"].label.set_color(p2.get_color())
    par2.axis["right2"].label.set_color(p3.get_color())


@gallery("demo_parasite_axes2")
def _(fig):
    host = host_subplot(111, axes_class=axisartist.Axes, figure=fig)
    fig.subplots_adjust(right=0.75)
    par1 = host.twinx()
    par2 = host.twinx()
    par2.axis["right"] = par2.new_fixed_axis(loc="right", offset=(60, 0))
    par1.axis["right"].toggle(all=True)
    par2.axis["right"].toggle(all=True)
    p1, p2, p3 = _three_datasets(host, par1, par2)
    host.axis["left"].label.set_color(p1.get_color())
    par1.axis["right"].label.set_color(p2.get_color())
    par2.axis["right"].label.set_color(p3.get_color())


# --- widgets: cursors ------------------------------------------------------------------


def _annotated_cursor(fig, title):
    """``annotated_cursor``'s figures: a parabola under a red crosshair
    cursor whose text, blue and bold, stays hidden until the pointer
    moves."""
    ax = fig.subplots()
    ax.set_title(title)
    x = np.linspace(-5, 5, 1000)
    ax.plot(x, x**2)
    ax.set_xlim(-5, 5)
    ax.set_ylim(0, 25)
    cursor = widgets.Cursor(ax, useblit=True, color="red", linewidth=2)
    ax.text(ax.get_xbound()[0], ax.get_ybound()[0], "0, 0", animated=True, visible=False,
            color="blue", fontweight="bold")
    return cursor


@gallery("annotated_cursor_1", figsize=(8, 6))
def _(fig):
    _annotated_cursor(fig, "Cursor Tracking x Position")


@gallery("annotated_cursor_2", figsize=(8, 6))
def _(fig):
    _annotated_cursor(fig, "Cursor Tracking y Position")


@gallery("cursor", figsize=(8, 6))
def _(fig):
    ax = fig.subplots()
    x, y = 4 * (rng(SEED).uniform(200).reshape(2, 100) - .5)
    ax.plot(x, y, "o")
    ax.set_xlim(-2, 2)
    ax.set_ylim(-2, 2)
    widgets.Cursor(ax, useblit=True, color="red", linewidth=2)


@gallery("mouse_cursor", figsize=(6, len(Cursors) + 0.5))
def _(fig):
    axs = fig.subplots(len(Cursors), gridspec_kw={"hspace": 0})
    fig.suptitle("Hover over an Axes to see alternate Cursors")
    for cursor, ax in zip(Cursors, axs):
        ax.text(0.5, 0.5, cursor.name, horizontalalignment="center",
                verticalalignment="center")
        ax.set(xticks=[], yticks=[])


T = np.arange(0.0, 2.0, 0.01)


@gallery("multicursor_1")
def _(fig):
    ax1, ax2 = fig.subplots(2, sharex=True)
    ax1.plot(T, np.sin(2 * np.pi * T))
    ax2.plot(T, np.sin(3 * np.pi * T))
    widgets.MultiCursor((ax1, ax2), color="r", lw=1)


@gallery("multicursor_2")
def _(fig):
    ax3 = fig.subplots()
    ax3.plot(T, np.sin(4 * np.pi * T))
    widgets.MultiCursor((ax3,), color="r", lw=1)


# --- widgets: buttons ------------------------------------------------------------------


@gallery("buttons")
def _(fig):
    freqs = np.arange(2, 20, 3)
    ax = fig.subplots()
    fig.subplots_adjust(bottom=0.2)
    t = np.arange(0.0, 1.0, 0.001)
    ax.plot(t, np.sin(2 * np.pi * freqs[0] * t), lw=2)
    axprev = fig.add_axes((0.7, 0.05, 0.1, 0.075))
    axnext = fig.add_axes((0.81, 0.05, 0.1, 0.075))
    widgets.Button(axnext, "Next")
    widgets.Button(axprev, "Previous")


@gallery("check_buttons")
def _(fig):
    ax = fig.subplots()
    l0, = ax.plot(T, np.sin(2 * np.pi * T), visible=False, lw=2, color="black", label="1 Hz")
    l1, = ax.plot(T, np.sin(4 * np.pi * T), lw=2, color="red", label="2 Hz")
    l2, = ax.plot(T, np.sin(6 * np.pi * T), lw=2, color="blue", label="3 Hz")
    lines_by_label = {line.get_label(): line for line in [l0, l1, l2]}
    line_colors = [line.get_color() for line in lines_by_label.values()]
    rax = ax.inset_axes([0.0, 0.0, 0.12, 0.2])
    widgets.CheckButtons(
        ax=rax,
        labels=lines_by_label.keys(),
        actives=[line.get_visible() for line in lines_by_label.values()],
        label_props={"color": line_colors},
        frame_props={"edgecolor": line_colors},
        check_props={"facecolor": line_colors},
    )


def _button_panel(ax, title):
    """The light grey panel with a title that holds radio buttons."""
    ax.set_facecolor("0.95")
    ax.spines[:].set_color("0.8")
    ax.set_title(title)


@gallery("radio_buttons")
def _(fig):
    fig.set_layout_engine("constrained")
    axd = fig.subplot_mosaic(
        [["main", "freq"], ["main", "color"], ["main", "linestyle"]], width_ratios=[5, 1])
    axd["main"].plot(T, np.sin(2 * np.pi * T), lw=2, color="red")
    axd["main"].set(xlabel="Time (s)", ylabel="Amplitude", title="Sine Wave")
    _button_panel(axd["freq"], "Frequency")
    widgets.RadioButtons(axd["freq"], labels=["1 Hz", "2 Hz", "4 Hz"],
                         label_props={"fontsize": [12, 14, 16]},
                         radio_props={"s": [16, 32, 64]})
    _button_panel(axd["color"], "Color")
    widgets.RadioButtons(
        axd["color"], ("red", "blue", "green"),
        label_props={"color": ["red", "blue", "green"]},
        radio_props={"facecolor": ["red", "blue", "green"],
                     "edgecolor": ["darkred", "darkblue", "darkgreen"]})
    _button_panel(axd["linestyle"], "Linestyle")
    widgets.RadioButtons(axd["linestyle"], ("solid", "dashed", "dashdot", "dotted"))


@gallery("radio_buttons_grid", figsize=(8, 4))
def _(fig):
    ax, ax_buttons = fig.subplots(1, 2, width_ratios=[4, 1.4])
    ax.plot(T, np.sin(2 * np.pi * T), lw=2, color="red")
    ax.set(xlabel="Time (s)", ylabel="Amplitude", title="Sine Wave - Click a color!")
    _button_panel(ax_buttons, "Line Color")
    widgets.RadioButtons(ax_buttons, ["red", "yellow", "green", "purple", "brown", "gray"],
                         layout=(3, 2))


# --- widgets: sliders and text boxes -----------------------------------------------------


@gallery("range_slider", figsize=(10, 5))
def _(fig):
    n = 128
    img = rng(SEED).normal(n * n).reshape(n, n)
    axs = fig.subplots(1, 2)
    fig.subplots_adjust(bottom=0.25)
    axs[0].imshow(img)
    axs[1].hist(img.flatten(), bins="auto")
    axs[1].set_title("Histogram of pixel intensities")
    slider_ax = fig.add_axes((0.20, 0.1, 0.60, 0.03))
    slider = widgets.RangeSlider(slider_ax, "Threshold", img.min(), img.max())
    axs[1].axvline(slider.val[0], color="k")
    axs[1].axvline(slider.val[1], color="k")


def _reset_button(fig):
    """The sliders examples' reset button."""
    return widgets.Button(fig.add_axes((0.8, 0.025, 0.1, 0.04)), "Reset", hovercolor="0.975")


@gallery("slider_demo")
def _(fig):
    t = np.linspace(0, 1, 1000)
    ax = fig.subplots()
    ax.plot(t, 5 * np.sin(2 * np.pi * 3 * t), lw=2)
    ax.set_xlabel("Time [s]")
    fig.subplots_adjust(left=0.25, bottom=0.25)
    widgets.Slider(ax=fig.add_axes((0.25, 0.1, 0.65, 0.03)), label="Frequency [Hz]",
                   valmin=0.1, valmax=30, valinit=3)
    widgets.Slider(ax=fig.add_axes((0.1, 0.25, 0.0225, 0.63)), label="Amplitude", valmin=0,
                   valmax=10, valinit=5, orientation="vertical")
    _reset_button(fig)


@gallery("slider_snap_demo")
def _(fig):
    t = np.arange(0.0, 1.0, 0.001)
    ax = fig.subplots()
    fig.subplots_adjust(bottom=0.25)
    ax.plot(t, 5 * np.sin(2 * np.pi * 3 * t), lw=2)
    ax_freq = fig.add_axes((0.25, 0.1, 0.65, 0.03))
    ax_amp = fig.add_axes((0.25, 0.15, 0.65, 0.03))
    allowed_amplitudes = np.concatenate([np.linspace(.1, 5, 100), [6, 7, 8, 9]])
    widgets.Slider(ax_amp, "Amp", 0.1, 9.0, valinit=5, valstep=allowed_amplitudes,
                   color="green")
    widgets.Slider(ax_freq, "Freq", 0, 10 * np.pi, valinit=2 * np.pi, valstep=np.pi,
                   initcolor="none")
    _reset_button(fig)


@gallery("textbox")
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(bottom=0.2)
    t = np.arange(-2.0, 2.0, 0.001)
    line, = ax.plot(t, np.zeros_like(t), lw=2)

    def submit(expression):
        line.set_ydata(eval(expression, {"np": np}, {"t": t}))
        ax.relim()
        ax.autoscale_view()

    text_box = widgets.TextBox(fig.add_axes((0.1, 0.05, 0.8, 0.075)), "Evaluate",
                               textalignment="center")
    text_box.on_submit(submit)
    text_box.set_val("t ** 2")


# --- widgets: selectors ------------------------------------------------------------------


@gallery("lasso_selector_demo_sgskip")
def _(fig):
    data = rng(SEED).uniform(200).reshape(100, 2)
    ax = fig.subplots(subplot_kw=dict(xlim=(0, 1), ylim=(0, 1), autoscale_on=False))
    ax.scatter(data[:, 0], data[:, 1], s=80)
    widgets.LassoSelector(ax, onselect=_noop)
    ax.set_title("Press enter to accept selected points.")


@gallery("polygon_selector_demo")
def _(fig):
    ax = fig.subplots()
    grid_size = 5
    ax.scatter(np.tile(np.arange(grid_size), grid_size), np.repeat(np.arange(grid_size),
                                                                   grid_size))
    widgets.PolygonSelector(ax, _noop, draw_bounding_box=True)


@gallery("polygon_selector_simple_1")
def _(fig):
    ax = fig.subplots()
    selector = widgets.PolygonSelector(ax, _noop)
    selector.verts = [(0.1, 0.4), (0.5, 0.9), (0.3, 0.2)]


@gallery("polygon_selector_simple_2")
def _(fig):
    widgets.PolygonSelector(fig.subplots(), _noop)


@gallery("rectangle_selector")
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2)
    x = np.linspace(0, 10, 100000)
    for ax, selector_class in zip(axs, [widgets.RectangleSelector, widgets.EllipseSelector]):
        ax.plot(x, np.sin(2 * np.pi * x))
        ax.set_title(f"Click and drag to draw a {selector_class.__name__}.")
        selector_class(ax, _noop, useblit=True, button=[1, 3], minspanx=5, minspany=5,
                       spancoords="pixels", interactive=True)
    axs[0].set_title("Press 't' to toggle the selectors on and off.\n" + axs[0].get_title())


@gallery("span_selector", figsize=(8, 6))
def _(fig):
    ax1, ax2 = fig.subplots(2)
    x = np.arange(0.0, 5.0, 0.01)
    y = np.sin(2 * np.pi * x) + 0.5 * rng(SEED).normal(len(x))
    ax1.plot(x, y)
    ax1.set_ylim(-2, 2)
    ax1.set_title("Press left mouse button and drag to select a region in the top graph")
    ax2.plot([], [])
    widgets.SpanSelector(ax1, _noop, "horizontal", useblit=True,
                         props=dict(alpha=0.5, facecolor="tab:blue"), interactive=True,
                         drag_from_anywhere=True)
