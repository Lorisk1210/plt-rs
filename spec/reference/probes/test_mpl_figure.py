"""Figures, ported from matplotlib's own ``tests/test_figure.py``: pyplot's
figure numbers and labels, the figure-level texts, sizes and resolution,
layout engines, tight boxes, subfigures, mosaics, clearing and saving.

Area ``mpl_figure``. Every op runs in the test style (see
``probes/_mpl_tests.py``): its ``style`` and ``rc`` arguments.

Ops
---
``pyplot.figure``  {steps} -> one entry per step (``null`` for actions)
    of a pyplot session that starts with no figures. Steps:
    ``["figure", num_or_label?, {"clear"?, "figsize"?, "dpi"?}?]``,
    ``["figure_of", k]`` (``figure(fig)`` with the figure step ``k`` made),
    ``["close", num_or_label?]``, ``["subplots", nrows, ncols, {"num",
    "clear"}]``, ``["suptitle", text]`` and ``["text", x, y, text]`` on the
    current figure, and the queries ``["get_fignums"]``,
    ``["get_figlabels"]``, ``["fignum_exists", num_or_label]``,
    ``["gcf_is", k]`` (whether the current figure is the one step ``k``
    made), ``["texts"]`` (the current figure's texts) and
    ``["get_size_inches"]``.
``Figure.align_labels``  {layout?, seed?} -> ``{"ylabel_x0": [...],
    "xlabel_y0": [...]}``: the left edge of every y label and the bottom
    edge of every x label (pixels) of ``test_align_labels_stray_axes``'s
    2 x 2 grid after ``align_ylabels``, ``align_xlabels`` and a draw;
    with ``layout="constrained"`` the axes hold meshes of
    ``rng(seed).normal(100)`` and the last one a colorbar.
``Figure.gca``  {} -> the axes ``gca`` names (numbered in creation order)
    after ``add_axes``, ``add_subplot(111)``, re-adding the first axes and
    ``sca``, and the axes order.
``Figure.suptitle``  {fontproperties} -> the title's font size and
    numeric weight when made with ``FontProperties(**fontproperties)``.
``SubFigure.get_facecolor``  {} -> the two facecolours of
    ``test_suptitle_subfigures``.
``Figure.get_suptitle``  {} -> ``[suptitle, supxlabel, supylabel]`` of a
    new figure, then after setting each in turn.
``Text.remove``  {} -> the number of figure texts after making the three
    figure-level texts and after removing each, and the getters after.
``Figure.set_size_inches``  {figsize?, calls} -> the size in inches after
    each call ``[method, *args]`` (``set_figwidth``, ``set_figheight``,
    ``set_size_inches``) on ``plt.figure(figsize=figsize)``.
``pyplot.figaspect``  {arg?, shape?} -> ``[width, height]`` for a ratio or
    an array of that shape.
``Figure.autofmt_xdate``  {which?, colorbar?} -> the rotations of the x
    tick labels: ``test_autofmt_xdate``'s date plot with minor ticks (the
    distinct rotations of the major and of the minor labels), or, with
    ``colorbar``, the constrained image with a colorbar of
    ``test_autofmt_xdate_colorbar_constrained`` (the second major label
    after a draw).
``Figure.set_dpi``  {} -> the renderer's size at 100 and then 50 dpi.
``Figure.subplots``  {} -> whether each axes of the shared log grid of
    ``test_subplots_shareax_loglabels`` shows x and y tick labels.
``Figure.savefig``  {format, metadata?} -> whether the first 100 bytes of
    the file saved to a path name the format (``test_fspath``), or
    ``true`` once saved with empty metadata.
``FigureCanvasBase._set_device_pixel_ratio``  {ratio, dpi?} -> {"changed",
    "device_pixel_ratio", "dpi", "saved"}: for ``plt.subplots()`` with a
    line, after ``set_dpi(dpi)`` when given and then
    ``canvas._set_device_pixel_ratio(ratio)``: whether the ratio changed,
    the canvas's ratio, the figure's dpi and the ``[width, height]`` of the
    PNG ``savefig`` writes.
``Figure.get_layout_engine``  {layout?, calls?} -> the engine's name
    (``"none"``, ``"tight"``, ``"constrained"``, ``"compressed"``; a
    placeholder engine is ``"none"``) of ``Figure(layout=layout)`` after
    each call: ``["set_layout_engine", name]`` (``"none"`` included),
    ``["tight_layout", {kwargs}]`` (a 1 x 2 grid made first),
    ``["savefig_tight"]``, ``["colorbar"]`` (a subplot holding
    ``pcolormesh([[1, 2], [3, 4]])`` and its colorbar). A call the figure
    refuses makes the whole op raise.
``Figure.get_constrained_layout_pads``  {params} -> the pads of a
    figure laid out by ``ConstrainedLayoutEngine(**params)``.
``Figure.get_tightbbox``  {} -> the tight boxes of
    ``test_tightbbox``: the text's right edge and the axes' (pixels), the
    figure's edges (inches), then with the text out of the layout and back.
``SubFigure.get_tightbbox``  {} -> the width of the tight box of a
    constrained figure of two empty subfigures.
``Axes.remove``  {} -> the number of axes left after removing the first
    of two axes sharing x, once drawn.
``Axis.get_major_formatter``  {set_scalar} -> the class name of the x
    major formatter of the first of two axes sharing x once the second is
    removed (after giving it a ``ScalarFormatter`` with ``set_scalar``)
    and dates are plotted on the first.
``Figure.clear``  {method} -> ``test_figure_clear``'s subfigure scenarios:
    after each step, the number of axes of the figure, of its subfigures
    and whether the axes made are still listed.
``Figure.pick``  {} -> whether the figure is stale after a draw and after
    a pick at the centre of a scatter.
``Figure.subplot_mosaic``  {mosaic, width_ratios?, height_ratios?,
    sharex?, sharey?, per_subplot_kw?, set_scales?} -> the labels in the
    order the mosaic lists them, the labels of ``fig.axes`` in order, the
    width and height ratios of each axes' grid, and with ``set_scales`` the
    scales of every axes after setting them on ``"A"``. ``per_subplot_kw``
    is a dict, or a list of ``[key, kwargs]`` pairs whose key is a label or
    a list of labels (a tuple key).
``SubplotSpec.get_gridspec``  {} -> whether the axes of
    ``test_reused_gridspec`` share one grid.
``SubFigure.set_dpi``  {} -> a subfigure's and its figure's dpi before and
    after setting the subfigure's.
``SubFigure.bbox``  {nrows, ncols, hspace?, wspace?, specs?, layout?} ->
    the extents (pixels) of ``subfigures(nrows, ncols, ...)`` or of
    ``add_subfigure`` on the grid cells ``specs`` ``[[r0, r1], [c0,
    c1]]``.
``Axes.get_xticks``  {} -> the x ticks of ``test_subfigure_ticks`` at 120
    and at 300 dpi.
``SubFigure.remove``  {} -> the number of subfigures left after removing
    one of a 2 x 2 grid, and whether ``subfigs`` lists a 4 x 3 grid in row
    order (``test_subfigure_row_order``).
``Figure.add_subplot``  {projections} -> for two ``add_subplot(1, 1, 1)``
    with these projections: whether they are distinct and their names.
``Figure.add_axes``  {projections} -> the same for ``add_axes``.
``Figure.get_label``  {} -> the labels of a figure and of its subfigure.
``Figure.set_visible``  {} -> whether the SVG of an empty figure draws
    a path (its background) before and after hiding the figure.
``Figure.stale``  {} -> the staleness of a figure and its nested
    subfigures along ``test_subfigure_stale_propagation``.
``Figure.colorbar``  {} -> a colorbar axes' original and actual positions
    after saving the region ``[[0, 0], [4, 4]]`` (inches).

Cases
-----
Named after the upstream tests. Adaptations: ``add_axes(ax)`` of an
existing axes is ``sca(ax)`` (what it does to an axes of the figure);
``test_align_labels_stray_axes`` sets the y labels its second
``set_xlabel`` evidently meant, and compares label extents rather than
positions; ``test_autofmt_xdate`` gives its times of day a date
(``datestr2num`` of a bare time takes today's); ``test_not_visible_figure``
looks for the background's path instead of a group (plt-rs's SVG has no
group for the figure itself); ``test_extra_per_subplot_kw`` asks for a
polar projection of the missing key, the per-subplot setting plt-rs
takes; ``test_savefig_pixel_ratio`` also runs in ``mpl20`` (the test style
saves at 100 dpi whatever the figure's resolution) and after ``set_dpi``;
random data comes from ``spec_support.rng``.

Not ported: Python calling conventions (``TypeError`` for argument counts,
``savefig`` keywords, ``axes_class``, ``projection=42``), deprecations
(setting ``Figure.number``, ``set_figure``), ``_as_mpl_axes``, the
``clf`` override check, ``repr``, ``deepcopy``, pickling, the
``max_open_warning`` warning, ``ginput`` and ``waitforbuttonpress``
(blocking GUI input), backends given to ``savefig``, warnings for
colorbars of another figure, and ``gridspec_kw`` dicts left unchanged.
"""

