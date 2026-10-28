"""Colorbars, ported from matplotlib's own ``tests/test_colorbar.py``: the
extension patches of inverted colorbars, the parent's anchor, removing a
colorbar, colorbars without a mappable, the label, the norm limits a
colorbar settles on, the placement and shape of colorbar axes, the
dividers, the tickers behind the colorbar properties, the offset text and
title above the extensions, and the options matplotlib rejects.

Area ``mpl_colorbar``. Every op runs in the test style (see
``probes/_mpl_tests.py``): its ``style`` and ``rc`` arguments.

Ops
---
``Colorbar._extend_patches``  {orientation, extend} -> the face colours
    of the extension patches of a colorbar of ``imshow(arange(12))`` in
    viridis with black under and white over colours, once its long axis
    is inverted.
``Axes.get_anchor``  {panchor, layout?} -> the anchor (as fractions) of
    an axes anchored ``"N"`` once a colorbar of its image took space from
    it with ``panchor`` (``false``: left alone), in ``Figure(layout=...)``.
``Colorbar.remove``  {scene, use_gridspec?} -> ``subplot`` and ``nested``
    (an axes on a cell of a grid nested in a cell of another): the extents
    of a scatter's axes before a colorbar, with it and after removing it,
    each after ``subplots_adjust()``; ``constrained``: the extents before
    the colorbar and after removing it, laid out by the constrained
    engine; ``no_figure``: the number of axes before and after removing
    the colorbar of a bare ``ScalarMappable`` drawn into the only axes.
``Axes.remove``  {} -> ``{"axes", "positions"}``: the number of axes
    with a colorbar and after removing the colorbar's axes, and the
    parent's extents before, with and after.
``Colorbar``  {cmap?, norm?, extend?} -> ``{"values", "boundaries",
    "ylim"}`` of ``Colorbar(ax, cmap=..., norm=..., extend=...)``, which
    has no mappable; ``norm`` is ``[name, *args]`` (``LogNorm`` from
    ``vmin, vmax``, ``PowerNorm`` from ``gamma, vmin, vmax``).
``pyplot.colorbar``  {} -> the colorbar of a mappable drawn in no axes,
    which matplotlib rejects.
``Axis.get_minorticklocs``  {} -> the minor ticks of a vertical and of a
    horizontal colorbar (extended both ways) of a mesh with limits 0.4 and
    2.6, under ``x/ytick.minor.visible``.
``Colorbar.vmin``  {} -> ``{"limits", "locators"}``: ``[vmin, vmax]`` of
    the colorbar of ``test_colorbar_renorm``'s image when made, after a
    ``LogNorm`` of the data limits and after a ``LogNorm`` of a thousand
    times the limits; the type of its locator after ``set_ticks([1, 2,
    3])``, after the first ``LogNorm`` and after ``set_ticks`` again.
``Colorbar.outline``  {} -> the edge colour of a colorbar's outline set to
    red, after the mesh took a ``LogNorm`` and then a ``Normalize``.
``ScalarMappable.norm``  {} -> ``[vmin, vmax]`` of an unscaled
    ``LogNorm`` given to a mesh with a colorbar.
``Colorbar.alpha``  {} -> ``{"colorbar", "mappable"}``: the colorbar's
    alpha (``null``) and the mesh's alphas, given per cell.
``Colorbar.set_label``  {} -> ``test_colorbar_label``'s labels: the y
    label after ``label="cbar"``, ``set_label(None)`` and
    ``set_label("cbar 2")``, of a colorbar made with ``label=None``, and
    the x label of a horizontal one.
``Figure.colorbar``  {scene, ...} -> ``clim`` {clim}: ``[vmin, vmax]`` of
    the norm of ``imshow([clim])`` once it has a colorbar; ``centered``:
    the same for a mesh of zeros with a ``CenteredNorm``; ``format``
    {format}: ``{"formatter", "labels"}``, the type of the formatter and
    the tick labels of the colorbar of ``imshow([[0, 1]])`` made with
    ``format``, after a draw; ``error`` {location?, orientation?,
    ticklocation?, extendfrac?, cax?}: the colorbar of an image made with
    these options (``cax``: drawn into an inset axes), which matplotlib
    rejects; a missing ``extendfrac`` entry is ``null``.
``Axes.get_position``  {scene, ...} -> ``anchored`` {location, anchor,
    shrink}: the extents of the parent axes and of the colorbar axes made
    in its grid cell, before drawing; ``aspects``: the extents of
    ``test_aspects``'s six colorbar axes after a draw; ``inset``: the bounds
    of a colorbar drawn into an inset axes of a constrained figure after a
    draw, and whether the inset is still a child of its axes.
``Colorbar.dividers``  {orientation, extend, coloroffset} -> the segments
    of the dividers of ``test_colorbar_extend_drawedges``'s colorbar with
    ``drawedges=True`` once its view is narrowed to 1.1 .. 2.9.
``Axes.get_yticks``  {} -> ``{"ylim", "yticks"}`` of each colorbar of
    ``test_negative_boundarynorm``'s ``BoundaryNorm``s of consecutive
    integers, drawn in turn into one axes.
``Colorbar.locator``  {} -> ``test_colorbar_set_formatter_locator``: for
    the major and minor locators and formatters set on the long axis,
    whether the colorbar property is the axis' one (``same``) and its type
    and, for locators, ticks (``axis``); the same read from the axis once
    set through the colorbar (``colorbar``); and the long axis' name.
``YAxis.offsetText``  {} -> ``{"offset", "top"}``: the extent (pixels)
    of the offset text of a colorbar extended at the top, after a draw,
    and the top of its parent axes.
``Text.get_window_extent``  {} -> ``[title, outline]``: the top (pixels)
    of the title of a colorbar extended at the top and the top of its
    outline, after a draw.

Cases
-----
Named after the upstream tests. Adaptations: pyplot's current axes and
image become the ones named; the int16 images of ``test_colorbar_int``
hold their values as floats (plt-rs's images are float, so nothing can
overflow); the parentless ``PatchCollection`` of
``test_parentless_mappable`` is a bare ``ScalarMappable``, plt-rs's
mappable drawn in no axes; ``test_axes_remove`` (a mock) records what
removing the colorbar's axes does; ``test_offset_text_loc`` records the
offset text's extent rather than its anchor; the probes record the values
the tests assert on (``_values`` rather than its sign, extents rather than
relations between them); type and identity checks become type names and
``is`` comparisons; a ``None`` in ``extendfrac`` is NaN, as numpy reads
it; random data comes from ``spec_support.rng``.

Covered elsewhere: the image tests (figure groups ``mpl_polar`` and
``colorbar``), the tick and scale scenarios (area
``colorbar_behaviour``); the smoke tests that only draw are figure cases
of the ``mpl_tests_colorbar`` group. Not ported: the warnings of
``test_colorbar_wrong_figure`` and ``test_colorbar_no_warning_rcparams_grid_true``
(plt-rs does not warn; their figures are cases).
"""

