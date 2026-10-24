"""Legends, ported from matplotlib's own ``tests/test_legend.py``: the
entries a legend lists, its proxy artists (``legend_handles``,
``get_lines``, ``get_patches``), label colours, the frame, title and
alignment, placement inside, beside and outside the axes, dragging, and
the legend labels of ``plot`` and ``boxplot``.

Area ``mpl_legend``. Every op runs in the test style (see
``probes/_mpl_tests.py``): its ``style`` and ``rc`` arguments.

Ops
---
``Axes.legend``  {seed} -> ``{"texts", "extent"}``: the labels and the
    window extent (pixels, after a draw) of ``test_legend_ordereddict``'s
    legend of ten single-point scatters (``rng(seed).normal(10)`` for x
    and y) labelled ``a`` and ``b``, made from an ordered dict of their
    handles and labels at ``loc="center left"``,
    ``bbox_to_anchor=(1, .5)``.
``Legend.get_window_extent``  {scene, dpi?} -> legend extents. ``patches``:
    ``test_legend_auto5``'s bounds in axes fractions of the legends at
    ``"center"`` and ``"best"`` among an ellipse, a U-shaped polygon and a
    ring; ``dpi``: the left edge (pixels, without drawing) of a legend in
    a figure at ``dpi``; ``cached``: the extents (pixels) of an axes and a
    figure legend after a draw.
``Figure.legend``  {loc} -> ``{"axes", "legend"}``: the extents (pixels)
    of the axes and of the figure legend at ``"outside <loc>"`` of a
    constrained figure at 100 dpi, laid out.
``legend._get_legend_handles_labels``  {colors} -> the labels of two
    scatters both labelled ``test``, in these colours.
``Axes.get_legend_handles_labels``  {artist} -> ``{"legend",
    "labels"}``: the labels of ``ax.legend()`` and of
    ``get_legend_handles_labels`` beside a labelled ``text`` or a
    ``pcolormesh`` (``rng(0).uniform(100)``).
``Legend.legend_handles``  {scene, ...} -> the proxy artists of a
    legend. ``reverse`` {reverse}: the markers of the handles and the
    labels of three marked lines; ``handlerline2d``: the marker of a
    ``Line2D`` handle given beside a scatter and the marker of its proxy;
    ``linecollection``: per line collection, its first dash pattern and
    its proxy's (``[offset, dashes]``); ``alpha``: the face and edge
    colours of the proxy of a histogram drawn at alpha 0.25 once set to
    alpha 1, then those of the histogram's second bar; ``patchcollection``
    {empty}: the labels, the number of proxies and, unless empty, the
    face and edge colours, line width and dash pattern of the proxy and of
    the collection.
``Legend.get_lines``  {scene, numpoints?} -> ``numpoints``: the x data of
    the proxy line of a plotted line at ``numpoints``; ``markers``: the
    markers of the proxies of three marker-only ``Line2D`` handles, and
    the labels.
``Legend.get_patches``  {} -> the face and edge colours of the proxy of
    a ``fill_between`` once set to alpha 0.5, then the collection's.
``Legend.get_frame``  {legend} -> ``{"alpha", "linewidth"}`` of the frame
    of ``ax.legend(**legend)`` for a labelled line.
``Legend.get_title``  {title_fontsize?, title_fontproperties?} -> the
    title's font size of ``plt.legend(title="Aardvark", ...)``;
    ``title_fontproperties`` is a ``FontProperties``'s arguments (given as
    a dict, or as the object with ``as_object``).
``Legend.get_alignment``  {alignment, set?} -> ``{"alignment", "title",
    "legend"}``: the alignment of a titled legend made with
    ``alignment=...`` (or set with ``set_alignment`` when ``set``), and
    the extents (pixels) of its title and frame after a draw.
``Legend.set_loc``  {loc} -> ``_get_loc()`` of an axes legend moved to
    ``loc``.
``Legend.get_texts``  {kind, calls, labelcolor?, rc_labelcolor?} -> the
    colours of the labels of a legend with ``labelcolor`` (unset: the rc
    parameter, set to the tuple ``rc_labelcolor`` if given) of one
    plotting call (``kind``: ``plot`` of ``arange(10)`` against
    ``k * arange(10)`` or ``scatter``) per keyword dict of ``calls``;
    colour lists stand for numpy arrays.
``Legend.texts``  {kind, calls} -> for each plotting call (``hist`` of
    ``arange(10)``, ``plot`` of ``arange(5)``, ``scatter`` of
    ``arange(5)`` against itself, with the keywords of ``calls``) added to
    one axes, after ``legend(labelcolor="linecolor")``: the colour of the
    last label, the colours of the last proxy (``face``/``edge`` of a
    patch or scatter, ``color``/``markerfacecolor``/``markeredgecolor`` of
    a line) and the same colours of the artist (a histogram's first
    patch). ``"none"`` reads as ``null``.
``Legend.get_draggable``  {draggable?} -> whether the legend made with
    ``draggable`` is draggable; without it, ``plt.legend()``'s with no
    labelled artists before and after ``set_draggable(True)`` and
    ``set_draggable(False)``.
``Axes.plot``  {x?, y, label} -> the labels of the lines of one ``plot``
    call (``y`` a list of rows: one line per column) and of its legend.
``SubFigure.legend``  {} -> ``{"subfigure", "figure", "texts"}``: the
    number of legends of a subfigure and of its figure after
    ``subfig.legend()``, and the legend's labels.
``Axes.boxplot``  {label, patch_artist?, showbox?} -> ``{"boxes",
    "medians", "handles", "labels"}``: the labels of the boxes and medians
    (``null`` for hidden ones, starting with ``_``) of a box plot of
    ``rng(0).uniform(40)`` in four columns, the kinds of the legend handles
    (``patch``/``line``) and the legend labels.
``pyplot.legend``  {} -> the labels of ``plt.legend()`` for a host axes
    and its ``twinx`` parasite, each with a labelled line.

Cases
-----
Named after the upstream tests. Adaptations: random data comes from
``spec_support.rng``; ``test_handler_numpoints`` gives 0.5 points, which
plt-rs (where ``numpoints`` is a count) cannot express: the port asks for
one point, the other case of at most one, and for none, which both reject;
the placement tests record the extents rather than comparing two of
them; ``test_legend_linewidth`` and ``test_shadow_framealpha`` share one
line; the mock-based ``test_parasite`` records the labels of the legend
made; ``test_subfigure_legend`` counts the legends of the subfigure and
of its figure instead of comparing ``get_figure``.

Not ported: generators and iterators as handles and labels
(``test_legend_generator``, ``test_legend_handle_label_mismatch_no_len``),
the ``TypeError`` of Python's calling conventions, mocks
(``test_legend_handler_map``, ``test_legend_annotate``), usetex, the
slow-``best`` warnings (they monkeypatch the timer), ``loc`` given as a
``dict``, ``shadow`` given as a string, warnings alone
(``test_legend_nolabels_warning``), attribute identity
(``test_legend_text_axes``), a string iterated as labels with the handles
of another figure (``test_cross_figure_patch_legend``), a Python class as a
label, and a ``title_fontproperties`` dict without a size under
``legend.title_fontsize: 20``: plt-rs's ``FontProperties`` always has a
size, so the dict cannot ask for the rc one.
"""