import datetime
import io
import pathlib
import tempfile
from types import SimpleNamespace

import matplotlib as mpl
import matplotlib.dates as mdates
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.figure import Figure
from matplotlib.font_manager import FontProperties, weight_dict
from matplotlib.layout_engine import (
    ConstrainedLayoutEngine,
    PlaceHolderLayoutEngine,
    TightLayoutEngine,
)
from matplotlib.ticker import AutoMinorLocator, FixedFormatter, ScalarFormatter

from probes._mpl_tests import prober
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_figure")
probe = prober(probes)


def _pyplot(steps):
    plt.close("all")
    made, results = [], []
    for step in steps:
        name, *args = step
        kwargs = args.pop() if args and isinstance(args[-1], dict) else {}
        result, obj = None, None
        if name == "figure":
            if "figsize" in kwargs:
                kwargs["figsize"] = tuple(kwargs["figsize"])
            obj = plt.figure(*args, **kwargs)
        elif name == "figure_of":
            obj = plt.figure(made[args[0]])
        elif name == "close":
            plt.close(*args)
        elif name == "subplots":
            obj, _ = plt.subplots(*args, **kwargs)
        elif name == "suptitle":
            plt.gcf().suptitle(*args)
        elif name == "text":
            plt.gcf().text(*args)
        elif name == "get_fignums":
            result = plt.get_fignums()
        elif name == "get_figlabels":
            result = plt.get_figlabels()
        elif name == "fignum_exists":
            result = plt.fignum_exists(*args)
        elif name == "gcf_is":
            result = plt.gcf() is made[args[0]]
        elif name == "texts":
            result = [t.get_text() for t in plt.gcf().texts]
        elif name == "get_size_inches":
            result = [float(v) for v in plt.gcf().get_size_inches()]
        else:
            raise ValueError(name)
        made.append(obj)
        results.append(result)
    return results