import matplotlib as mpl
import matplotlib.colors as mcolors
import matplotlib.pyplot as plt
import matplotlib.transforms as mtransforms
import numpy as np
from matplotlib import cm
from matplotlib.colorbar import Colorbar
from matplotlib.ticker import FixedLocator, Locator, LogFormatter

from probes._mpl_tests import prober
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_colorbar")
probe = prober(probes)

#: Extents in pixels or figure fractions, compared to a millionth.
EXTENT_TOL = 1e-6


def _extents(bbox):
    return [float(v) for v in bbox.extents]


def _rgba(color):
    return [float(v) for v in mcolors.to_rgba(color)]


def _floats(values):
    return [float(v) for v in values]


def _bump():
    x, y = np.ogrid[-4:4:31j, -4:4:31j]
    return 120000 * np.exp(-x**2 - y**2)


def _extend_patches(orientation, extend):
    fig, ax = plt.subplots()
    cmap = mpl.colormaps["viridis"].with_extremes(under=(0, 0, 0, 1), over=(1, 1, 1, 1))
    im = ax.imshow(np.arange(12).reshape(3, 4), cmap=cmap)
    cbar = fig.colorbar(im, orientation=orientation, extend=extend)
    if orientation == "horizontal":
        cbar.ax.invert_xaxis()
    else:
        cbar.ax.invert_yaxis()
    return [_rgba(patch.get_facecolor()) for patch in cbar._extend_patches]


def test_colorbar_extension_inverted_axis():
    for orientation in ["horizontal", "vertical"]:
        for extend in ["min", "max", "both"]:
            probe("Colorbar._extend_patches", _extend_patches,
                  orientation=orientation, extend=extend)


