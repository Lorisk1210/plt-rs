"""
What the differential tester's hostile and interaction programs (seventh
round) found pltrs doing otherwise than matplotlib 3.11: axisartist's
clears, the renderer its texts keep, the callbacks matplotlib connects to
the limits of host Axes and colorbars first, a draw that stops at the
first failing ticks, bars of category names on 3-D Axes. Each expected
value is recorded from matplotlib.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.projections import PolarAxes
from pltrs.transforms import Affine2D
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import host_subplot
from pltrs.mpl_toolkits.axisartist import SubplotHost, floating_axes


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X = np.linspace(0, 2 * np.pi, 7)
Y = np.sin(X)


def units_handlers(ax):
    """How many callbacks each axis of *ax* calls on a change of units."""
    return tuple(len(axis.callbacks.callbacks.get('units', {}))
                 for axis in (ax.xaxis, ax.yaxis))


# --- Clears ----------------------------------------------------------------

@pytest.mark.parametrize("axes_class", [axisartist.Axes, SubplotHost])
def test_axisartist_clears_disconnect_the_units_handler(axes_class):
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axes_class)
    assert units_handlers(ax) == (1, 1)
    ax.cla()
    assert units_handlers(ax) == (0, 0)
    ax.plot([2, 8], [1, 6])
    ax.set_xbound('ab')
    # No handler relims: the line's limits stay.
    assert ax.dataLim.bounds == (2.0, 1.0, 6.0, 5.0)


def test_floating_axes_clear_disconnects_the_units_handler():
    fig = plt.figure()
    tr = Affine2D().scale(np.pi / 180., 1.) + PolarAxes.PolarTransform()
    helper = floating_axes.GridHelperCurveLinear(tr, extremes=(0, 90, 0, 1))
    ax = fig.add_subplot(axes_class=floating_axes.FloatingAxes, grid_helper=helper)
    assert units_handlers(ax) == (1, 1)
    ax.cla()
    assert units_handlers(ax) == (0, 0)
    assert ax.get_xlim() == pytest.approx((-0.01, 1.01))
    assert ax.get_ylim() == pytest.approx((-0.01, 1.01))


# --- The renderer of axisartist's texts -----------------------------------

def contains_none(text):
    """What ``text.contains(None)`` returns, or the exception it raises."""
    try:
        return text.contains(None)
    except AttributeError as e:
        return str(e)


def test_axisartist_texts_keep_the_renderer_of_the_draw_that_drew_them():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    bottom = ax.axis['bottom']
    parts = [bottom.major_ticklabels, bottom.minor_ticklabels, bottom.label]
    assert [contains_none(part) for part in parts] == [(False, {})] * 3
    fig.canvas.draw()
    no_x = "'NoneType' object has no attribute 'x'"
    # No minor tick label is drawn.
    assert [contains_none(part) for part in parts] == [no_x, (False, {}), no_x]
    ax.axis['left'].major_ticklabels.set_visible(False)
    fig.canvas.draw()
    assert contains_none(ax.axis['left'].major_ticklabels) == (False, {})


def test_axisartist_labels_keep_the_renderer_they_were_measured_with():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    label = ax.axis['bottom'].label
    label.set_text('xl')
    label.get_window_extent()
    assert contains_none(label) == "'NoneType' object has no attribute 'x'"


# --- Limit callbacks matplotlib connects first ----------------------------

def test_parasites_follow_their_host_limits_through_its_callbacks():
    fig = plt.figure()
    host = host_subplot(111, figure=fig)
    parasite = host.twinx()
    nothing = lambda ax: None  # noqa: E731
    assert host.callbacks.connect('xlim_changed', nothing) == 2
    assert host.callbacks.connect('ylim_changed', nothing) == 3
    assert parasite.callbacks.connect('xlim_changed', nothing) == 0
    host.cla()
    # The parasites' clear connects them again.
    assert host.callbacks.connect('xlim_changed', nothing) == 2


def test_colorbars_follow_the_limits_of_their_axes():
    fig, ax = plt.subplots()
    cb = fig.colorbar(ax.imshow(np.arange(6).reshape(2, 3)))
    nothing = lambda ax: None  # noqa: E731
    assert cb.ax.callbacks.connect('xlim_changed', nothing) == 2
    assert (cb._extend_cid1, cb._extend_cid2) == (0, 1)
    cbar_ax = cb.ax
    cb.remove()
    assert cb.ax is cbar_ax
    assert list(cb.ax.callbacks.callbacks) == ['xlim_changed']
    assert list(cb.ax.callbacks.callbacks['xlim_changed']) == [2]


# --- A draw stops at the first failing ticks -------------------------------

@pytest.mark.parametrize("twin", [True, False])
def test_a_draw_computes_failing_ticks_once(twin):
    fig = plt.figure()
    ax = fig.add_subplot(1, 2, 1)
    ax.plot(X, Y)
    other = ax.twinx() if twin else fig.add_subplot(1, 2, 2)
    other.plot(X, Y)
    fig.canvas.draw()
    for a in [ax] if twin else [ax, other]:
        a.set_xlim(*a.get_xlim()[::-1])
        a.set_xscale('log')
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        with pytest.raises(ValueError, match="cannot convert float NaN to integer"):
            fig.canvas.draw()
    assert [str(w.message) for w in caught] == ['invalid value encountered in log10']


# --- Bars of category names on 3-D Axes ------------------------------------

def test_3d_bars_take_category_names():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    bars = ax.bar(['a', 'b'], [3, 1])
    assert [type(bar).__name__ for bar in bars] == ['Patch3D', 'Patch3D']
    assert dict(ax.xaxis.units._mapping) == {'a': 0, 'b': 1}
    # One name, three heights: matplotlib's 3-D bar makes two of them 3-D.
    bars = ax.bar('ab', [3, 1, 2])
    assert [type(bar).__name__ for bar in bars] == ['Patch3D', 'Patch3D', 'Rectangle']
    assert dict(ax.xaxis.units._mapping) == {'a': 0, 'b': 1, 'ab': 2}
    bars = ax.bar([1, 2, 3], ['p', 'q', 'p'])
    assert dict(ax.yaxis.units._mapping) == {'p': 0, 'q': 1}
    with pytest.raises(AttributeError, match="'Rectangle' object has no attribute "
                                             "'do_3d_projection'"):
        fig.canvas.draw()


# --- Ticks made anew as the spines take their positions -----------------

def test_the_first_ticks_made_after_a_clear_make_the_ticks_anew():
    fig, ax = plt.subplots()
    fig.canvas.draw()
    ax.cla()
    first = ax.set_yticks([0])[0]
    assert ax.set_yticks([0])[0] is first
    ticks = ax.set_yticks([0, 0])
    assert all(tick is not first for tick in ticks)
    # The spines have their positions now: ticks are added to these.
    assert ax.set_yticks([0, 0, 0])[:2] == ticks
    ax.cla()
    tick = ax.yaxis.majorTicks[0]
    assert ax.spines.left.get_position() == ('outward', 0.0)
    assert ax.yaxis.majorTicks[0] is not tick
    tick = ax.yaxis.majorTicks[0]
    ax.spines.left.get_position()
    assert ax.yaxis.majorTicks[0] is tick


# --- What matplotlib's draws and measures do with odd arguments ------------

def test_pie_percentage_templates_of_no_conversion_label_as_written():
    fig, ax = plt.subplots()
    ax.pie([1, 2, 3], autopct='x%%')
    assert [t.get_text() for t in ax.texts] == ['', '', '', 'x%', 'x%', 'x%']
    assert len(ax.patches) == 3
    with pytest.raises(TypeError, match='autopct must be callable'):
        ax.pie([1, 2, 3], autopct=0)
    ax.pie([1, 2, 3], autopct='x')
    assert len(ax.patches) == 9


@pytest.mark.parametrize("renderer", [3, 'x'])
def test_layouts_measuring_no_text_take_anything_as_renderer(renderer):
    fig, ax = plt.subplots()
    fig.canvas.draw()
    ax.axis('off')
    assert ax.get_tightbbox(renderer).bounds == pytest.approx((80.0, 52.8, 496.0, 369.6))
    assert fig.text(0.5, 0.5, '').get_window_extent(renderer).bounds == (320.0, 240.0, 0.0, 0.0)
    ax.axis('on')
    with pytest.raises(TypeError, match="cannot create weak reference"):
        ax.get_tightbbox(renderer)