def test_figure_label():
    probe("pyplot.figure", _pyplot, steps=[
        ["figure", "today"], ["figure", 3], ["figure", "tomorrow"], ["figure"], ["figure", 0],
        ["figure", 1], ["figure", 3], ["get_fignums"], ["get_figlabels"], ["close", 10],
        ["close"], ["close", 5], ["close", "tomorrow"], ["get_fignums"], ["get_figlabels"],
        ["figure_of", 0], ["gcf_is", 0],
    ])


def test_fignum_exists():
    probe("pyplot.figure", _pyplot, steps=[
        ["figure", "one"], ["figure", 2], ["figure", "three"], ["figure"],
        ["fignum_exists", "one"], ["fignum_exists", 2], ["fignum_exists", "three"],
        ["fignum_exists", 4], ["close", "one"], ["close", 4], ["fignum_exists", "one"],
        ["fignum_exists", 4],
    ])


def test_clf_keyword():
    probe("pyplot.figure", _pyplot, steps=[
        ["figure", {"num": 1}], ["suptitle", "A fancy plot"], ["texts"],
        ["figure", {"num": 1, "clear": False}], ["text", 0.5, 0.5, "Really fancy!"],
        ["gcf_is", 0], ["texts"], ["subplots", 2, 1, {"num": 1, "clear": True}],
        ["gcf_is", 0], ["texts"],
    ])


def _figure_size(*figsizes):
    for figsize in figsizes:
        probe("pyplot.figure", _pyplot,
              steps=[["figure", {"figsize": figsize, "dpi": 100}], ["get_size_inches"]])


def test_figsize():
    _figure_size([6, 4], [6, 4, "in"], [5.08, 2.54, "cm"], [600, 400, "px"], [1, float("nan")],
                 [-1, 1], [float("inf"), 1])


def test_figsize_partial_none():
    _figure_size([None, 4], [6, None])


def test_figsize_both_none():
    _figure_size([None, None])


def test_figsize_invalid_unit():
    _figure_size([6, 4, "um"])


def _align_labels(layout=None, seed=None):
    fig, axs = plt.subplots(2, 2, layout=layout)
    for nn, ax in enumerate(axs.flat):
        ax.set_xlabel('Boo')
        ax.set_ylabel('Who')
        if layout is None:
            ax.plot(np.arange(4) ** nn, np.arange(4) ** nn)
        else:
            pc = ax.pcolormesh(rng(seed + nn).normal(100).reshape(10, 10))
    if layout is not None:
        fig.colorbar(pc, ax=ax)
    fig.align_ylabels()
    fig.align_xlabels()
    fig.draw_without_rendering()
    return {"ylabel_x0": [ax.yaxis.label.get_window_extent().x0 for ax in axs.flat],
            "xlabel_y0": [ax.xaxis.label.get_window_extent().y0 for ax in axs.flat]}


def test_align_labels_stray_axes():
    probe("Figure.align_labels", _align_labels, tol=1e-9)
    probe("Figure.align_labels", _align_labels, layout="constrained", seed=19680801, tol=1e-6)


def _gca():
    fig = plt.figure()
    numbered = []

    def number(ax):
        return next(i for i, a in enumerate(numbered) if a is ax)

    ax0 = fig.add_axes((0, 0, 1, 1))
    numbered.append(ax0)
    after_add_axes = number(fig.gca())
    ax1 = fig.add_subplot(111)
    numbered.append(ax1)
    after_add_subplot = number(fig.gca())
    fig.add_axes(ax0)
    after_readd = number(fig.gca())
    fig.sca(ax0)
    order = [number(ax) for ax in fig.axes]
    fig.add_subplot(ax1)
    return {"after_add_axes": after_add_axes, "after_add_subplot": after_add_subplot,
            "after_readd": after_readd, "order": order, "last": number(fig.gca())}


def test_gca():
    probe("Figure.gca", _gca)


def _suptitle(fontproperties):
    fig, ax = plt.subplots()
    txt = fig.suptitle('fontprops title', fontproperties=FontProperties(**fontproperties))
    weight = txt.get_weight()
    return {"fontsize": txt.get_fontsize(), "weight": weight_dict.get(weight, weight)}


def test_suptitle_fontproperties():
    probe("Figure.suptitle", _suptitle, fontproperties={"size": "large", "weight": "bold"})