def _anchor(panchor, layout=None):
    fig = plt.figure(layout=layout)
    ax = fig.add_subplot(111, anchor="N")
    im = ax.imshow([[0, 1]])
    fig.colorbar(im, panchor=panchor)
    anchor = ax.get_anchor()
    return _floats(mtransforms.Bbox.coefs[anchor] if isinstance(anchor, str) else anchor)


def test_colorbar_single_ax_panchor_false():
    probe("Axes.get_anchor", _anchor, panchor=False)


def test_colorbar_single_ax_panchor_east():
    for layout in [None, "constrained"]:
        probe("Axes.get_anchor", _anchor, panchor="E", layout=layout)


def _remove(scene, use_gridspec=None):
    if scene == "no_figure":
        fig, ax = plt.subplots()
        before = len(fig.get_axes())
        cb = fig.colorbar(cm.ScalarMappable(), cax=ax)
        cb.remove()
        return [before, len(fig.get_axes())]
    if scene == "constrained":
        fig, ax = plt.subplots(constrained_layout=True)
        sc = ax.scatter([1, 2], [3, 4])
        sc.set_array(np.array([5, 6]))
        fig.draw_without_rendering()
        pre = _extents(ax.get_position())
        cb = fig.colorbar(sc)
        cb.remove()
        fig.draw_without_rendering()
        return [pre, _extents(ax.get_position())]
    fig = plt.figure()
    if scene == "nested":
        gs = fig.add_gridspec(2, 2)[1, 1].subgridspec(2, 2)
        ax = fig.add_subplot(gs[1, 1])
    else:
        ax = fig.add_subplot()
    sc = ax.scatter([1, 2], [3, 4])
    sc.set_array(np.array([5, 6]))
    positions = [_extents(ax.get_position())]
    cb = fig.colorbar(sc, use_gridspec=use_gridspec)
    fig.subplots_adjust()
    positions.append(_extents(ax.get_position()))
    cb.remove()
    fig.subplots_adjust()
    positions.append(_extents(ax.get_position()))
    return positions


def test_remove_from_figure():
    for scene in ["subplot", "nested"]:
        for use_gridspec in [True, False]:
            probe("Colorbar.remove", _remove, scene=scene, use_gridspec=use_gridspec,
                  tol=EXTENT_TOL)


def test_remove_from_figure_cl():
    probe("Colorbar.remove", _remove, scene="constrained", tol=EXTENT_TOL)


def test_remove_cb_whose_mappable_has_no_figure():
    probe("Colorbar.remove", _remove, scene="no_figure")


def _axes_remove():
    fig, ax = plt.subplots()
    sc = ax.scatter([1, 2], [3, 4])
    positions = [_extents(ax.get_position())]
    cb = fig.colorbar(sc)
    positions.append(_extents(ax.get_position()))
    counts = [len(fig.axes)]
    cb.ax.remove()
    counts.append(len(fig.axes))
    positions.append(_extents(ax.get_position()))
    return {"axes": counts, "positions": positions}


def test_axes_remove():
    probe("Axes.remove", _axes_remove, tol=EXTENT_TOL)


NORMS = {"LogNorm": mcolors.LogNorm, "PowerNorm": mcolors.PowerNorm}


def _bare(cmap=None, norm=None, extend=None):
    fig, ax = plt.subplots()
    kwargs = {}
    if cmap is not None:
        kwargs["cmap"] = plt.colormaps[cmap]
    if norm is not None:
        name, *args = norm
        kwargs["norm"] = NORMS[name](*args)
    cb = Colorbar(ax, orientation="vertical", extend=extend, **kwargs)
    return {"values": _floats(cb._values), "boundaries": _floats(cb._boundaries),
            "ylim": _floats(ax.get_ylim())}


def test_colorbarbase():
    probe("Colorbar", _bare, cmap="bone")


def test_colorbar_lognorm_extension():
    for extend in ["both", "min", "max"]:
        probe("Colorbar", _bare, norm=["LogNorm", 0.1, 1000.0], extend=extend)


def test_colorbar_powernorm_extension():
    probe("Colorbar", _bare, norm=["PowerNorm", 0.5, 0.0, 1.0], extend="both")


def _parentless():
    plt.figure()
    plt.colorbar(cm.ScalarMappable(cmap="viridis"))


