"""
What the merges of 2026-11-13 broke on main (ECO6's, GEN5's and PROJ2's
met there) and what the comparison after them found: each expected value
is recorded from matplotlib 3.11.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.table import Cell, Table


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _pixels_of(fig, colour):
    """How many pixels of the drawn *fig* have exactly the RGB *colour*."""
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())[..., :3]
    return int((pixels == colour).all(axis=-1).sum())


def test_a_cell_made_by_its_constructor_is_drawn_in_its_colours():
    # The class holding a table's cells by position went in between
    # ``Cell`` and its decorator, which pushes a cell's look to pltrs
    # (Rust): a cell the program made was drawn without its fill.
    fig, ax = plt.subplots()
    ax.set_axis_off()
    table = Table(ax, bbox=[0.1, 0.1, 0.8, 0.8])
    cell = Cell((0, 0), 1, 1, text="a", facecolor="red", edgecolor="blue")
    table[0, 0] = cell
    ax.add_table(table)
    assert cell.get_facecolor() == (1.0, 0.0, 0.0, 1.0)
    assert cell.get_edgecolor() == (0.0, 0.0, 1.0, 1.0)
    assert _pixels_of(fig, (255, 0, 0)) == 114706
    cell.set_facecolor("lime")
    assert _pixels_of(fig, (0, 255, 0)) == 114706


def _boxes_to_anchor(ax):
    """Legend keywords anchoring to a box, by name."""
    from pltrs.transforms import Bbox, TransformedBbox
    return {
        "a box": {"bbox_to_anchor": Bbox([[0.1, 0.1], [0.9, 0.9]])},
        "a box and a transform": {
            "bbox_to_anchor": Bbox([[0.1, 0.2], [0.6, 0.9]]),
            "bbox_transform": ax.transAxes, "loc": "center"},
        "the Axes' box": {"bbox_to_anchor": ax.bbox, "loc": "upper left"},
        "a transformed box": {
            "bbox_to_anchor": TransformedBbox(Bbox.unit(), ax.transAxes),
            "loc": "lower right"},
    }


@pytest.mark.parametrize("name, window, anchor", [
    ("a box", [136.544, 353.048, 59.0, 25.448],
     [129.6, 89.76, 396.8, 295.68]),
    ("a box and a transform", [224.1, 243.356, 59.0, 25.448],
     [129.6, 126.72, 248.0, 258.72]),
    # The transform of the parent's fractions applies to a box as to a
    # tuple, so one in pixels lands far outside the figure.
    ("the Axes' box", [39766.944, 156139.448, 59.0, 25.448],
     [39760.0, 19567.68, 246016.0, 136604.16]),
    ("a transformed box", [285710.056, 19574.624, 59.0, 25.448],
     [39760.0, 19567.68, 246016.0, 136604.16]),
])
def test_a_legend_is_made_anchored_to_a_box(name, window, anchor):
    # The constructor handed ``bbox_to_anchor`` to pltrs (Rust) as the
    # numbers of a tuple, so a `.BboxBase`, which the docstring names
    # first, raised "'Bbox' object is not iterable" (ultraplot's centered
    # legends).
    from pltrs.transforms import TransformedBbox
    fig, ax = plt.subplots()
    ax.plot([1, 2], label="a")
    legend = ax.legend(**_boxes_to_anchor(ax)[name])
    fig.canvas.draw()
    assert type(legend.get_bbox_to_anchor()) is TransformedBbox
    assert legend.get_bbox_to_anchor().bounds == pytest.approx(anchor)
    assert legend.get_window_extent().bounds == pytest.approx(window, abs=1e-3)


def test_a_figure_legend_is_made_anchored_to_a_box():
    from pltrs.legend import Legend
    from pltrs.transforms import Bbox
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    box = Bbox([[0.2, 0.2], [0.8, 0.8]])
    legend = fig.legend([line], ["a"], bbox_to_anchor=box, loc="center")
    fig.canvas.draw()
    assert legend.get_bbox_to_anchor().bounds == pytest.approx(
        [128.0, 96.0, 384.0, 288.0])
    assert legend.get_window_extent().bounds == pytest.approx(
        [290.5, 227.276, 59.0, 25.448], abs=1e-3)
    # The constructor alone, which makes a legend its parent does not draw.
    alone = Legend(ax, [line], ["a"], bbox_to_anchor=box)
    assert alone.get_bbox_to_anchor().bounds == pytest.approx(
        [179.2, 126.72, 297.6, 221.76])
    assert ax.get_legend() is None
