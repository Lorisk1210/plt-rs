"""
What task TAIL5 found pltrs doing otherwise than matplotlib 3.11: the
smallest positive datum a shared locator reads where it autoscales and
where it widens limits a program sets. Each expected value is recorded
from matplotlib.
"""

import pytest

import pltrs
import pltrs.pyplot as plt


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _grid():
    """A 2x2 grid sharing y by rows and x by columns."""
    fig = plt.figure()
    axs = fig.subplots(2, 2)
    axs[0, 1].sharey(axs[0, 0]); axs[1, 1].sharey(axs[1, 0])
    axs[0, 0].sharex(axs[1, 0]); axs[0, 1].sharex(axs[1, 1])
    return fig, axs


def _rounded(limits):
    return ["%.3g" % v for v in limits]


@pytest.mark.parametrize("draw", [False, True], ids=["before a draw", "after a draw"])
def test_shared_log_axes_autoscale_from_every_siblings_smallest_datum(draw):
    # python-control's pvtol-nested-ss, reduced: the bottom row's scales
    # set the shared limits, which autoscales the top row from the axes
    # whose own smallest datum is the larger one.
    fig, axs = _grid()
    axs[0, 0].loglog([1, 2, 3], [1e-21, 1e-3, 1])
    axs[0, 1].loglog([1, 2, 3], [0, 1e-9, 1])
    axs[1, 0].loglog([1, 2, 3], [1, 2, 3])
    axs[1, 1].loglog([1, 2, 3], [1, 2, 3])
    if draw:
        fig.canvas.draw()
    assert _rounded(axs[0, 0].get_ylim()) == ["8.91e-23", "11.2"]
    assert _rounded(axs[0, 1].get_ylim()) == ["8.91e-23", "11.2"]


def test_a_limit_set_on_a_shared_logit_axis_is_widened_with_the_bound_axis_datum():
    # The shared locator is bound to the first axes' axis, whose smallest
    # datum replaces the zero given.
    fig, axs = _grid()
    for ax in axs.flat:
        ax.set_yscale("logit")
    axs[0, 0].plot([1, 2, 3], [1e-21, 1e-3, 0.5])
    axs[0, 1].plot([0, 1e-9, 4], [0, 1e-9, 0.9])
    axs[0, 1].set_ylim(0, 1)
    assert _rounded(axs[0, 0].get_ylim())[0] == "1e-21"