import collections

import matplotlib as mpl
import matplotlib.collections as mcollections
import matplotlib.colors as mcolors
import matplotlib.lines as mlines
import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from matplotlib import legend as mlegend
from matplotlib.font_manager import FontProperties

from probes._mpl_tests import prober
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_legend")
probe = prober(probes)

#: Extents in pixels, compared to a millionth.
EXTENT_TOL = 1e-6


def _extents(bbox):
    return [float(v) for v in bbox.extents]


def _rgba(color):
    return [float(v) for v in mcolors.to_rgba(color)]


def _first_rgba(colors):
    """The first colour of a colour or colour array, ``null`` for none."""
    if isinstance(colors, str):
        return _rgba(colors)
    colors = np.asarray(colors, dtype=float)
    if colors.ndim == 2:
        return [float(v) for v in colors[0]] if len(colors) else None
    return _rgba(colors)


def _dashes(pattern):
    offset, dashes = pattern
    return [float(offset), None if dashes is None else [float(v) for v in dashes]]


def _ordereddict(seed):
    r = rng(seed)
    X, Y = r.normal(10), r.normal(10)
    labels = ['a'] * 5 + ['b'] * 5
    colors = ['r'] * 5 + ['g'] * 5
    fig, ax = plt.subplots()
    for x, y, label, color in zip(X, Y, labels, colors):
        ax.scatter(x, y, label=label, c=color)
    handles, labels = ax.get_legend_handles_labels()
    legend = collections.OrderedDict(zip(labels, handles))
    leg = ax.legend(legend.values(), legend.keys(), loc='center left', bbox_to_anchor=(1, .5))
    fig.canvas.draw()
    return {"texts": [t.get_text() for t in leg.get_texts()],
            "extent": _extents(leg.get_window_extent())}


