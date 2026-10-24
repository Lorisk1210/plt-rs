"""matplotlib's own tests of polar axes (``tests/test_polar.py``) that the
``polar_behaviour`` area and the ``mpl_polar`` figure group leave out: the
refused ``datalim`` adjustable, gridline alpha, the outer spine's extent, the
interpolation of θ spans, the tick labels of shared polar grids, the status
bar text with custom formatters, the radial locator and its tick labels.

Area ``mpl_polar``. Every op draws on ``Figure()`` (6.4 x 4.8 in at 100
dpi) with ``ax = fig.add_subplot(projection="polar")`` unless noted.

Ops
---
``axes.Axes.set_adjustable``  {projection, adjustable} -> ``get_adjustable()``
    after ``set_adjustable(adjustable)`` on an axes of ``projection``, or
    the error refusing it.
``axes.Axes.grid``  {alpha} -> the alpha of the first major x and y
    gridlines after ``ax.grid(alpha=alpha)`` and a draw.
``spines.Spine.get_tightbbox``  {theta_direction?, theta_zero_location?,
    theta_offset?} -> the extents of ``ax.spines["polar"]`` after a draw,
    the axes turned by ``set_theta_direction``, ``set_theta_zero_location(
    *theta_zero_location)`` and ``set_theta_offset`` as given.
``axes.Axes.axvspan._interpolation_steps``  {xmin, xmax} -> the
    interpolation steps of the path of ``ax.axvspan(xmin, xmax)``.
``figure.Figure.subplots.get_ticklabels``  {how, projection} -> the texts of
    the x and y tick labels shown on the upper right axes of a 2 x 2 grid
    sharing both axes, after a draw: ``fig.subplots(2, 2, sharex=True,
    sharey=True, subplot_kw={"projection": projection})`` for ``how =
    "subplots"``, ``fig.subplot_mosaic("ab\\ncd", ...)`` for ``"mosaic"``.
``axes.Axes.format_coord``  {theta, r, fmt_xdata, fmt_ydata} -> the status
    bar text of data point ``(theta, r)``; a formatter is ``null`` or
    ``"millions"``, the test's ``'$%1.1fM' % (x * 1e-6)``.
``axis.RadialAxis.get_majorticklocs``  {} -> ``[isDefault_majloc,
    majorticklocs]`` of the radial axis after each step of
    ``test_radial_locator_wrapping``: as made, after
    ``set_major_locator(MaxNLocator(3))``, ``clear()``, ``set_rticks([0, 1,
    2, 3])``, ``clear()``, ``set_rgrids([0, 1, 2, 3])``, ``clear()`` and
    ``set_yscale("log")``.
``axis.RadialAxis.set_ticklabels``  {ticks, labels} -> the radial tick
    labels shown after ``set_rticks(ticks)``, ``yaxis.set_ticklabels(labels)``
    and a draw, or the error refusing a label count that does not match.

Cases
-----
Named after the upstream tests; ``test_cursor_precision`` asks for the
upstream data points here (the ``polar_behaviour`` area moves the cursor in
pixels). ``axes.Axes.format_coord`` draws the figure first: until then
matplotlib's polar axes are still stretched to the subplot box (the equal
aspect applies at draw time), while plt-rs lays a figure out before
formatting coordinates.

Already covered
---------------
By the ``polar_behaviour`` area: ``test_polar_rlim_zero``,
``test_polar_no_data``, ``test_polar_default_log_lims``,
``test_get_tightbbox_polar``, ``test_thetalim_valid_invalid``,
``test_thetalim_args``, ``test_default_thetalocator``,
``test_axvline_axvspan_do_not_modify_rlims``, ``test_polar_neg_theta_lims`` and
``test_radial_limits_behavior``. By the ``mpl_pyplot`` area:
``test_polar_twice``. By the ``mpl_polar`` figure group: every image test
and the ``check_figures_equal`` tests ``test_polar_wrap``,
``test_polar_rlim``, ``test_polar_rlim_bottom``,
``test_polar_interpolation_steps_constant_r``,
``test_polar_interpolation_steps_variable_r``, ``test_remove_shared_polar``,
``test_polar_log_rorigin`` and the reference figures of
``test_polar_units_1`` and ``test_polar_units_2``.

Left out
--------
``format_coord(0, 0.005)`` of ``test_cursor_precision``: one pixel from the
centre, a neighbouring pixel lies straight across it, and whether the θ
resolution rounds to half a turn or just below is a matter of the last bit.
The test figures of ``test_polar_units_1`` and ``test_polar_units_2``
(``matplotlib.testing.jpl_units``, Python unit classes), the locator class
checks of ``test_radial_locator_wrapping`` (its tick locations are probed)
and the warning ``test_set_rticks_ticklabels_no_warning`` watches for (its
labels are probed).
"""

import numpy as np
import matplotlib.ticker as mticker
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mpl_polar")


def _polar():
    fig = Figure()
    return fig, fig.add_subplot(projection="polar")


def _set_adjustable(projection, adjustable):
    ax = Figure().add_subplot(projection=projection)
    ax.set_adjustable(adjustable)
    return ax.get_adjustable()


def test_polar_not_datalim_adjustable():
    r = probes.probe("axes.Axes.set_adjustable", _set_adjustable, projection="polar",
                     adjustable="datalim")
    assert isinstance(r, ValueError)
    probes.probe("axes.Axes.set_adjustable", _set_adjustable, projection="polar",
                 adjustable="box")
    probes.probe("axes.Axes.set_adjustable", _set_adjustable, projection=None,
                 adjustable="datalim")


