"""
What the written2 family (task USR2: programs of the kind experts write)
showed pltrs lacking that it can read back plainly: the private names
programs read on lines and texts. Every expected value was recorded from
matplotlib 3.11.
"""

import numpy as np
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


def test_a_lines_transformed_path_is_its_path_through_its_transform():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=72)
    ln, = ax.plot(np.arange(5), np.arange(5) ** 2)
    tp = ln._get_transformed_path()
    assert type(tp).__name__ == "TransformedPath"
    assert tp is ln._get_transformed_path()
    fig.canvas.draw()
    pts, affine = tp.get_transformed_points_and_affine()
    assert np.round(affine.transform(pts.vertices)[:2], 3).tolist() == [[34.609, 20.88], [72.655, 27.18]]
    path, affine = tp.get_transformed_path_and_affine()
    assert len(path.vertices) == 5
    assert np.round(affine.get_matrix()[0], 3).tolist() == [38.045, 0.0, 34.609]


def test_a_lines_transformed_path_is_made_anew_where_matplotlib_recaches_the_line():
    fig, ax = plt.subplots()
    ln, = ax.plot(np.arange(5), np.arange(5) ** 2)
    first = ln._get_transformed_path()
    ln.set_data([0, 1], [1, 0])
    stale = ln._get_transformed_path()
    # Until the line is drawn or its path read, matplotlib keeps the old one.
    assert stale is first and len(stale.get_fully_transformed_path().vertices) == 5
    fig.canvas.draw()
    drawn = ln._get_transformed_path()
    assert drawn is not first and len(drawn.get_fully_transformed_path().vertices) == 2
    ln.set_data([0, 1, 2], [1, 0, 1])
    ln.get_path()
    read = ln._get_transformed_path()
    assert read is not drawn and len(read.get_fully_transformed_path().vertices) == 3
    ln.set_transform(ax.transAxes)
    assert ln._get_transformed_path() is read
    fig.canvas.draw()
    redrawn = ln._get_transformed_path()
    assert redrawn is not read and redrawn._transform is ax.transAxes


def test_a_texts_wrap_line_width_and_display_position():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=72)
    t = ax.text(1, 2, "wrap me " * 10, wrap=True, rotation=30)
    ax.plot(np.arange(5), np.arange(5) ** 2)
    fig.canvas.draw()
    assert round(t._get_wrap_line_width(), 3) == 165.521
    # matplotlib's sets the text's rotation mode as it measures.
    assert t.get_rotation_mode() == "anchor"
    centred = ax.text(0.5, 0.5, "x", ha="center", transform=ax.transAxes)
    assert round(centred._get_wrap_line_width(), 3) == 210.6
    assert np.round(centred._get_xy_display(), 3).tolist() == [110.7, 71.28]


def test_an_annotations_display_position_is_its_text_position():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=72)
    ax.plot(np.arange(5), np.arange(5) ** 2)
    an = ax.annotate("a", (1, 4), (2, 9), arrowprops=dict(arrowstyle="->"))
    fig.canvas.draw()
    assert np.round(an._get_xy_display(), 3).tolist() == [110.7, 77.58]