def test_legend_ordereddict():
    probe("Axes.legend", _ordereddict, seed=0, tol=EXTENT_TOL)


def _window_extent(scene, dpi=None):
    if scene == "patches":
        fig, axs = plt.subplots(ncols=2, figsize=(9.6, 4.8))
        bounds = []
        for ax, loc in zip(axs.flat, ("center", "best")):
            for patch in [
                    mpatches.Ellipse(xy=(0.5, 0.9), width=0.8, height=0.2, fc="C1"),
                    mpatches.Polygon(np.array([
                        [0, 1], [0, 0], [1, 0], [1, 1], [0.9, 1.0], [0.9, 0.1],
                        [0.1, 0.1], [0.1, 1.0], [0.1, 1.0]]), fc="C1"),
                    mpatches.Wedge((0.5, 0.5), 0.5, 0, 360, width=0.05, fc="C0")]:
                ax.add_patch(patch)
            ax.plot([0.1, 0.9], [0.9, 0.9], label="A segment")
            leg = ax.legend(loc=loc)
            fig.canvas.draw()
            bounds.append([float(v) for v in
                           leg.get_window_extent().transformed(ax.transAxes.inverted()).bounds])
        return bounds
    if scene == "dpi":
        fig, ax = plt.subplots(dpi=dpi)
        ax.plot(range(10), label='Aardvark')
        leg = ax.legend()
        return float(leg.get_window_extent(fig.canvas.get_renderer()).x0)
    if scene == "cached":
        fig, ax = plt.subplots(dpi=100)
        ax.plot(range(10), label='Aardvark')
        leg = ax.legend()
        leg2 = fig.legend()
        fig.canvas.draw()
        return [_extents(leg.get_window_extent()), _extents(leg2.get_window_extent())]
    raise ValueError(scene)


def test_legend_auto5():
    probe("Legend.get_window_extent", _window_extent, scene="patches", tol=EXTENT_TOL)


def test_legend_proper_window_extent():
    for dpi in [100, 200]:
        probe("Legend.get_window_extent", _window_extent, scene="dpi", dpi=dpi, tol=EXTENT_TOL)


def test_window_extent_cached_renderer():
    probe("Legend.get_window_extent", _window_extent, scene="cached", tol=EXTENT_TOL)


def _outside(loc):
    fig, axs = plt.subplots(constrained_layout=True, dpi=100)
    axs.plot(range(10), label='Boo1')
    leg = fig.legend(loc='outside ' + loc)
    fig.draw_without_rendering()
    return {"axes": _extents(axs.get_window_extent()),
            "legend": _extents(leg.get_window_extent())}


def test_figure_legend_outside():
    todos = ['upper ' + pos for pos in ['left', 'center', 'right']]
    todos += ['lower ' + pos for pos in ['left', 'center', 'right']]
    todos += ['left ' + pos for pos in ['lower', 'center', 'upper']]
    todos += ['right ' + pos for pos in ['lower', 'center', 'upper']]
    for loc in todos:
        probe("Figure.legend", _outside, loc=loc, tol=EXTENT_TOL)


def _repeatcheckok(colors):
    fig, ax = plt.subplots()
    ax.scatter(0.0, 1.0, color=colors[0], marker='o', label='test')
    ax.scatter(0.5, 0.0, color=colors[1], marker='v', label='test')
    ax.legend()
    _, labels = mlegend._get_legend_handles_labels([ax])
    return labels