def _grid(alpha):
    fig, ax = _polar()
    ax.grid(alpha=alpha)
    fig.draw_without_rendering()
    return [ax.xaxis.majorTicks[0].gridline.get_alpha(),
            ax.yaxis.majorTicks[0].gridline.get_alpha()]


def test_polar_gridlines():
    assert probes.probe("axes.Axes.grid", _grid, alpha=0.2) == [0.2, 0.2]


def _spine_extents(theta_direction=None, theta_zero_location=None, theta_offset=None):
    fig, ax = _polar()
    if theta_direction is not None:
        ax.set_theta_direction(theta_direction)
    if theta_zero_location is not None:
        ax.set_theta_zero_location(*theta_zero_location)
    if theta_offset is not None:
        ax.set_theta_offset(theta_offset)
    fig.draw_without_rendering()
    return ax.spines["polar"].get_tightbbox().extents


def test_polar_outer_spine_not_collapsed():
    for kw in [{"theta_direction": -1, "theta_zero_location": ["N", 20]},
               {"theta_direction": -1, "theta_zero_location": ["N", 30]},
               {"theta_offset": 1.570796327}]:
        x0, y0, x1, y1 = probes.probe("spines.Spine.get_tightbbox", _spine_extents, **kw)
        assert min(x1 - x0, y1 - y0) > 1


def _span_steps(xmin, xmax):
    _, ax = _polar()
    return ax.axvspan(xmin, xmax).get_path()._interpolation_steps


def test_axvspan():
    r = probes.probe("axes.Axes.axvspan._interpolation_steps", _span_steps, xmin=0,
                     xmax=np.pi / 4)
    assert r > 1


def _shared_ticklabels(how, projection):
    fig = Figure()
    kw = {"subplot_kw": {"projection": projection}, "sharex": True, "sharey": True}
    if how == "subplots":
        ax = fig.subplots(2, 2, **kw)[0, 1]
    else:
        ax = fig.subplot_mosaic("ab\ncd", **kw)["b"]
    fig.draw_without_rendering()
    return [[t.get_text() for t in axis.get_ticklabels()] for axis in (ax.xaxis, ax.yaxis)]


def test_shared_polar_keeps_ticklabels():
    for how in ["subplots", "mosaic"]:
        for projection in ["polar", None]:
            probes.probe("figure.Figure.subplots.get_ticklabels", _shared_ticklabels, how=how,
                         projection=projection)


def millions(x):
    return '$%1.1fM' % (x * 1e-6)


FORMATTERS = {None: None, "millions": millions}


def _format_coord(theta, r, fmt_xdata, fmt_ydata):
    fig, ax = _polar()
    fig.draw_without_rendering()
    ax.fmt_xdata = FORMATTERS[fmt_xdata]
    ax.fmt_ydata = FORMATTERS[fmt_ydata]
    return ax.format_coord(theta, r)


def test_cursor_precision():
    for theta in [0, 1, 2]:
        for r in [0.005, .1, 1]:
            if (theta, r) != (0, 0.005):
                probes.probe("axes.Axes.format_coord", _format_coord, theta=theta, r=r,
                             fmt_xdata=None, fmt_ydata=None)


def test_custom_fmt_data():
    for fmt_xdata, fmt_ydata, points in [
            (None, "millions", [(12, 2e7), (1234, 2e6), (3, 100)]),
            ("millions", None, [(2e5, 1), (1, .1), (1e6, 0.005)]),
            ("millions", "millions", [(2e6, 2e4 * 3e5), (1e18, 12891328123),
                                      (63**7, 1081968 * 1024)])]:
        for theta, r in points:
            probes.probe("axes.Axes.format_coord", _format_coord, theta=theta, r=r,
                         fmt_xdata=fmt_xdata, fmt_ydata=fmt_ydata)


def _radial_locators():
    fig, ax = _polar()
    steps = [
        lambda: None,
        lambda: ax.yaxis.set_major_locator(mticker.MaxNLocator(3)),
        ax.clear,
        lambda: ax.set_rticks([0, 1, 2, 3]),
        ax.clear,
        lambda: ax.set_rgrids([0, 1, 2, 3]),
        ax.clear,
        lambda: ax.set_yscale("log"),
    ]
    out = []
    for step in steps:
        step()
        out.append([ax.yaxis.isDefault_majloc, ax.yaxis.get_majorticklocs()])
    return out


def test_radial_locator_wrapping():
    probes.probe("axis.RadialAxis.get_majorticklocs", _radial_locators)


def _radial_ticklabels(ticks, labels):
    fig, ax = _polar()
    ax.set_rticks(ticks)
    ax.yaxis.set_ticklabels(labels)
    fig.draw_without_rendering()
    return [t.get_text() for t in ax.yaxis.get_ticklabels()]


def test_set_rticks_ticklabels_no_warning():
    probes.probe("axis.RadialAxis.set_ticklabels", _radial_ticklabels, ticks=[0, 1, 2, 3],
                 labels=["zero", "one", "two", "three"])
    r = probes.probe("axis.RadialAxis.set_ticklabels", _radial_ticklabels, ticks=[0, 1, 2, 3],
                     labels=["zero", "one", "two"])
    assert isinstance(r, ValueError)
