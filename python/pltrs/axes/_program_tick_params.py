"""
The tick sides of an Axes class of the program's that overrides
``tick_params`` (astropy's ``WCSAxes``) or ``clear``.

matplotlib's ``_AxesBase.__init__`` gives the ticks their sides from the
rcParams (``xtick.top``, ``ytick.labelleft``, ...) by calling
``self.tick_params``. pltrs (Rust) makes an Axes with those sides already,
which is the same unless a subclass overrides ``tick_params``: then the
override receives them instead (``WCSAxes`` hands them to its own
coordinates, or ignores them), and the axes' ticks stay as matplotlib's
`.Tick` makes them, marked on both sides and labelled on the first.
"""

from .. import rcParams
from .._program_classes import takes_from_program

#: The sides of the ticks of an axis as matplotlib's `.Tick` makes them,
#: by axis.
_TICK_DEFAULTS = {
    "x": {"bottom": True, "top": True, "labelbottom": True, "labeltop": False},
    "y": {"left": True, "right": True, "labelleft": True, "labelright": False},
}


def apply(ax):
    """Give the ticks of *ax* their sides from the rcParams through
    ``ax.tick_params``, as matplotlib's ``__init__`` ends, where that can
    differ from the sides pltrs (Rust) made it with: its class overrides
    ``tick_params`` (which then receives them), or ``clear`` (which may
    have changed them)."""
    overrides = takes_from_program(type(ax), "tick_params")
    if not (overrides or takes_from_program(type(ax), "clear")):
        return
    if overrides:
        for axis in (ax.xaxis, ax.yaxis):
            axis.set_tick_params(which="both", **_TICK_DEFAULTS[axis.axis_name])
    for which in ("minor", "major"):
        ax.tick_params(
            top=rcParams["xtick.top"] and rcParams[f"xtick.{which}.top"],
            bottom=rcParams["xtick.bottom"] and rcParams[f"xtick.{which}.bottom"],
            labeltop=rcParams["xtick.labeltop"] and rcParams[f"xtick.{which}.top"],
            labelbottom=(rcParams["xtick.labelbottom"]
                         and rcParams[f"xtick.{which}.bottom"]),
            left=rcParams["ytick.left"] and rcParams[f"ytick.{which}.left"],
            right=rcParams["ytick.right"] and rcParams[f"ytick.{which}.right"],
            labelleft=rcParams["ytick.labelleft"] and rcParams[f"ytick.{which}.left"],
            labelright=(rcParams["ytick.labelright"]
                        and rcParams[f"ytick.{which}.right"]),
            which=which)