def test_legend_repeatcheckok():
    for colors in [["k", "r"], ["k", "k"]]:
        probe("legend._get_legend_handles_labels", _repeatcheckok, colors=colors)


def _nohandler(artist):
    fig, ax = plt.subplots()
    if artist == "text":
        ax.plot([0], label="mock data")
        ax.text(x=0, y=0, s="text", label="label")
        legend = [t.get_text() for t in ax.legend().get_texts()]
    else:
        ax.pcolormesh(rng(0).uniform(100).reshape(10, 10))
        legend = None
    return {"legend": legend, "labels": ax.get_legend_handles_labels()[1]}


def test_text_nohandler_warning():
    for artist in ["text", "pcolormesh"]:
        probe("Axes.get_legend_handles_labels", _nohandler, artist=artist)


def _legend_handles(scene, reverse=None, empty=None):
    fig, ax = plt.subplots()
    if scene == "reverse":
        labels = ["First label", "Second label", "Third label"]
        for marker, label in zip(['.', ',', 'o'], labels):
            ax.plot(1, 1, marker, label=label)
        leg = ax.legend(reverse=reverse)
        return {"texts": [t.get_text() for t in leg.get_texts()],
                "markers": [h.get_marker() for h in leg.legend_handles]}
    if scene == "handlerline2d":
        ax.scatter([0, 1], [0, 1], marker="v")
        handles = [mlines.Line2D([0], [0], marker="v")]
        leg = ax.legend(handles, ["Aardvark"], numpoints=1)
        return [handles[0].get_marker(), leg.legend_handles[0].get_marker()]
    if scene == "linecollection":
        lines1 = [[(0, .5), (.5, 1)], [(.3, .6), (.2, .2)]]
        lines2 = [[[0.7, .2], [.8, .4]], [[.5, .7], [.6, .1]]]
        lines3 = [[[0.6, .2], [.8, .4]], [[.5, .7], [.1, .1]]]
        lc1 = mcollections.LineCollection(lines1, linestyles="--", lw=3)
        lc2 = mcollections.LineCollection(lines2, linestyles="-.")
        lc3 = mcollections.LineCollection(lines3, linestyles=":", lw=.5)
        for lc in (lc1, lc2, lc3):
            ax.add_collection(lc)
        leg = ax.legend([lc1, lc2, lc3], ["line1", "line2", 'line 3'])
        return [[_dashes(oh.get_linestyles()[0]), _dashes(lh._dash_pattern)]
                for oh, lh in zip((lc1, lc2, lc3), leg.legend_handles)]
    if scene == "alpha":
        x, n, hh = ax.hist([1, 2, 3], alpha=0.25, label='data', color='red')
        legend = ax.legend()
        for lh in legend.legend_handles:
            lh.set_alpha(1.0)
        return [_first_rgba(lh.get_facecolor()), _first_rgba(lh.get_edgecolor()),
                _first_rgba(hh[1].get_facecolor()), _first_rgba(hh[1].get_edgecolor())]
    if scene == "patchcollection":
        patches = [] if empty else [mpatches.Circle((0, 0), 1), mpatches.Circle((2, 0), 1)]
        style = {} if empty else {"facecolor": 'red', "edgecolor": 'blue', "linewidths": 3,
                                  "linestyle": '--'}
        pc = mcollections.PatchCollection(patches, label="empty collection" if empty
                                          else "patch collection", **style)
        ax.add_collection(pc)
        if not empty:
            ax.autoscale_view()
        leg = ax.legend()
        out = {"texts": [t.get_text() for t in leg.get_texts()],
               "handles": len(leg.legend_handles)}
        if not empty:
            lp = leg.legend_handles[0]
            out["proxy"] = [_first_rgba(lp.get_facecolor()), _first_rgba(lp.get_edgecolor()),
                            float(lp.get_linewidth()), _dashes(lp.get_linestyle())]
            out["collection"] = [_first_rgba(pc.get_facecolor()),
                                 _first_rgba(pc.get_edgecolor()),
                                 float(pc.get_linewidths()[0]), _dashes(pc.get_linestyles()[0])]
        return out
    raise ValueError(scene)