def _subfigure_facecolors():
    fig = plt.figure(figsize=(4, 3))
    sf1, sf2 = fig.subfigures(1, 2)
    sf2.set_facecolor('white')
    sf1.subplots()
    sf2.subplots()
    fig.suptitle("This is a visible suptitle.")
    return [list(sf1.get_facecolor()), list(sf2.get_facecolor())]


def test_suptitle_subfigures():
    probe("SubFigure.get_facecolor", _subfigure_facecolors)


def _get_suptitles():
    fig, ax = plt.subplots()

    def getters():
        return [fig.get_suptitle(), fig.get_supxlabel(), fig.get_supylabel()]

    out = [getters()]
    for method, text in [("suptitle", "suptitle"), ("supxlabel", "supxlabel"),
                         ("supylabel", "supylabel")]:
        getattr(fig, method)(text)
        out.append(getters())
    return out


def test_get_suptitle_supxlabel_supylabel():
    probe("Figure.get_suptitle", _get_suptitles)


def _remove_suptitles():
    fig = plt.figure()
    texts = [fig.suptitle('suptitle'), fig.supxlabel('supxlabel'), fig.supylabel('supylabel')]
    counts = [len(fig.texts)]
    for text in texts:
        text.remove()
        counts.append(len(fig.texts))
    return {"texts": counts,
            "getters": [fig.get_suptitle(), fig.get_supxlabel(), fig.get_supylabel()]}


def test_remove_suptitle_supxlabel_supylabel():
    probe("Text.remove", _remove_suptitles)


def _set_size_inches(calls, figsize=None):
    fig = plt.figure(figsize=None if figsize is None else tuple(figsize))
    out = []
    for method, *args in calls:
        getattr(fig, method)(*args)
        out.append([float(v) for v in fig.get_size_inches()])
    return out


def test_set_fig_size():
    probe("Figure.set_size_inches", _set_size_inches, calls=[
        ["set_figwidth", 5], ["set_figheight", 1], ["set_size_inches", 2, 4],
        ["set_size_inches", [1, 3]],
    ])


def test_invalid_figure_size():
    for width, height in [(1, float("nan")), (-1, 1), (float("inf"), 1)]:
        probe("Figure.set_size_inches", _set_size_inches,
              calls=[["set_size_inches", width, height]])


def _figaspect(arg=None, shape=None):
    return [float(v) for v in plt.figaspect(np.zeros(shape) if shape else arg)]


def test_figaspect():
    probe("pyplot.figaspect", _figaspect, arg=2.0)
    probe("pyplot.figaspect", _figaspect, arg=2)
    for shape in [[1, 2], [2, 2]]:
        probe("pyplot.figaspect", _figaspect, shape=shape)


DATES = ['3 Jan 2013', '4 Jan 2013', '5 Jan 2013', '6 Jan 2013', '7 Jan 2013', '8 Jan 2013',
         '9 Jan 2013', '10 Jan 2013', '11 Jan 2013', '12 Jan 2013', '13 Jan 2013', '14 Jan 2013']
TIMES = [f'3 Jan 2013 {time}'
         for time in ['16:44:00', '16:45:00', '16:46:00', '16:47:00', '16:48:00', '16:49:00',
                      '16:51:00', '16:52:00', '16:53:00', '16:55:00', '16:56:00', '16:57:00']]


def _autofmt_xdate(which=None, colorbar=False):
    if colorbar:
        fig, ax = plt.subplots(layout="constrained")
        im = ax.imshow([[1, 4, 6], [2, 3, 5]])
        plt.colorbar(im)
        fig.autofmt_xdate()
        fig.draw_without_rendering()
        return [ax.get_xticklabels(which='major')[1].get_rotation()]
    fig, ax = plt.subplots()
    ax.plot(mdates.datestr2num(DATES), mdates.datestr2num(TIMES))
    ax.yaxis_date()
    ax.xaxis_date()
    ax.xaxis.set_minor_locator(AutoMinorLocator(2))
    ax.xaxis.set_minor_formatter(FixedFormatter([1, 2, 3, 4, 5, 6, 7]))
    fig.autofmt_xdate(0.2, 60, 'right', which)
    return {level: sorted({label.get_rotation() for label in ax.get_xticklabels(minor, level)})
            for minor, level in [(False, "major"), (True, "minor")]}


def test_autofmt_xdate():
    for which in ['both', 'major', 'minor']:
        probe("Figure.autofmt_xdate", _autofmt_xdate, which=which)
    probe("Figure.autofmt_xdate", _autofmt_xdate, colorbar=True)


def _change_dpi():
    fig = plt.figure(figsize=(4, 4))
    fig.draw_without_rendering()
    sizes = [[fig.canvas.renderer.width, fig.canvas.renderer.height]]
    fig.dpi = 50
    fig.draw_without_rendering()
    sizes.append([fig.canvas.renderer.width, fig.canvas.renderer.height])
    return sizes


def test_change_dpi():
    probe("Figure.set_dpi", _change_dpi, style="mpl20")


