"""
What the merges of 2026-11-13 broke on main (ECO6's, GEN5's and PROJ2's
met there): each expected value is recorded from matplotlib 3.11.
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