def test_reverse_legend_handles_and_labels():
    probe("Legend.legend_handles", _legend_handles, scene="reverse", reverse=True)


def test_linecollection_scaled_dashes():
    probe("Legend.legend_handles", _legend_handles, scene="linecollection")


def test_alpha_handles():
    probe("Legend.legend_handles", _legend_handles, scene="alpha")


def test_handlerline2d():
    probe("Legend.legend_handles", _legend_handles, scene="handlerline2d")


def test_patchcollection_legend():
    probe("Legend.legend_handles", _legend_handles, scene="patchcollection", empty=False)


def test_patchcollection_legend_empty():
    probe("Legend.legend_handles", _legend_handles, scene="patchcollection", empty=True)


def _get_lines(scene, numpoints=None):
    fig, ax = plt.subplots()
    if scene == "numpoints":
        ax.plot(range(5), label='test')
        leg = ax.legend(numpoints=numpoints)
        return [float(v) for v in leg.get_lines()[0].get_xdata()]
    if scene == "markers":
        lines = [mlines.Line2D([0], [0], ls='None', marker=mark) for mark in ['.', '*', 'v']]
        legend = ax.legend(lines, ["foo", "bar", "xyzzy"])
        return {"markers": [line.get_marker() for line in legend.get_lines()],
                "texts": [text.get_text() for text in legend.get_texts()]}
    raise ValueError(scene)


def test_handler_numpoints():
    for numpoints in [0, 1]:
        probe("Legend.get_lines", _get_lines, scene="numpoints", numpoints=numpoints)


def test_legend_markers_from_line2d():
    probe("Legend.get_lines", _get_lines, scene="markers")


def _get_patches():
    fig, ax = plt.subplots()
    pc = ax.fill_between([0, 1], [2, 3], color='#123456', label='label')
    patch = ax.legend().get_patches()[0]
    patch.set_alpha(0.5)
    return [_first_rgba(patch.get_facecolor()), _first_rgba(patch.get_edgecolor()),
            _first_rgba(pc.get_facecolor()), _first_rgba(pc.get_edgecolor())]


def test_setting_alpha_keeps_polycollection_color():
    probe("Legend.get_patches", _get_patches)


def _get_frame(legend):
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], label='data')
    frame = ax.legend(**legend).get_frame()
    return {"alpha": frame.get_alpha(), "linewidth": float(frame.get_linewidth())}


def test_shadow_framealpha():
    probe("Legend.get_frame", _get_frame, legend={"shadow": True, "facecolor": "w"})


def test_legend_linewidth():
    probe("Legend.get_frame", _get_frame, legend={"linewidth": 2.5})
    probe("Legend.get_frame", _get_frame, legend={}, rc={"legend.linewidth": 3.0})
    probe("Legend.get_frame", _get_frame, legend={},
          rc={"legend.linewidth": None, "patch.linewidth": 1.5})
    probe("Legend.get_frame", _get_frame, legend={"linewidth": 4.0},
          rc={"legend.linewidth": 1.0})


def _get_title(title_fontsize=None, title_fontproperties=None, as_object=False):
    plt.plot(range(10), label="mock data")
    fontproperties = title_fontproperties
    if as_object:
        fontproperties = FontProperties(**title_fontproperties)
    leg = plt.legend(title='Aardvark', title_fontsize=title_fontsize,
                     title_fontproperties=fontproperties)
    return float(leg.get_title().get_fontsize())


def test_legend_title_fontprop_fontsize():
    serif22 = {"family": "serif", "size": 22}
    probe("Legend.get_title", _get_title, title_fontsize=22, title_fontproperties=serif22)
    probe("Legend.get_title", _get_title, title_fontproperties=serif22, as_object=True)
    probe("Legend.get_title", _get_title, title_fontsize=22)
    probe("Legend.get_title", _get_title, title_fontproperties=serif22)
    probe("Legend.get_title", _get_title, title_fontproperties={"family": "serif"},
          rc={"legend.title_fontsize": None})
    probe("Legend.get_title", _get_title, rc={"legend.title_fontsize": None})
    probe("Legend.get_title", _get_title, rc={"legend.title_fontsize": 20})