def test_parentless_mappable():
    probe("pyplot.colorbar", _parentless)


def _minor_ticks():
    fig, ax = plt.subplots()
    im = ax.pcolormesh([[1, 2]], vmin=0.4, vmax=2.6)
    vertical = fig.colorbar(im, extend="both", orientation="vertical")
    horizontal = fig.colorbar(im, extend="both", orientation="horizontal")
    return [_floats(vertical.ax.yaxis.get_minorticklocs()),
            _floats(horizontal.ax.xaxis.get_minorticklocs())]


def test_cbar_minorticks_for_rc_xyminortickvisible():
    probe("Axis.get_minorticklocs", _minor_ticks,
          rc={"ytick.minor.visible": True, "xtick.minor.visible": True})


def _renorm():
    z = _bump()
    fig, ax = plt.subplots()
    im = ax.imshow(z)
    cbar = fig.colorbar(im)
    limits = [[cbar.vmin, cbar.vmax]]
    cbar.set_ticks([1, 2, 3])
    locators = [type(cbar.locator).__name__]
    im.set_norm(mcolors.LogNorm(z.min(), z.max()))
    limits.append([cbar.vmin, cbar.vmax])
    locators.append(type(cbar.locator).__name__)
    cbar.set_ticks([1, 2, 3])
    locators.append(type(cbar.locator).__name__)
    im.set_norm(mcolors.LogNorm(z.min() * 1000, z.max() * 1000))
    limits.append([cbar.vmin, cbar.vmax])
    return {"limits": [_floats(pair) for pair in limits], "locators": locators}


def test_colorbar_renorm():
    probe("Colorbar.vmin", _renorm)


def _outline():
    fig, ax = plt.subplots()
    pcm = ax.pcolormesh(_bump(), cmap="RdBu_r", rasterized=True)
    cbar = fig.colorbar(pcm, ax=ax)
    cbar.outline.set_edgecolor("red")
    pcm.set_norm(mcolors.LogNorm(vmin=1, vmax=100))
    pcm.set_norm(mcolors.Normalize(vmin=-20, vmax=20))
    return _rgba(cbar.outline.get_edgecolor())


def _assigned_norm():
    fig, ax = plt.subplots()
    pcm = ax.pcolormesh(_bump(), cmap="RdBu_r", rasterized=True)
    fig.colorbar(pcm, ax=ax)
    pcm.norm = mcolors.LogNorm()
    return _floats([pcm.norm.vmin, pcm.norm.vmax])


def test_colorbar_scale_reset():
    probe("Colorbar.outline", _outline)
    probe("ScalarMappable.norm", _assigned_norm)


def _alpha():
    fig, ax = plt.subplots()
    x = np.arange(1, 5).reshape(2, 2) / 4
    pc = ax.pcolormesh(x, alpha=x)
    cb = fig.colorbar(pc, ax=ax)
    fig.draw_without_rendering()
    return {"colorbar": cb.alpha, "mappable": _floats(np.ravel(pc.get_alpha()))}


def test_mappable_2d_alpha():
    probe("Colorbar.alpha", _alpha)


def _labels():
    fig, ax = plt.subplots()
    im = ax.imshow([[1, 2], [3, 4]])
    cbar = fig.colorbar(im, label="cbar")
    labels = [cbar.ax.get_ylabel()]
    cbar.set_label(None)
    labels.append(cbar.ax.get_ylabel())
    cbar.set_label("cbar 2")
    labels.append(cbar.ax.get_ylabel())
    labels.append(fig.colorbar(im, label=None).ax.get_ylabel())
    labels.append(fig.colorbar(im, orientation="horizontal", label="horizontal cbar")
                  .ax.get_xlabel())
    return labels


def test_colorbar_label():
    probe("Colorbar.set_label", _labels)


