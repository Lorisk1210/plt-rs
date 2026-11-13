"""
More of matplotlib's public instance attributes assigned as programs
assign them (task TYPES3): a legend's labels, handles and frame, an
annotation's arrow, read where matplotlib reads them (the getters, the
next draw and measure, a pickle); and copies of ``rcParams``, which read
the "auto" backend as matplotlib's do. Expected values are matplotlib
3.11.2's (``target/types1/assign/probe.py``, ``rccopy.py``).
"""

import os
import pickle
import subprocess
import sys

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import offsetbox, patches


def drawn(fig):
    # Twice: a figure's layout may settle after its first draw.
    fig.canvas.draw()
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def legend_figure():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    ax.plot([0, 1], label="a")
    ax.plot([1, 0], label="b")
    return fig, ax.legend()


def test_legend_lists_read_back_through_their_getters():
    fig, legend = legend_figure()
    before = drawn(fig)
    legend.texts = []
    legend.legend_handles = []
    # matplotlib's draw does not read them: the legend draws as it was.
    assert np.array_equal(before, drawn(fig))
    assert (legend.get_texts(), legend.get_lines(), legend.get_patches()) == ([], [], [])
    plt.close(fig)


def test_an_assigned_legend_frame_is_drawn_as_the_frame():
    fig, legend = legend_figure()
    before = drawn(fig)
    frame = patches.FancyBboxPatch((0, 0), 1, 1, boxstyle="square", facecolor="yellow",
                                   edgecolor="blue", lw=3)
    legend.legendPatch = frame
    assert legend.get_frame() is frame and legend.legendPatch is frame
    assert not np.array_equal(before, drawn(fig))
    # matplotlib sizes the frame to the legend's box when it draws it.
    assert frame.get_facecolor() == (1.0, 1.0, 0.0, 1.0)
    frame.set_facecolor("green")
    assert legend.get_frame().get_facecolor() == (0.0, 0.5019607843137255, 0.0, 1.0)
    plt.close(fig)


@pytest.mark.parametrize("make, message", [
    (lambda ax: ax.annotate("x", (0.2, 0.2), xytext=(0.6, 0.6), arrowprops={"arrowstyle": "->"}),
     "'NoneType' object has no attribute 'set_mutation_scale'"),
    (lambda ax: ax.add_artist(offsetbox.AnnotationBbox(
        offsetbox.TextArea("box"), (0.2, 0.2), xybox=(0.6, 0.6), arrowprops={"arrowstyle": "->"})),
     "'NoneType' object has no attribute 'set_positions'"),
])
def test_an_arrow_assigned_none_fails_the_next_draw(make, message):
    fig, ax = plt.subplots()
    annotation = make(ax)
    annotation.arrow_patch = None
    assert annotation.arrow_patch is None
    # matplotlib's update_positions calls the arrow's methods when the
    # annotation is drawn or measured; the figure pickles all the same.
    with pytest.raises(AttributeError, match=message):
        fig.canvas.draw()
    with pytest.raises(AttributeError, match=message):
        annotation.get_window_extent(fig.canvas.get_renderer())
    pickle.loads(pickle.dumps(fig))
    plt.close(fig)


def test_an_assigned_arrow_is_drawn_as_the_arrow():
    fig, ax = plt.subplots(figsize=(2, 2), dpi=50)
    annotation = ax.annotate("x", (0.2, 0.2), xytext=(0.6, 0.6), arrowprops={"arrowstyle": "->"})
    before = drawn(fig)
    arrow = patches.FancyArrowPatch((0, 0), (1, 1), arrowstyle="-[", color="red", lw=3)
    annotation.arrow_patch = arrow
    assert annotation.arrow_patch is arrow
    assert not np.array_equal(before, drawn(fig))
    plt.close(fig)


#: Copying ``rcParams`` resolves its "auto" backend (``copy``'s protocol
#: reads the items), as recorded from matplotlib 3.11.2.
RC_COPY = """
import copy, sys
import pltrs
c = copy.copy(pltrs.rcParams)
print("pltrs.pyplot" in sys.modules, c["backend"], c == pltrs.rcParams,
      copy.deepcopy(pltrs.rcParams)["backend"], pltrs.rcParams.copy() == pltrs.rcParams)
"""


def test_copies_of_rcparams_resolve_the_auto_backend(tmp_path):
    environment = {key: value for key, value in os.environ.items()
                   if key not in ("MPLBACKEND", "DISPLAY", "WAYLAND_DISPLAY")}
    environment["PYTHONPATH"] = os.pathsep.join(
        [os.path.dirname(os.path.dirname(pltrs.__file__)), environment.get("PYTHONPATH", "")])
    environment["MPLCONFIGDIR"] = environment["PLTRSCONFIGDIR"] = str(tmp_path)
    done = subprocess.run([sys.executable, "-c", RC_COPY], env=environment, cwd=tmp_path,
                          capture_output=True, text=True, check=True)
    assert done.stdout.split() == ["True", "agg", "True", "agg", "True"]