def _shareax_loglabels():
    fig, axs = plt.subplots(2, 2, sharex=True, sharey=True, squeeze=False)
    for ax in axs.flat:
        ax.plot([10, 20, 30], [10, 20, 30])
    ax.set_yscale("log")
    ax.set_xscale("log")
    return [[len(ax.xaxis.get_ticklabels(which='both')) > 0,
             len(ax.yaxis.get_ticklabels(which='both')) > 0] for ax in axs.flat]


def test_subplots_shareax_loglabels():
    probe("Figure.subplots", _shareax_loglabels)


def _savefig(format, metadata=None):
    fig = plt.figure()
    if metadata is not None:
        fig.savefig(io.BytesIO(), format=format, metadata=metadata)
        return True
    with tempfile.TemporaryDirectory() as tmp:
        out = pathlib.Path(tmp, f"test.{format}")
        plt.savefig(out)
        with out.open("rb") as file:
            return format.encode("ascii") in file.read(100).lower()


def test_fspath():
    for fmt in ["png", "pdf", "ps", "eps", "svg"]:
        probe("Figure.savefig", _savefig, format=fmt)


def test_savefig_metadata():
    for fmt in ['eps', 'pdf', 'png', 'ps', 'svg', 'svgz']:
        probe("Figure.savefig", _savefig, format=fmt, metadata={})


def test_savefig_metadata_error():
    for fmt in ['jpeg', 'jpg', 'tif', 'tiff', 'webp', "raw", "rgba"]:
        probe("Figure.savefig", _savefig, format=fmt, metadata={})


def _png_size(fig):
    with io.BytesIO() as buf:
        fig.savefig(buf, format="png")
        header = buf.getvalue()[16:24]
    return [int.from_bytes(header[:4], "big"), int.from_bytes(header[4:], "big")]


def _pixel_ratio(ratio, dpi=None):
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    if dpi is not None:
        fig.set_dpi(dpi)
    changed = fig.canvas._set_device_pixel_ratio(ratio)
    return {"changed": changed, "device_pixel_ratio": fig.canvas.device_pixel_ratio,
            "dpi": fig.dpi, "saved": _png_size(fig)}


def test_savefig_pixel_ratio():
    # The test style saves at ``savefig.dpi: 100``; ``mpl20`` saves at the
    # figure's resolution, which the ratio must leave alone.
    for style in [{}, {"style": "mpl20"}]:
        for ratio, dpi in [(1, {}), (2, {}), (1.234, {}), (1, {"dpi": 50}), (2, {"dpi": 50})]:
            probe("FigureCanvasBase._set_device_pixel_ratio", _pixel_ratio, ratio=ratio,
                  **dpi, **style)


def _engine_name(engine):
    if engine is None or isinstance(engine, PlaceHolderLayoutEngine):
        return "none"
    if isinstance(engine, TightLayoutEngine):
        return "tight"
    if isinstance(engine, ConstrainedLayoutEngine):
        return "compressed" if engine._compress else "constrained"
    raise TypeError(type(engine).__name__)


def _layout_engine(layout=None, calls=()):
    fig = plt.figure(layout=layout)
    out = [_engine_name(fig.get_layout_engine())]
    for name, *args in calls:
        if name == "set_layout_engine":
            fig.set_layout_engine(*args)
        elif name == "tight_layout":
            fig.subplots(ncols=2)
            fig.tight_layout(**args[0])
        elif name == "savefig_tight":
            fig.savefig(io.BytesIO(), bbox_inches='tight')
        elif name == "colorbar":
            fig.colorbar(fig.subplots().pcolormesh([[1.0, 2.0], [3.0, 4.0]]))
        out.append(_engine_name(fig.get_layout_engine()))
    return out


def test_valid_layouts():
    for layout in [None, "tight", "constrained"]:
        probe("Figure.get_layout_engine", _layout_engine, layout=layout)


def test_invalid_layouts():
    probe("Figure.get_layout_engine", _layout_engine, layout="foobar")
    probe("Figure.get_layout_engine", _layout_engine, layout="constrained",
          calls=[["set_layout_engine", "tight"], ["set_layout_engine", "constrained"]])
    # Once a colorbar exists, an engine placing colorbars differently is
    # refused, also after the engine was removed ("none" leaves a
    # placeholder); engines placing them alike are accepted.
    for layout, other in [("constrained", "tight"), ("tight", "constrained")]:
        probe("Figure.get_layout_engine", _layout_engine, layout=layout,
              calls=[["colorbar"], ["set_layout_engine", other]])
        probe("Figure.get_layout_engine", _layout_engine, layout=layout,
              calls=[["colorbar"], ["set_layout_engine", "none"]])
        probe("Figure.get_layout_engine", _layout_engine, layout=layout,
              calls=[["colorbar"], ["set_layout_engine", "none"], ["set_layout_engine", other]])
        probe("Figure.get_layout_engine", _layout_engine, layout=layout,
              calls=[["colorbar"], ["set_layout_engine", "none"], ["set_layout_engine", layout]])
    probe("Figure.get_layout_engine", _layout_engine, layout="constrained",
          calls=[["colorbar"], ["set_layout_engine", "compressed"]])
    probe("Figure.get_layout_engine", _layout_engine,
          calls=[["colorbar"], ["set_layout_engine", "none"], ["set_layout_engine", "constrained"]])
    probe("Figure.get_layout_engine", _layout_engine, layout="constrained",
          calls=[["colorbar"], ["tight_layout", {}]])