def _alignment(alignment, set=False):
    fig, ax = plt.subplots()
    ax.plot(range(10), label='test')
    if set:
        leg = ax.legend(title="Aardvark")
        leg.set_alignment(alignment)
    else:
        leg = ax.legend(title="Aardvark", alignment=alignment)
    assert leg.get_children()[0].align == leg.get_alignment()
    fig.canvas.draw()
    return {"alignment": leg.get_alignment(),
            "title": _extents(leg.get_title().get_window_extent()),
            "legend": _extents(leg.get_frame().get_window_extent())}


def test_legend_alignment():
    for alignment in ['center', 'left', 'right']:
        probe("Legend.get_alignment", _alignment, alignment=alignment, tol=EXTENT_TOL)


def test_legend_set_alignment():
    for alignment in ['center', 'left', 'right']:
        probe("Legend.get_alignment", _alignment, alignment=alignment, set=True,
              tol=EXTENT_TOL)


def _set_loc(loc):
    fig, ax = plt.subplots()
    ax.plot(range(10), label='test')
    leg = ax.legend()
    leg.set_loc(loc)
    return leg._get_loc()


def test_ax_legend_set_loc():
    for loc in ['center', 'best']:
        probe("Legend.set_loc", _set_loc, loc=loc)


def _label_colors(kind, calls, labelcolor=None, rc_labelcolor=None):
    if rc_labelcolor is not None:
        mpl.rcParams['legend.labelcolor'] = tuple(rc_labelcolor)
    fig, ax = plt.subplots()
    for k, kwargs in enumerate(calls, start=1):
        kwargs = {key: np.array(value) if isinstance(value, list) else value
                  for key, value in kwargs.items()}
        if kind == "plot":
            ax.plot(np.arange(10), np.arange(10) * k, **kwargs)
        else:
            ax.scatter(np.arange(10), np.arange(10) * k, **kwargs)
    leg = ax.legend() if labelcolor is None else ax.legend(labelcolor=labelcolor)
    return [_rgba(text.get_color()) for text in leg.get_texts()]


def _viridis(seed):
    return [[float(v) for v in c] for c in plt.colormaps["viridis"](rng(seed).uniform(10))]


def test_legend_pathcollection_labelcolors():
    cycle = ['r', 'g', 'b', 'c', 'm'] * 2
    for rule, keyword in [("linecolor", "c"), ("markeredgecolor", "edgecolor"),
                          ("markerfacecolor", "facecolor")]:
        # test_legend_pathcollection_labelcolor_<rule>_iterable
        probe("Legend.get_texts", _label_colors, kind="scatter",
              calls=[{"label": "#1", keyword: cycle}], labelcolor=rule)
    # test_legend_pathcollection_labelcolor_markerfacecolor
    probe("Legend.get_texts", _label_colors, kind="scatter",
          calls=[{"label": f"#{k}", "facecolor": c} for k, c in [(1, "r"), (2, "g"), (3, "b")]],
          labelcolor="markerfacecolor")
    # test_legend_pathcollection_labelcolor_markeredgecolor_cmap
    probe("Legend.get_texts", _label_colors, kind="scatter",
          calls=[{"label": "#1", "c": list(range(10)), "edgecolor": _viridis(1),
                  "cmap": "Reds"}], labelcolor="markeredgecolor")
    # test_legend_pathcollection_labelcolor_markfacecolor_cmap
    probe("Legend.get_texts", _label_colors, kind="scatter",
          calls=[{"label": "#1", "c": _viridis(2)}], labelcolor="markerfacecolor")


def test_legend_labelcolor_rcparams():
    plain = [{"label": f"#{k}"} for k in (1, 2, 3)]
    for color in ["red", "none"]:
        # test_legend_labelcolor_rcparam_single
        probe("Legend.get_texts", _label_colors, kind="plot", calls=plain,
              rc={"legend.labelcolor": color})
    # A tuple, which the rc-file syntax cannot give this parameter.
    probe("Legend.get_texts", _label_colors, kind="plot", calls=plain,
          rc_labelcolor=[0.5, 0.5, 0.5])
    for short, keyword in [("mec", "markeredgecolor"), ("mfc", "markerfacecolor")]:
        # test_legend_labelcolor_rcparam_<keyword>_short
        probe("Legend.get_texts", _label_colors, kind="plot",
              calls=[{"label": f"#{k}", keyword: c} for k, c in [(1, "r"), (2, "g"), (3, "b")]],
              rc={"legend.labelcolor": short})


