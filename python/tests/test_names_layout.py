"""matplotlib's names and class layout in pltrs (Python): the module-level
names programs import, the methods each class defines as its own, and the
calls whose meaning the signatures carry. The expected values are
matplotlib 3.11's."""

import datetime
import inspect
import io

import dateutil.rrule
import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections as mcoll, font_manager, spines
from pltrs.backend_bases import MouseEvent
from pltrs.lines import Line2D
from pltrs.mpl_toolkits.mplot3d.art3d import Collection3D
from pltrs.projections.geo import LambertAxes


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_dates_reexport_dateutils_weekdays_and_rules():
    from pltrs.dates import (EPOCH_OFFSET, FRIDAY, MONDAY, SUNDAY, WEEKDAYS,
                             relativedelta, rrule)
    assert MONDAY is dateutil.rrule.MO and FRIDAY is dateutil.rrule.FR
    assert WEEKDAYS == (MONDAY, *WEEKDAYS[1:6], SUNDAY) and len(WEEKDAYS) == 7
    assert EPOCH_OFFSET == 719163.0
    assert rrule is dateutil.rrule.rrule
    assert relativedelta(months=1) + datetime.date(2020, 1, 31) == datetime.date(2020, 2, 29)


def test_modules_have_the_modules_they_import():
    import pltrs.cbook
    import pltrs.figure
    assert pltrs.cbook.np is np
    assert pltrs.figure.mpl is pltrs
    from pltrs.mpl_toolkits.axisartist.axes_divider import make_axes_locatable  # noqa: F401


def test_pyplot_star_import_takes_every_public_name():
    import pltrs.pylab as pylab
    assert not hasattr(plt, "__all__")
    for name in ("axhline", "imshow", "Figure", "Line2D", "colormaps", "rrule"):
        assert hasattr(pylab, name), name


def test_methods_are_the_classes_own():
    assert Line2D.draw.__qualname__ == "Line2D.draw"
    assert Line2D.set.__module__ == "pltrs.artist"
    assert font_manager.FontProperties.set_name is font_manager.FontProperties.set_family
    assert inspect.ismethod(font_manager.get_font_names)
    assert str(inspect.signature(spines.Spine)) == "(axes, spine_type, path, **kwargs)"
    assert str(inspect.signature(mcoll.PolyQuadMesh)) == "(coordinates, **kwargs)"


def test_a_locked_canvas_picks_nothing():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1], picker=True)
    picked = []
    fig.canvas.mpl_connect("pick_event", lambda event: picked.append(event.artist))
    x, y = ax.transData.transform((0.5, 0.5))
    event = MouseEvent("button_press_event", fig.canvas, x, y, 1)
    fig.canvas.widgetlock(ax)
    fig.pick(event)
    assert picked == []
    fig.canvas.widgetlock.release(ax)
    fig.pick(event)
    assert len(picked) == 1


def test_print_methods_take_matplotlibs_keywords():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="unexpected keyword argument 'facecolor'"):
        fig.canvas.print_png(io.BytesIO(), facecolor="red")
    out = io.BytesIO()
    fig.savefig(out, format="raw", facecolor="blue")
    width, height = fig.canvas.get_width_height()
    pixels = np.frombuffer(out.getvalue(), np.uint8).reshape(height, width, 4)
    assert pixels[0, 0].tolist() == [0, 0, 255, 255]


def test_classes_without_matplotlibs_methods():
    subfigure = plt.figure().subfigures(1, 2)[0]
    assert [hasattr(subfigure, name) for name in (
        "set_layout_engine", "draw_artist", "add_axobserver",
        "draw_without_rendering", "get_layout_engine")] == [
            False, False, False, False, True]
    ax3d = plt.figure().add_subplot(projection="3d")
    assert ax3d.set_frame_on is None and ax3d.get_frame_on is None
    with pytest.raises(AttributeError,
                       match="Collection3D.set.. got an unexpected keyword argument 'sort_zpos'"):
        Collection3D().set(sort_zpos=1)


def test_default_tick_helpers_can_be_marked():
    fig, ax = plt.subplots()
    ax.xaxis.isDefault_majfmt = False
    assert (ax.xaxis.isDefault_majfmt, ax.xaxis.isDefault_majloc) == (False, True)
    ax.plot([datetime.date(2020, 1, 1), datetime.date(2020, 2, 1)], [1, 2])
    # The date units replace the default locator, not the formatter.
    assert type(ax.xaxis.get_major_formatter()).__name__ == "ScalarFormatter"
    assert type(ax.xaxis.get_major_locator()).__name__ == "AutoDateLocator"


def test_lambert_axes_take_their_centre():
    fig = plt.figure()
    ax = fig.add_subplot(projection="lambert", center_longitude=0.5, center_latitude=0.2)
    assert ax.transData.transform([[0.5, 0.2]]).tolist() == [[328.0, 237.59999999999997]]
    made = LambertAxes(fig, [0, 0, 1, 1], center_longitude=0.5, center_latitude=0.2)
    assert made.transProjection._center == (0.5, 0.2)


def test_poly_quad_mesh_is_made_from_its_coordinates():
    x, y = np.meshgrid(np.arange(4.0), np.arange(3.0))
    coordinates = np.ma.masked_array(np.stack([x, y], axis=-1))
    coordinates[1, 1] = np.ma.masked
    mesh = mcoll.PolyQuadMesh(coordinates, facecolor="red")
    assert len(mesh.get_paths()) == 2
    assert mesh.get_coordinates().shape == (3, 4, 2)
    assert mesh.get_facecolor().tolist() == [[1.0, 0.0, 0.0, 1.0]]
    fig, ax = plt.subplots()
    ax.add_collection(mesh)
    ax.autoscale_view()
    assert ax.get_xlim() == (1.95, 3.05) and ax.get_ylim() == (-0.1, 2.1)
    mesh.set_array(np.arange(6.0))
    assert len(mesh.get_paths()) == 2
