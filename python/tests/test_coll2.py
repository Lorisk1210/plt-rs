"""
Contour sets changed after they are made, drawn as matplotlib 3.11 draws
them (task COLL2): hatches assigned to a filled set, the array of a set
made with given colours (integers indexing the colormap through a
`NoNorm`, as for any collection), a hatched set's array reaching every
band, and a transform set to None standing for the data coordinates.
Every expected value was recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors as mcolors

x = np.linspace(-2, 2, 30)
X, Y = np.meshgrid(x, x)
Z = np.sin(X) * np.cos(Y)


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _figure():
    return plt.subplots(figsize=(3, 3), dpi=40)


def _pixels(fig):
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba())[..., :3].copy()


def _dark(fig):
    return int((_pixels(fig).astype(int).sum(-1) < 200).sum())


@pytest.mark.parametrize("drawn", [False, True])
def test_hatches_assigned_to_a_filled_set_are_drawn_as_given_ones(drawn):
    fig, ax = _figure()
    cs = ax.contourf(X, Y, Z, levels=3)
    if drawn:
        fig.canvas.draw()
    cs.hatches = ["//", "\\\\", "xx"]
    assigned = _pixels(fig)
    made_fig, made_ax = _figure()
    made_ax.contourf(X, Y, Z, levels=3, hatches=["//", "\\\\", "xx"])
    assert (assigned == _pixels(made_fig)).all()
    assert _dark(fig) == 1725


def test_hatches_assigned_to_a_set_of_given_colours():
    fig, ax = _figure()
    cs = ax.contourf(X, Y, Z, levels=[-0.5, 0, 0.5], colors=["red", "green"])
    cs.hatches = ["//", "\\\\", "xx"]
    assert _dark(fig) == 3927


@pytest.mark.parametrize("drawn", [False, True])
def test_the_array_of_a_set_of_given_colours_indexes_them(drawn):
    fig, ax = _figure()
    cs = ax.contour(X, Y, Z, levels=[-0.5, 0, 0.5], colors=["red", "green", "blue"])
    if drawn:
        fig.canvas.draw()
    cs.set_array(cs.cvalues)
    fig.canvas.draw()
    assert cs.get_edgecolor().tolist() == [
        [1.0, 0.0, 0.0, 1.0], [0.0, 0.5019607843137255, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]]
    cs.set_array(np.array([2, 1, 0]))
    fig.canvas.draw()
    assert np.round(cs.get_edgecolor(), 2).tolist() == [
        [0.0, 0.0, 1.0, 1.0], [0.0, 0.5, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0]]


def test_integers_index_the_colormap_through_an_identity_norm():
    cmap = mcolors.ListedColormap(["red", "green", "blue"])
    for values, expected in ((np.array([[0, 1]]), [[1, 0, 0], [0, 0.5, 0]]),
                             (np.array([[True, False]]), [[0, 0.5, 0], [1, 0, 0]]),
                             (np.array([[0.0, 1.0]]), [[1, 0, 0], [0, 0, 1]])):
        fig, ax = plt.subplots()
        mesh = ax.pcolormesh(values, cmap=cmap, norm=mcolors.NoNorm())
        fig.canvas.draw()
        assert np.round(mesh.get_facecolor()[:, :3], 2).tolist() == expected


def test_a_hatched_sets_array_reaches_every_band():
    fig, ax = _figure()
    cs = ax.contourf(X, Y, Z, levels=3, hatches=["/", None, "x"])
    cs.set_array(cs.cvalues[::-1].copy())
    assert _dark(fig) == 786


@pytest.mark.parametrize("filled", [False, True])
def test_none_for_a_transform_is_the_data_transform(filled):
    fig, ax = _figure()
    cs = (ax.contourf if filled else ax.contour)(X, Y, Z, levels=3)
    before = _pixels(fig)
    cs.set_transform(None)
    assert cs.get_transform() is ax.transData
    assert cs.is_transform_set()
    assert (_pixels(fig) == before).all()