def _maybe_rgba(color):
    rgba = _first_rgba(color)
    return None if rgba is None or (rgba[3] == 0 and np.all(np.asarray(rgba) == 0)) else rgba


def _last_entry_colors(kind, calls):
    fig, ax = plt.subplots()
    out = []
    for kwargs in calls:
        if kind == "hist":
            artist = ax.hist(np.arange(10), **kwargs)[-1][0]
        elif kind == "plot":
            kwargs = dict(kwargs)
            fmt = kwargs.pop("fmt", None)
            artist, = ax.plot(np.arange(5), *([fmt] if fmt else []), **kwargs)
        else:
            artist = ax.scatter(np.arange(5), np.arange(5), **kwargs)
        leg = ax.legend(labelcolor='linecolor')
        label = _maybe_rgba(leg.texts[-1].get_color())
        if kind == "plot":
            proxy = leg.get_lines()[-1]
            getters = ["get_color", "get_markerfacecolor", "get_markeredgecolor"]
        else:
            proxy = leg.get_patches()[-1] if kind == "hist" else leg.legend_handles[-1]
            getters = ["get_facecolor", "get_edgecolor"]
        out.append([label, [_maybe_rgba(getattr(proxy, g)()) for g in getters],
                    [_maybe_rgba(getattr(artist, g)()) for g in getters]])
    return out


def test_legend_labelcolor_linecolor_histograms():
    probe("Legend.texts", _last_entry_colors, kind="hist", calls=[
        {"histtype": 'bar', "color": 'r', "label": "red bar hist with a red label"},
        {"histtype": 'step', "color": 'g', "label": "green step hist, green label"},
        {"histtype": 'stepfilled', "color": 'b', "label": "blue stepfilled hist"},
        {"histtype": 'bar', "color": 'r', "ec": 'b', "label": "red bar hist, blue edges"},
        {"histtype": 'bar', "fc": 'r', "ec": 'b', "label": "red bar hist, blue edges 2"},
        {"histtype": 'bar', "fc": 'none', "ec": 'b', "label": "unfilled blue bar hist"},
        {"histtype": 'step', "color": 'r', "ec": 'b', "label": "blue step hist"},
        {"histtype": 'step', "ec": 'b', "label": "blue step hist 2"},
        {"histtype": 'stepfilled', "color": 'r', "ec": 'b', "label": "red stepfilled hist"},
        {"histtype": 'stepfilled', "fc": 'r', "ec": 'b', "label": "red stepfilled hist 2"},
        {"histtype": 'stepfilled', "fc": 'none', "ec": 'b', "label": "unfilled stepfilled"},
        {"histtype": 'stepfilled', "fc": 'r', "ec": 'none', "label": "edgeless stepfilled"},
    ])


def test_legend_labelcolor_linecolor_plot():
    probe("Legend.texts", _last_entry_colors, kind="plot", calls=[
        {"c": 'r', "label": "red line with a red label"},
        {"fmt": 'o', "c": 'r', "label": "red circles with a red label"},
        {"fmt": 'o', "c": 'r', "mec": 'b', "label": "red circles, blue edges, red label"},
        {"fmt": 'o', "mfc": 'r', "mec": 'b', "label": "red circles, blue edges 2"},
        {"fmt": 'o', "mfc": 'none', "mec": 'b', "label": "blue unfilled circles"},
        {"fmt": 'o', "mfc": 'r', "mec": 'none', "label": "red edgeless circles"},
        {"fmt": 'o', "c": 'none', "mec": 'none', "label": "invisible circles"},
    ])


def test_legend_labelcolor_linecolor_scatter():
    probe("Legend.texts", _last_entry_colors, kind="scatter", calls=[
        {"c": 'r', "label": "red circles with a red label"},
        {"c": 'r', "ec": 'b', "label": "red circles, blue edges, red label"},
        {"fc": 'r', "ec": 'b', "label": "red circles, blue edges 2"},
        {"fc": 'none', "ec": 'b', "label": "blue unfilled circles, blue label"},
        {"fc": 'r', "ec": 'none', "label": "red edgeless circles, red label"},
        {"c": 'none', "ec": 'none', "label": "invisible circles"},
    ])