def _make(scene, clim=None, format=None, cax=False, extendfrac=None, **kwargs):
    if scene == "clim":
        fig, ax = plt.subplots()
        im = ax.imshow([[*map(np.int16, clim)]])
        fig.colorbar(im)
        return _floats([im.norm.vmin, im.norm.vmax])
    if scene == "centered":
        fig, ax = plt.subplots(figsize=(1, 3))
        norm = mcolors.CenteredNorm()
        fig.colorbar(ax.pcolormesh(np.zeros((3, 3)), norm=norm))
        return _floats([norm.vmin, norm.vmax])
    if scene == "format":
        fig, ax = plt.subplots()
        cb = fig.colorbar(ax.imshow([[0, 1]]), format=format)
        fig.draw_without_rendering()
        return {"formatter": type(cb.formatter).__name__,
                "labels": [label.get_text() for label in cb.ax.yaxis.get_ticklabels()]}
    if scene == "error":
        fig, ax = plt.subplots()
        im = ax.imshow([[0, 1], [2, 3]])
        if cax:
            kwargs["cax"] = ax.inset_axes([0, 1.05, 1, 0.05])
        if extendfrac is not None:
            kwargs["extendfrac"] = tuple(extendfrac)
        fig.colorbar(im, **kwargs)
        return None
    raise ValueError(scene)


def test_colorbar_int():
    for clim in [[-20000, 20000], [-32768, 0]]:
        probe("Figure.colorbar", _make, scene="clim", clim=clim)


def test_centerednorm():
    probe("Figure.colorbar", _make, scene="centered")


def test_colorbar_format_string_and_old():
    probe("Figure.colorbar", _make, scene="format", format="{x}%")


def test_colorbar_errors():
    for kwargs in [{"location": "top", "orientation": "vertical"},
                   {"location": "top", "orientation": "vertical", "cax": True},
                   {"ticklocation": "top", "orientation": "vertical", "cax": True},
                   {"location": "top", "extendfrac": [0, None]}]:
        probe("Figure.colorbar", _make, scene="error", **kwargs)


RAMP = np.arange(1200).reshape(30, 40)
RAMP_LEVELS = [0, 200, 400, 600, 800, 1000, 1200]


def _position(scene, location=None, anchor=None, shrink=None):
    if scene == "anchored":
        fig, ax = plt.subplots()
        cs = ax.contourf(RAMP, levels=RAMP_LEVELS)
        cbar = fig.colorbar(cs, ax=ax, use_gridspec=True, location=location,
                            anchor=tuple(anchor), shrink=shrink)
        return [_extents(ax.get_position()), _extents(cbar.ax.get_position())]
    if scene == "aspects":
        fig, ax = plt.subplots(3, 2, figsize=(8, 8))
        colorbars = []
        for nn, orient in enumerate(["vertical", "horizontal"]):
            for mm, (aspect, extend) in enumerate(zip([20, 20, 10], ["neither", "both", "both"])):
                pc = ax[mm, nn].pcolormesh(np.arange(100).reshape(10, 10))
                colorbars.append(fig.colorbar(pc, ax=ax[mm, nn], orientation=orient,
                                              aspect=aspect, extend=extend))
        fig.draw_without_rendering()
        return [_extents(cb.ax.get_position()) for cb in colorbars]
    if scene == "inset":
        fig, ax = plt.subplots(constrained_layout=True, figsize=(3, 6))
        pc = ax.imshow(np.arange(100).reshape(10, 10))
        cax = ax.inset_axes([1.02, 0.1, 0.03, 0.8])
        cb = fig.colorbar(pc, cax=cax)
        fig.draw_without_rendering()
        return {"bounds": _floats(cb.ax.get_position().bounds),
                "child": any(child is cb.ax for child in ax.child_axes)}
    raise ValueError(scene)


def test_anchored_cbar_position_using_specgrid():
    for location, anchor in [("right", [1, 0.3]), ("left", [1, 0.3]),
                             ("top", [0.3, 1]), ("bottom", [0.3, 1])]:
        probe("Axes.get_position", _position, scene="anchored", location=location,
              anchor=anchor, shrink=0.5, tol=EXTENT_TOL)


def test_aspects():
    probe("Axes.get_position", _position, scene="aspects", tol=EXTENT_TOL)


def test_inset_colorbar_layout():
    probe("Axes.get_position", _position, scene="inset", tol=EXTENT_TOL)


def _dividers(orientation, extend, coloroffset):
    fig, ax = plt.subplots()
    bounds = np.arange(5)
    colors = mpl.colormaps["viridis"](
        np.linspace(100, 255, len(bounds) + coloroffset).astype(int))
    cmap, norm = mcolors.from_levels_and_colors(bounds, colors, extend=extend)
    cbar = Colorbar(ax, cmap=cmap, norm=norm, orientation=orientation, drawedges=True)
    if orientation == "horizontal":
        ax.set_xlim(1.1, 2.9)
    else:
        ax.set_ylim(1.1, 2.9)
    return np.asarray(cbar.dividers.get_segments(), dtype=float).tolist()


