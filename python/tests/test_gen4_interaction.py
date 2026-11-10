"""
What the differential tester's programs driving figures with events
(``generated/interaction``: the navigation toolbar, the key bindings,
the canvas' events, widgets, picking, blitting and animations) found
pltrs doing otherwise than matplotlib 3.11. Each expected value is
recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.backend_bases import KeyEvent, MouseButton, MouseEvent, NavigationToolbar2


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_axes_connect_their_units_handler_first():
    fig, ax = plt.subplots()
    assert ax.xaxis.callbacks.connect('units', lambda: None) == 1
    assert len(ax.yaxis.callbacks.callbacks['units']) == 1
    ax.cla()
    # A clear makes new registries, the Axes' handler gone with the rest.
    assert ax.xaxis.callbacks.connect('units', lambda: None) == 0
    seen = []

    def changed():
        seen.append('x')

    ax.xaxis.callbacks.connect('units', changed)
    ax.plot(['a', 'b'], [1, 2])
    assert seen == ['x']


def test_an_explicit_autoscale_runs_the_waiting_one_first():
    fig, ax = plt.subplots()
    counts = {'x': 0, 'y': 0}
    ax.callbacks.connect('xlim_changed', lambda a: counts.__setitem__('x', counts['x'] + 1))
    ax.callbacks.connect('ylim_changed', lambda a: counts.__setitem__('y', counts['y'] + 1))
    ax.plot([0, 1], [0, 4])
    ax.autoscale_view()
    assert counts == {'x': 2, 'y': 2}
    ax.autoscale_view()
    assert counts == {'x': 3, 'y': 3}
    ax.set_autoscale_on(False)
    ax.plot([0, 2], [0, 8])
    ax.autoscale_view()
    ax.get_xlim()
    assert counts == {'x': 3, 'y': 3}


def test_tick_lists_hold_the_ticks_made():
    fig, ax = plt.subplots()
    assert (len(ax.xaxis.majorTicks), len(ax.xaxis.minorTicks)) == (1, 1)
    ax.plot([0, 10], [0, 1])
    ax.xaxis.get_majorticklocs()
    assert len(ax.xaxis.majorTicks) == 1
    fig.canvas.draw()
    assert (len(ax.xaxis.majorTicks), len(ax.xaxis.minorTicks)) == (8, 1)
    ax.set_xlim(0, 2)
    fig.canvas.draw()
    assert len(ax.xaxis.majorTicks) == 9
    ax.set_xlim(0, 100)
    ax.xaxis.get_major_ticks()
    assert len(ax.xaxis.majorTicks) == 9
    ax.cla()
    assert len(ax.xaxis.majorTicks) == 1


def test_grid_key_after_a_log_scale_the_data_refuse():
    # 'k' makes x logarithmic over limits reaching below 0: the draw fails,
    # and the tick lists the grid key reads stay as the last draw left them.
    fig, ax = plt.subplots()
    ax.plot(np.linspace(0, 2 * np.pi, 7), np.linspace(0, 1, 7))
    ax.set_xlim(*ax.get_xlim()[::-1])
    fig.canvas.draw()
    NavigationToolbar2(fig.canvas)
    at = ax.transAxes.transform((0.5, 0.5))
    with pytest.raises(ValueError, match="cannot convert float NaN to integer"):
        KeyEvent('key_press_event', fig.canvas, 'k', *at)._process()
    assert ax.get_xscale() == 'log'
    assert len(ax.xaxis.majorTicks) == 9
    assert not any(t.gridline.get_visible() for t in ax.xaxis.majorTicks)


def test_a_number_picker_is_the_pick_radius_of_a_plotted_line():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2], [1, 2], 's', picker=4)
    assert (line.get_pickradius(), line.get_picker()) == (4, 4)
    line, = ax.plot([1, 2], [1, 2], picker=7, pickradius=2)
    assert line.get_pickradius() == 7
    line, = ax.plot([1, 2], [1, 2], picker=True, pickradius=2)
    assert line.get_pickradius() == 2
    assert ax.axhline(0.5, picker=3).get_pickradius() == 3


def test_collections_hit_items_as_c_ints():
    fig, ax = plt.subplots()
    dots = ax.scatter([0, 1], [0, 1], s=200)
    lines = ax.vlines([0.5], 0, 1)
    fig.canvas.draw()
    at = MouseEvent('button_press_event', fig.canvas, *ax.transData.transform((0.5, 0.5)), 1)
    assert dots.contains(at)[1]['ind'].dtype == np.int32
    hit, details = lines.contains(at)
    assert hit and details['ind'].dtype == np.int32 and list(details['ind']) == [0]


def test_polar_axes_contain_the_row_through_their_centre():
    fig = plt.figure(figsize=(4, 4))
    ax = fig.add_subplot(projection='polar')
    fig.canvas.draw()
    for x, y in ((205, 198), (54, 198)):
        event = MouseEvent('motion_notify_event', fig.canvas, x, y)
        assert ax.in_axes(event) and ax.patch.contains(event)[0]
        assert event.inaxes is None
    event = MouseEvent('motion_notify_event', fig.canvas, 143, 229)
    assert ax.in_axes(event) and event.inaxes is ax


def test_polar_pan_starts_at_the_centre():
    fig = plt.figure(figsize=(4, 4))
    ax = fig.add_subplot(projection='polar')
    ax.plot([0, 1], [1, 2])
    fig.canvas.draw()
    toolbar = NavigationToolbar2(fig.canvas)
    messages = []
    toolbar.set_message = messages.append
    toolbar.pan()
    MouseEvent('button_press_event', fig.canvas, 205, 198, MouseButton.LEFT)._process()
    MouseEvent('motion_notify_event', fig.canvas, 180, 210, buttons={MouseButton.LEFT})._process()
    MouseEvent('button_release_event', fig.canvas, 180, 210, MouseButton.LEFT)._process()
    # The pan took the motion, which no message reports.
    assert messages == []


def test_axes_contain_points_a_pixel_around_them():
    fig, ax = plt.subplots(figsize=(4, 3))
    fig.canvas.draw()
    box = ax.bbox
    middle = (box.y0 + box.y1) / 2
    assert ax.contains_point((box.x0 - 0.5, middle))
    assert ax.contains_point((box.x1 + 0.5, middle))
    assert not ax.contains_point((box.x0 - 1.5, middle))
    assert not ax.patch.contains_point((box.x0 - 0.5, middle))
    for x, y, under in ((box.x0, box.y0, False), (box.x1, box.y0, False), (box.x0, middle, True),
                        (box.x1, box.y1, True)):
        assert (MouseEvent('motion_notify_event', fig.canvas, x, y).inaxes is ax) == under


@pytest.mark.parametrize("layout, bbox_inches, renderers", [
    (None, None, ['RendererAgg']),
    (None, 'tight', ['RendererAgg', 'RendererAgg']),
    ('constrained', None, ['RendererAgg', 'RendererAgg']),
])
def test_prints_emit_a_draw_event_per_draw_with_their_renderer(layout, bbox_inches, renderers):
    import io
    fig, ax = plt.subplots(layout=layout)
    seen = []
    fig.canvas.mpl_connect(
        'draw_event', lambda event: seen.append((type(event.renderer).__name__,
                                                 event.renderer is getattr(event.canvas, 'renderer', None))))
    fig.savefig(io.BytesIO(), format='png', bbox_inches=bbox_inches)
    assert seen == [(name, True) for name in renderers]
    seen.clear()
    fig.savefig(io.BytesIO(), format='svg', bbox_inches=bbox_inches)
    assert [name for name, _ in seen] == ['MixedModeRenderer'] * len(renderers)


def test_zooms_invert_the_data_transform_as_matplotlib_does():
    fig, axs = plt.subplots(2, 1, figsize=(4, 4))
    ax = axs[0]
    ax.set_xlim(-0.3141592653589793, 6.5973445725385655)
    ax.set_ylim(-0.9526279441628829, 0.9526279441628827)
    fig.canvas.draw()
    ax._set_view_from_bbox((112, 240, 236, 310))
    assert ax.get_xlim() == (1.0681415022205296, 3.8327430373795472)
    assert ax.get_ylim() == (-0.57157676649773, 0.3810511776651526)


def test_axisartist_draws_fail_as_their_locator_fails():
    from pltrs.mpl_toolkits import axisartist
    fig = plt.figure(figsize=(4, 3))
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.plot(np.linspace(0, 2 * np.pi, 7), np.sin(np.linspace(0, 2 * np.pi, 7)))
    fig.canvas.draw()
    ax.set_xlim(*ax.get_xlim()[::-1])
    ax.set_xscale('log')
    with pytest.warns(RuntimeWarning, match="invalid value encountered in log10"), \
            pytest.raises(ValueError, match="cannot convert float NaN to integer"):
        fig.canvas.draw()