def test_tightlayout_autolayout_deconflict():
    for autolayout in [False, True]:
        probe("Figure.get_layout_engine", _layout_engine,
              calls=[["tight_layout", {"w_pad": 10}]], rc={"figure.autolayout": autolayout})


def test_layout_change_warning():
    for layout in ["constrained", "compressed"]:
        probe("Figure.get_layout_engine", _layout_engine, layout=layout,
              calls=[["tight_layout", {}]])


def test_repeated_tightlayout():
    probe("Figure.get_layout_engine", _layout_engine,
          calls=[["tight_layout", {}]] * 3)


def test_savefig_preserve_layout_engine():
    probe("Figure.get_layout_engine", _layout_engine, layout="compressed",
          calls=[["savefig_tight"]])


def _constrained_pads(params):
    fig = plt.figure(layout=ConstrainedLayoutEngine(**params))
    engine = fig.get_layout_engine().get()
    return [engine[key] for key in ["w_pad", "h_pad", "wspace", "hspace"]]


def test_get_constrained_layout_pads():
    probe("Figure.get_constrained_layout_pads", _constrained_pads,
          params={'w_pad': 0.01, 'h_pad': 0.02, 'wspace': 0.03, 'hspace': 0.04})


def _tightbbox():
    fig, ax = plt.subplots()
    ax.set_xlim(0, 1)
    t = ax.text(1., 0.5, 'This dangles over end')
    renderer = fig.canvas.get_renderer()
    out = {"text_x1": t.get_tightbbox(renderer).x1, "axes_x1": ax.get_tightbbox(renderer).x1,
           "fig_x1": fig.get_tightbbox(renderer).x1, "fig_x0": fig.get_tightbbox(renderer).x0}
    t.set_in_layout(False)
    out["axes_x1_out"] = ax.get_tightbbox(renderer).x1
    out["fig_x1_out"] = fig.get_tightbbox(renderer).x1
    t.set_in_layout(True)
    out["axes_x1_back"] = ax.get_tightbbox(renderer).x1
    return out


def test_tightbbox():
    probe("Figure.get_tightbbox", _tightbbox, tol=1e-6)


def _subfigure_tightbbox():
    fig = plt.figure(layout='constrained')
    fig.subfigures(1, 2)
    return fig.get_tightbbox(fig.canvas.get_renderer()).width


def test_subfigure_tightbbox():
    probe("SubFigure.get_tightbbox", _subfigure_tightbbox, tol=1e-9)


def _removed_axis():
    fig, axs = plt.subplots(2, sharex=True)
    axs[0].remove()
    fig.canvas.draw()
    return len(fig.axes)


def test_removed_axis():
    probe("Axes.remove", _removed_axis)


def _formatter_after_removal(set_scalar):
    fig, axs = plt.subplots(1, 2, sharex=True)
    if set_scalar:
        axs[1].xaxis.set_major_formatter(ScalarFormatter())
    axs[1].remove()
    axs[0].plot([datetime.datetime(2000, 1, 1), datetime.datetime(2000, 2, 1)], [0, 1])
    return type(axs[0].xaxis.get_major_formatter()).__name__


def test_axes_removal():
    for set_scalar in [False, True]:
        probe("Axis.get_major_formatter", _formatter_after_removal, set_scalar=set_scalar)


def _figure_clear(method):
    fig = plt.figure()
    out = []

    def record(*axes):
        out.append([len(fig.axes), len(fig.subfigs),
                    [any(ax is a for a in fig.axes) for ax in axes]])

    def clear(target):
        getattr(target, method)()

    gs = fig.add_gridspec(ncols=2, nrows=1)
    subfig = fig.add_subfigure(gs[0])
    subaxes = subfig.add_subplot(111)
    clear(fig)
    record(subaxes)
    subfig = fig.add_subfigure(gs[0])
    subaxes = subfig.add_subplot(111)
    mainaxes = fig.add_subplot(gs[1])
    mainaxes.remove()
    record(subaxes)
    mainaxes = fig.add_subplot(gs[1])
    subaxes.remove()
    record(mainaxes)
    subaxes = subfig.add_subplot(111)
    record(mainaxes, subaxes)
    clear(subfig)
    record(mainaxes, subaxes)
    out.append(len(subfig.axes))
    subaxes = subfig.add_subplot(111)
    clear(fig)
    record(subaxes)
    subfigs = [fig.add_subfigure(gs[i]) for i in [0, 1]]
    subaxes = [sfig.add_subplot(111) for sfig in subfigs]
    record(*subaxes)
    clear(subfigs[0])
    record(*subaxes)
    clear(subfigs[1])
    subfigs = [fig.add_subfigure(gs[i]) for i in [0, 1]]
    subaxes = [sfig.add_subplot(111) for sfig in subfigs]
    record(*subaxes)
    clear(fig)
    record(*subaxes)
    return out


def test_figure_clear():
    for method in ["clear", "clf"]:
        probe("Figure.clear", _figure_clear, method=method)


