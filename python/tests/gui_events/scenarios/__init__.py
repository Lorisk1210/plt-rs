"""
The scenarios of the harness: for each, the figure the program shows
(`Scenario.setup`, run by the recorder under either library) and the
input the driver feeds it (`Scenario.steps`, run by the driver), in the
figure's pixels from its bottom-left corner, as matplotlib's events count
them.

This package imports neither library: the recorder passes pyplot to the
setups.
"""

from dataclasses import dataclass, field


@dataclass
class Scenario:
    """A figure and the input fed to it."""
    #: ``setup(plt, log)``: makes the figure (returned) and its widgets,
    #: which write what they report with ``log.note(name, **fields)``.
    setup: object
    #: ``steps(driver)``: the input, through the driver's methods; ends by
    #: closing the window.
    steps: object = None
    #: Whether the window has the toolbar.
    toolbar: bool = False
    #: Whether the default key bindings (``keymap.*``) stay: off, keys
    #: only reach the program.
    keymaps: bool = False
    #: rcParams set before the figure is made.
    rc: dict = field(default_factory=dict)
    #: The backend of matplotlib's program to compare with; None where
    #: pltrs means to differ from matplotlib's QtAgg, and `check` alone
    #: judges pltrs' records.
    reference: object = "qtagg"
    #: ``check(records)``: what is wrong with pltrs' records besides any
    #: difference from matplotlib's (a list of lines), such as an
    #: animation's timing.
    check: object = None
    #: ``summarize(records)``: the records compared, where the raw ones
    #: depend on timing (an animation closed while it plays).
    summarize: object = None
    #: Whether the scenario needs what only an X server lets the driver
    #: do (resize windows as a window manager, remap keys for programs
    #: running), or what a window manager does that the X server lacks.
    x11_only: bool = False


#: The scenarios by name, in the order they were registered.
SCENARIOS = {}


def scenario(name, **options):
    """Register the decorated function as the setup of scenario *name*;
    ``@<setup>.steps`` registers the input."""
    def register(setup):
        holder = SCENARIOS[name] = Scenario(setup, **options)

        def steps(function):
            holder.steps = function
            return function
        setup.steps = steps

        def check(function):
            holder.check = function
            return function
        setup.check = check
        return setup
    return register


#: The figure's size in pixels in most scenarios (4 × 3 inches at 100 dpi).
WIDTH, HEIGHT = 400, 300
#: Where one axes of the default subplot parameters is in such a figure,
#: ``(left, bottom, right, top)`` pixels.
AXES_BOX = (50.0, 33.0, 360.0, 264.0)


def at(x, y, box=AXES_BOX, xlim=(0, 10), ylim=(0, 10)):
    """The figure pixel of data point *(x, y)* in axes *box* with limits
    *xlim* and *ylim*."""
    left, bottom, right, top = box
    return (left + (x - xlim[0]) / (xlim[1] - xlim[0]) * (right - left),
            bottom + (y - ylim[0]) / (ylim[1] - ylim[0]) * (top - bottom))


def one_axes(plt, **subplot):
    """A 400 × 300 figure with one axes, limits 0 to 10."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100, **subplot)
    ax.set(xlim=(0, 10), ylim=(0, 10))
    return fig, ax


def axes_state(ax):
    """What navigation changes of *ax*: its limits, scales and grids."""
    def visible(axis, which):
        ticks = (axis.get_major_ticks() if which == "major"
                 else axis.get_minor_ticks())
        return bool(ticks) and ticks[0].gridline.get_visible()
    return {
        "xlim": list(ax.get_xlim()), "ylim": list(ax.get_ylim()),
        "xscale": ax.get_xscale(), "yscale": ax.get_yscale(),
        "grid": [visible(ax.xaxis, "major"), visible(ax.yaxis, "major"),
                 visible(ax.xaxis, "minor"), visible(ax.yaxis, "minor")],
    }


from . import animations, inputs, navigation, picking, selectors, widgets  # noqa: E402,F401
