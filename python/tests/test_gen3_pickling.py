"""
What the differential tester's pickling programs (task GEN3) found pltrs
doing otherwise than matplotlib 3.11: figures, artists and values pickled,
unpickled and copied. Each expected value is recorded from matplotlib.
"""

import copy
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import transforms, widgets
from pltrs.figure import Figure


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X = np.linspace(0, 2 * np.pi, 7)
Y = np.sin(X)
Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
VALUES = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])


def roundtrip(obj):
    return pickle.loads(pickle.dumps(obj, pickle.HIGHEST_PROTOCOL))


def test_a_colorbar_in_a_subfigure_pickles():
    fig = plt.figure()
    left, right = fig.subfigures(1, 2)
    im = right.add_subplot().imshow(Z)
    right.colorbar(im)
    fig.canvas.draw()
    fig2 = roundtrip(fig)
    assert [type(a).__name__ for a in fig2.axes] == ['Axes', 'Axes']
    assert fig2.subfigs[1].axes[0].images[0].colorbar is not None


def test_triangulations_pickle_with_their_contours():
    fig, ax = plt.subplots()
    ax.tricontour(X / 7 * 10, (Y + 1) * 4, VALUES, levels=3, colors='r')
    fig2 = roundtrip(fig)
    assert list(fig2.axes[0].collections[0].levels) == [0, 2, 4, 6, 8, 10]
    triangulation = roundtrip(pltrs.tri.Triangulation(X, Y))
    assert triangulation.triangles.shape == (6, 3)
    assert triangulation.edges.shape == (12, 2)


def test_curvilinear_grids_pickle():
    from pltrs.mpl_toolkits import axisartist
    from pltrs.mpl_toolkits.axisartist import angle_helper, GridHelperCurveLinear
    from pltrs.projections import PolarAxes
    fig = plt.figure()
    transform = transforms.Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform()
    helper = GridHelperCurveLinear(
        transform, extreme_finder=angle_helper.ExtremeFinderCycle(
            20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None,
            lat_minmax=(0, np.inf)),
        grid_locator1=angle_helper.LocatorDMS(12), tick_formatter1=angle_helper.FormatterDMS())
    fig.add_subplot(axes_class=axisartist.Axes, grid_helper=helper)
    fig.canvas.draw()
    fig2 = roundtrip(fig)
    assert type(fig2.axes[0].get_grid_helper()).__name__ == 'GridHelperCurveLinear'
    fig2.canvas.draw()


def test_unpickled_collections_keep_what_their_draw_computed():
    fig, ax = plt.subplots(figsize=(5, 4))
    ax.scatter(X, Y, c=VALUES, s=VALUES * 10)
    fig.canvas.draw()
    scatter = roundtrip(fig).axes[0].collections[0]
    assert scatter.get_transforms()[0, 0, 0] == pytest.approx(7.60725774)


def test_a_widget_pickles_alone():
    fig = plt.figure()
    slider = widgets.Slider(fig.add_axes((0.45, 0.05, 0.45, 0.1)), 'amp', 0, 2, valinit=1)
    fig.canvas.draw()
    copied = roundtrip(slider)
    assert (type(copied).__name__, copied.val, copied.valmax) == ('Slider', 1, 2)


def test_a_shallow_copy_of_a_figure_is_a_figure_of_its_own():
    fig = plt.figure('a label', figsize=(2, 2))
    fig.add_subplot().plot([1, 2])
    shallow = copy.copy(fig)
    assert shallow is not fig and shallow.axes == fig.axes
    assert plt.get_fignums() == [1, 2]
    assert type(shallow.canvas).__name__ == 'FigureCanvasAgg'


def test_unmanaged_figures_stay_out_of_pyplot():
    unmanaged = Figure()
    unmanaged.add_subplot().plot([1, 2])
    loaded = roundtrip(unmanaged)
    assert plt.get_fignums() == []
    assert type(loaded.canvas).__name__ == 'FigureCanvasBase'


def test_limit_callbacks_keep_their_signals():
    fig, ax = plt.subplots()
    ax.callbacks.connect('xlim_changed', lambda a: None)
    ax.callbacks.connect('ylim_changed', lambda a: None)
    ax2 = roundtrip(fig).axes[0]
    assert {name: len(found) for name, found in ax2.callbacks.callbacks.items()} == {
        'xlim_changed': 0, 'ylim_changed': 0}
    calls = []
    ax2.callbacks.connect('xlim_changed', lambda a: calls.append(a.get_xlim()))
    ax2.set_xlim(1, 4)
    assert calls == [(1, 4)]


def test_unpickled_figures_restore_the_draw_of_the_original():
    fig, ax = plt.subplots(figsize=(4, 3))
    ax.plot([0, 1], [0, 1])
    ax.set_xlabel('x label')
    fig.canvas.draw()
    ax2 = roundtrip(fig).axes[0]
    assert ax2.xaxis.label.get_position()[1] == pytest.approx(3.833333333333327)