def _pick():
    fig, ax = plt.subplots()
    ax.scatter([0], [0], [1000], picker=True)
    fig.canvas.draw()
    out = [fig.stale]
    fig.pick(SimpleNamespace(x=ax.bbox.x0 + ax.bbox.width / 2,
                             y=ax.bbox.y0 + ax.bbox.height / 2, inaxes=ax, guiEvent=None))
    return out + [fig.stale]


def test_picking_does_not_stale():
    probe("Figure.pick", _pick, style="mpl20")


def _mosaic_layout(mosaic):
    if isinstance(mosaic, list):
        return [[_mosaic_layout(c) if isinstance(c, list) else c for c in row] for row in mosaic]
    return mosaic


def _per_subplot_kw(entries):
    if entries is None or isinstance(entries, dict):
        return entries
    return {tuple(key) if isinstance(key, list) else key: kw for key, kw in entries}


def _subplot_mosaic(mosaic, width_ratios=None, height_ratios=None, sharex=False, sharey=False,
                    per_subplot_kw=None, set_scales=None):
    fig = plt.figure()
    axd = fig.subplot_mosaic(_mosaic_layout(mosaic), width_ratios=width_ratios,
                             height_ratios=height_ratios, sharex=sharex, sharey=sharey,
                             per_subplot_kw=_per_subplot_kw(per_subplot_kw))
    label = {id(ax): key for key, ax in axd.items()}
    out = {"keys": list(axd), "axes": [label[id(ax)] for ax in fig.axes],
           "width_ratios": {k: list(ax.get_gridspec().get_width_ratios()) for k, ax in axd.items()},
           "height_ratios": {k: list(ax.get_gridspec().get_height_ratios())
                             for k, ax in axd.items()}}
    if set_scales is not None:
        axd["A"].set(xscale=set_scales[0], yscale=set_scales[1])
        out["scales"] = {k: [ax.get_xscale(), ax.get_yscale()] for k, ax in axd.items()}
    return out


NESTED_ORDER = [["A", [["B", "C"], ["D", "E"]]], ["F", "G"], [".", [["H", [["I"], ["."]]]]]]


def test_nested_width_ratios():
    probe("Figure.subplot_mosaic", _subplot_mosaic, mosaic=[["A", [["B"], ["C"]]]],
          width_ratios=[2, 1])


def test_nested_height_ratios():
    probe("Figure.subplot_mosaic", _subplot_mosaic, mosaic=[["A", [["B"], ["C"]]], ["D", "D"]],
          height_ratios=[1, 2])


def test_fail():
    for mosaic in [[["A", "."], [".", "A"]], [["A", "B"], [None, [["A", "B"], ["C", "D"]]]],
                   "AAA\nc\nBBB", [["A", [["B", "C"], ["D"]]], ["E", "E"]]]:
        probe("Figure.subplot_mosaic", _subplot_mosaic, mosaic=mosaic)


def test_user_order():
    for pattern in ['abc', 'cab', 'bca', 'cba', 'acb', 'bac']:
        probe("Figure.subplot_mosaic", _subplot_mosaic, mosaic=pattern)


def test_nested_user_order():
    probe("Figure.subplot_mosaic", _subplot_mosaic, mosaic=NESTED_ORDER)


def test_share_all():
    probe("Figure.subplot_mosaic", _subplot_mosaic, mosaic=NESTED_ORDER, sharex=True,
          sharey=True, set_scales=["log", "logit"])


def test_per_subplot_kw_expander():
    for entries in ([["A", {}], ["B", {}]], [[["A", "B"], {}]], [[["A", "B"], {}], ["B", {}]],
                    [["B", {}], [["A", "B"], {}]]):
        probe("Figure.subplot_mosaic", _subplot_mosaic, mosaic="AB", per_subplot_kw=entries)


def test_extra_per_subplot_kw():
    probe("Figure.subplot_mosaic", _subplot_mosaic, mosaic="A",
          per_subplot_kw={"B": {"projection": "polar"}})


def _reused_gridspec():
    fig = plt.figure()
    ax1 = fig.add_subplot(3, 2, (3, 5))
    ax2 = fig.add_subplot(3, 2, 4)
    ax3 = plt.subplot2grid((3, 2), (2, 1), colspan=2, fig=fig)
    gs1, gs2, gs3 = (ax.get_subplotspec().get_gridspec() for ax in (ax1, ax2, ax3))
    return [gs1 == gs2, gs1 == gs3]


def test_reused_gridspec():
    probe("SubplotSpec.get_gridspec", _reused_gridspec)


def _subfigure_dpi():
    fig = plt.figure(dpi=100)
    sub_fig = fig.subfigures()
    out = [sub_fig.get_dpi(), fig.get_dpi()]
    sub_fig.set_dpi(200)
    return out + [sub_fig.get_dpi(), fig.get_dpi()]


def test_subfigure_dpi():
    probe("SubFigure.set_dpi", _subfigure_dpi)