def test_colorbar_extend_drawedges():
    for orientation in ["horizontal", "vertical"]:
        for extend, coloroffset in [("both", 1), ("min", 0), ("max", 0), ("neither", -1)]:
            probe("Colorbar.dividers", _dividers, orientation=orientation, extend=extend,
                  coloroffset=coloroffset)


def _boundary_norms():
    fig, ax = plt.subplots(figsize=(1, 3))
    cmap = mpl.colormaps["viridis"]
    out = []
    for start, stop in [(-94, -85), (85, 94), (-3, 3), (-8, 1)]:
        clevs = np.arange(start, stop)
        norm = mcolors.BoundaryNorm(clevs, cmap.N)
        cb = fig.colorbar(cm.ScalarMappable(cmap=cmap, norm=norm), cax=ax)
        out.append({"ylim": _floats(cb.ax.get_ylim()), "yticks": _floats(cb.ax.get_yticks())})
    return out


def test_negative_boundarynorm():
    probe("Axes.get_yticks", _boundary_norms)


def _ticker(ticker):
    """The type of a locator or formatter, and a locator's ticks."""
    name = type(ticker).__name__
    return [name, _floats(ticker())] if isinstance(ticker, Locator) else [name]


def _tickers():
    fig, ax = plt.subplots()
    pc = ax.pcolormesh(rng(0).normal(100).reshape(10, 10))
    cb = fig.colorbar(pc)
    axis = cb.ax.yaxis
    axis.set_major_locator(FixedLocator(np.arange(10)))
    axis.set_minor_locator(FixedLocator(np.arange(0, 10, 0.2)))
    axis.set_major_formatter(LogFormatter())
    axis.set_minor_formatter(LogFormatter())
    properties = {"locator": cb.locator, "minorlocator": cb.minorlocator,
                  "formatter": cb.formatter, "minorformatter": cb.minorformatter}
    axis_tickers = {"locator": axis.get_major_locator(), "minorlocator": axis.get_minor_locator(),
                    "formatter": axis.get_major_formatter(),
                    "minorformatter": axis.get_minor_formatter()}
    same = {name: ticker is axis_tickers[name] for name, ticker in properties.items()}
    through_axis = {name: _ticker(ticker) for name, ticker in properties.items()}
    cb.locator = FixedLocator(np.arange(7))
    cb.minorlocator = FixedLocator(np.arange(0, 7, 0.1))
    cb.formatter = LogFormatter()
    cb.minorformatter = LogFormatter()
    through_colorbar = {
        "locator": _ticker(axis.get_major_locator()),
        "minorlocator": _ticker(axis.get_minor_locator()),
        "formatter": _ticker(axis.get_major_formatter()),
        "minorformatter": _ticker(axis.get_minor_formatter()),
    }
    return {"same": same, "axis": through_axis, "colorbar": through_colorbar,
            "long_axis": "y" if cb.long_axis is cb.ax.yaxis else "x"}


def test_colorbar_set_formatter_locator():
    probe("Colorbar.locator", _tickers)


def _offset_text():
    fig, ax = plt.subplots()
    pc = ax.pcolormesh(rng(19680808).normal(100).reshape(10, 10) * 1e6)
    cb = fig.colorbar(pc, location="right", extend="max")
    fig.draw_without_rendering()
    return {"offset": _extents(cb.ax.yaxis.offsetText.get_window_extent()),
            "top": float(ax.bbox.y1)}


def test_offset_text_loc():
    probe("YAxis.offsetText", _offset_text, style="mpl20", tol=EXTENT_TOL)


def _title():
    fig, ax = plt.subplots()
    pc = ax.pcolormesh(rng(19680808).normal(100).reshape(10, 10))
    cb = fig.colorbar(pc, location="right", extend="max")
    cb.ax.set_title("Aardvark")
    fig.draw_without_rendering()
    return _floats([cb.ax.title.get_window_extent(fig.canvas.get_renderer()).ymax,
                    cb.ax.spines["outline"].get_window_extent().ymax])


def test_title_text_loc():
    probe("Text.get_window_extent", _title, style="mpl20", tol=EXTENT_TOL)