def _draggable(draggable=None):
    if draggable is None:
        legend = plt.legend()
        states = [legend.get_draggable()]
        legend.set_draggable(True)
        states.append(legend.get_draggable())
        legend.set_draggable(False)
        states.append(legend.get_draggable())
        return states
    fig, ax = plt.subplots()
    ax.plot(range(10), label='shabnams')
    return ax.legend(draggable=draggable).get_draggable()


def test_get_set_draggable():
    probe("Legend.get_draggable", _draggable)


def test_legend_draggable():
    for draggable in [True, False]:
        probe("Legend.get_draggable", _draggable, draggable=draggable)


def _plot(y, label, x=None):
    fig, ax = plt.subplots()
    lines = ax.plot(y, label=label) if x is None else ax.plot(x, y, label=label)
    leg = ax.legend()
    return {"lines": [line.get_label() for line in lines],
            "texts": [entry.get_text() for entry in leg.get_texts()]}


def test_plot_multiple_input_multiple_label():
    probe("Axes.plot", _plot, x=[1, 2, 3], y=[[1, 2], [2, 5], [4, 9]], label=['low', 'high'])


def test_plot_multiple_input_single_label():
    for label in ['one', 1]:
        probe("Axes.plot", _plot, x=[1, 2, 3], y=[[1, 2], [2, 5], [4, 9]], label=label)


def test_plot_single_input_multiple_label():
    probe("Axes.plot", _plot, x=[1, 2, 3], y=[2, 5, 6], label=['low', 'high'])


def test_plot_single_input_list_label():
    probe("Axes.plot", _plot, y=[[0], [1]], label=['A'])


def test_plot_multiple_label_incorrect_length_exception():
    probe("Axes.plot", _plot, x=[1, 2, 3], y=[[1, 2], [2, 5], [4, 9]],
          label=['high', 'low', 'medium'])


def _subfigure_legend():
    fig = plt.figure()
    subfig = fig.subfigures()
    ax = subfig.subplots()
    ax.plot([0, 1], [0, 1], label="line")
    leg = subfig.legend()
    assert leg.get_figure(root=False) is subfig
    return {"subfigure": len(subfig.legends), "figure": len(fig.legends),
            "texts": [t.get_text() for t in leg.get_texts()]}


def test_subfigure_legend():
    probe("SubFigure.legend", _subfigure_legend)


def _shown(label):
    """A legend label, ``null`` for the hidden ones (starting with ``_``)."""
    return None if label.startswith("_") else label


def _boxplot(label, patch_artist=False, showbox=True):
    data = rng(0).uniform(40).reshape(10, 4)
    fig, ax = plt.subplots()
    bp = ax.boxplot(data, patch_artist=patch_artist, label=label, showbox=showbox)
    handles, labels = ax.get_legend_handles_labels()
    return {"boxes": [_shown(v.get_label()) for v in bp['boxes']],
            "medians": [_shown(v.get_label()) for v in bp['medians']],
            "handles": ["patch" if isinstance(h, mpatches.Patch) else "line" for h in handles],
            "labels": labels}


def test_boxplot_legend_labels():
    labels = ['box A', 'box B', 'box C', 'box D']
    probe("Axes.boxplot", _boxplot, label=labels, patch_artist=True)
    probe("Axes.boxplot", _boxplot, label=labels, showbox=False)
    probe("Axes.boxplot", _boxplot, label=labels[:-1])
    probe("Axes.boxplot", _boxplot, label='box A')


def _parasite():
    from mpl_toolkits.axes_grid1 import host_subplot

    host = host_subplot(111)
    par = host.twinx()
    host.plot([0, 1, 2], [0, 1, 2], label="Density")
    par.plot([0, 1, 2], [0, 3, 2], label="Temperature")
    return [t.get_text() for t in plt.legend().get_texts()]


def test_parasite():
    probe("pyplot.legend", _parasite)