def _subfigure_bbox(nrows=1, ncols=1, hspace=None, wspace=None, specs=None, layout=None):
    fig = plt.figure(layout=layout)
    if specs is None:
        sub_figs = list(np.ravel(fig.subfigures(nrows, ncols, hspace=hspace, wspace=wspace)))
    else:
        gs = fig.add_gridspec(nrows, ncols)
        sub_figs = [fig.add_subfigure(gs[r0:r1, c0:c1]) for (r0, r1), (c0, c1) in specs]
    out = [[list(sf.bbox.min), list(sf.bbox.max)] for sf in sub_figs]
    if specs is not None:
        for sf in sub_figs:
            sf.add_subplot()
        fig.draw_without_rendering()
    return out


def test_subfigure_spanning():
    probe("SubFigure.bbox", _subfigure_bbox, nrows=3, ncols=3, layout="constrained",
          specs=[[[0, 1], [0, 1]], [[0, 2], [1, 2]], [[2, 3], [1, 3]], [[0, 3], [1, 3]]],
          tol=1e-9)


def test_subfigures_wspace_hspace():
    probe("SubFigure.bbox", _subfigure_bbox, nrows=2, ncols=3, hspace=0.5, wspace=1 / 6.,
          tol=1e-9)


def _subfigure_ticks():
    fig = plt.figure(constrained_layout=True, figsize=(10, 3))
    (subfig_bl, subfig_br) = fig.subfigures(1, 2, wspace=0.01, width_ratios=[7, 2])
    gs = subfig_bl.add_gridspec(nrows=1, ncols=14)
    ax1 = subfig_bl.add_subplot(gs[0, :1])
    ax1.scatter(x=[-56.46881504821776, 24.179891162109396], y=[1500, 3600])
    ax2 = subfig_bl.add_subplot(gs[0, 1:3], sharey=ax1)
    ax2.scatter(x=[-126.5357270050049, 94.68456736755368], y=[1500, 3600])
    subfig_bl.add_subplot(gs[0, 3:14], sharey=ax1)
    out = []
    for dpi in [120, 300]:
        fig.dpi = dpi
        fig.draw_without_rendering()
        out.append(list(ax2.get_xticks()))
    return out


def test_subfigure_ticks():
    probe("Axes.get_xticks", _subfigure_ticks, style="mpl20")


def _subfigure_remove():
    fig = plt.figure()
    sfs = fig.subfigures(2, 2)
    sfs[1, 1].subplots()
    sfs[1, 1].remove()
    out = [len(fig.subfigs)]
    fig = plt.figure()
    sf_arr = fig.subfigures(4, 3)
    return out + [all(a is b for a, b in zip(sf_arr.ravel(), fig.subfigs))]


def test_subfigure_remove():
    probe("SubFigure.remove", _subfigure_remove)


def _added(method, projections):
    fig = plt.figure()
    adder = fig.add_subplot if method == "add_subplot" else fig.add_axes
    where = (1, 1, 1) if method == "add_subplot" else ((0, 0, 1, 1),)
    axes = [adder(*where, **({} if p is None else {"projection": p})) for p in projections]
    return {"distinct": axes[0] is not axes[1], "names": [ax.name for ax in fig.axes]}


ADDED_PROJECTIONS = [[None, None], ["polar", "polar"], ["polar", None]]


def test_add_subplot_kwargs():
    for projections in ADDED_PROJECTIONS:
        probe("Figure.add_subplot", lambda projections: _added("add_subplot", projections),
              projections=projections)


def test_add_axes_kwargs():
    for projections in ADDED_PROJECTIONS:
        probe("Figure.add_axes", lambda projections: _added("add_axes", projections),
              projections=projections)


def _labels():
    fig = Figure(label='whole Figure')
    sub_fig = fig.subfigures(1, 1, label='sub figure')
    return [fig.get_label(), sub_fig.get_label()]


def test_kwargs_pass():
    probe("Figure.get_label", _labels)


def _visible_svg():
    fig = Figure()
    out = []
    for visible in [True, False]:
        fig.set_visible(visible)
        buf = io.StringIO()
        fig.savefig(buf, format='svg')
        out.append('<path ' in buf.getvalue())
    return out


def test_not_visible_figure():
    probe("Figure.set_visible", _visible_svg)


def _stale():
    fig = plt.figure()
    fig.draw_without_rendering()
    out = [fig.stale]
    sfig1 = fig.subfigures()
    out.append(fig.stale)
    fig.draw_without_rendering()
    out += [fig.stale, sfig1.stale]
    sfig2 = sfig1.subfigures()
    out += [fig.stale, sfig1.stale]
    fig.draw_without_rendering()
    return out + [fig.stale, sfig1.stale, sfig2.stale]


def test_subfigure_stale_propagation():
    probe("Figure.stale", _stale)


def _locate_colorbar():
    fig, ax = plt.subplots()
    pc = ax.pcolormesh(rng(2).normal(4).reshape(2, 2))
    cbar = fig.colorbar(pc, aspect=40)
    fig.savefig(io.BytesIO(), bbox_inches=mpl.transforms.Bbox([[0, 0], [4, 4]]))
    return [list(cbar.ax.get_position(original=True).bounds),
            list(cbar.ax.get_position(original=False).bounds)]


def test_savefig_locate_colorbar():
    probe("Figure.colorbar", _locate_colorbar, tol=1e-9)
