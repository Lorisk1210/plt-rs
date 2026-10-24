"""Behaviour of texts and annotations, ported from the non-image tests of
matplotlib's ``lib/matplotlib/tests/test_text.py``.

Area ``text_behaviour``. Unless noted, a scene draws on ``Figure()``
(6.4 x 4.8 in at 100 dpi) with an Agg canvas; bounding boxes are
``[[x0, y0], [x1, y1]]`` in display pixels.

Ops
---
``text.get_rotation``  {rotation, transform?} -> degrees
    ``Text(0, 0, "foo", rotation=rotation).get_rotation()``; ``rotation``
    is a number, ``"horizontal"``, ``"vertical"``, ``null`` or something
    invalid (``ValueError``). With ``transform`` (a transform op list as in
    the ``transforms`` area, applied to a fresh ``Affine2D``) the text is
    ``ax.text(0, 0, "test", transform=<affine>,
    transform_rotates_text=True)``.
    Mirrors ``test_get_rotation_string``, ``test_get_rotation_float``,
    ``test_get_rotation_int``, ``test_get_rotation_raises``,
    ``test_get_rotation_none``, ``test_get_rotation_mod360``,
    ``test_invalid_rotation_values`` and ``test_transform_rotates_text``.
``text.wrapped``  {figsize, subfigure, x, y, text, kw} -> str
    ``_get_wrapped_text()`` after a draw of ``(sub)fig.text(x, y, text,
    wrap=True, **kw)``; with ``subfigure`` the text is placed on the
    centre subfigure of a 3 x 3 ``GridSpec`` (``fig.add_subfigure(gs[1,
    1])``). ``kw`` holds ``rotation``, ``ha`` and ``size``.
    Mirrors ``test_wrap``, ``test_mathwrap``, ``test_long_word_wrap`` and
    ``test_wrap_no_wrap``.
``text.annotate_coords``  {xycoords} -> true
    ``ax.annotate("xy", (0, 0), xytext=(0.5, 0.5), xycoords=xycoords)``
    then a draw: ``true``, or the error for names that are not coordinate
    systems or that are offsets. Mirrors ``test_annotate_errors`` (the
    string cases).
``text.scene``  {scene} -> scene-specific observations:

``annotation_contains``  ``ax.annotate("hello", xy=(.4, .4), xytext=(.6,
    .6), arrowprops={"arrowstyle": "->"})``, drawn; ``[contains, ...]`` for
    a press at each data point of ``CONTAINS_POINTS``
    (``test_annotation_contains``).
``set_position``  ``ax.annotate("test", (0, 0), xytext=(0, 0),
    textcoords="figure pixels")``; ``{"before", "after"}`` extents around
    ``set_position((15, 15))`` (``test_set_position``).
``non_default_dpi``  ``ax.text(0.5, 0.5, s, ha="left", va="bottom")`` for
    ``s`` in ``""`` and ``"O"``: ``[[extent, extent at dpi=1000], ...]``
    (``test_non_default_dpi``; ``get_window_extent(dpi=...)``). Held to the
    upstream test's relative tolerance of 5e-2: matplotlib measures the
    glyphs at the renderer's dpi but the font-wide ascent and descent at
    the requested one, so only the latter scale.
``null_rotation``  for every ``ha`` x ``va``, ``ax.text(.5, .5, "test",
    rotation=0, ...)`` with ``rotation_mode="anchor"`` and ``"default"``:
    ``[[ha, va, anchor extent, default extent], ...]``
    (``test_null_rotation_with_rotation_mode``).
``annotation_update``  ``ax.annotate("annotation", xy=(0.5, 0.5))``:
    ``{"before", "after"}`` extents around ``fig.tight_layout()``
    (``test_annotation_update``).
``suptitle_wrapped``  ``Figure(figsize=(3, 3))`` suptitles: ``{"wrapped",
    "explicit"}`` extents of the long title with ``wrap=True`` and of the
    same title broken by hand (``test_get_window_extent_wrapped``).
``offset_fontsize``  ``ax.annotate("test", xy=(0.5, 0.5), fontsize=10)``
    with ``xytext=(10, 10), textcoords="offset points"`` and ``xytext=(1,
    1), textcoords="offset fontsize"``: ``[extent, extent]``
    (``test_annotate_offset_fontsize``).
``antialiased``  ``{"text", "set_false", "set_true", "annotation",
    "annotation_false"}``: ``get_antialiased()`` of a new text, after
    ``set_antialiased(False)`` and ``(True)``, of a new annotation and of
    one made with ``antialiased=False`` (``test_get_set_antialiased``,
    ``test_annotation_antialiased``).
``annotation_text_only``  ``{"annotation", "text"}``: extents of
    ``ax.annotate("test", xy=(0, 0), xycoords="figure pixels")`` and of
    ``fig.text(0, 0, "test")`` (``test_text_annotation_get_window_extent``).
``annotation_with_arrow``  ``Figure(dpi=100)``: ``{"text", "annotation"}``
    extents of ``ax.text(0, 0, "test")`` and of ``ax.annotate("test",
    xy=(0, 50), xytext=(50, 50), xycoords="figure pixels",
    arrowprops={"facecolor": "black", "width": 2, "headwidth": 21,
    "shrink": 0})`` (``test_text_with_arrow_annotation_get_window_extent``).
``arrow_annotation``  ``Figure(figsize=(2, 2))``: the extent of an empty
    annotation from ``(50, 50)`` to ``(0, 50)`` (figure pixels) with
    ``arrowprops={"facecolor": "black", "width": 8, "headwidth": 10,
    "shrink": 0}`` (``test_arrow_annotation_get_window_extent``).
``empty_annotation``  ``Figure(figsize=(2, 2))``: the extent of an empty
    annotation at ``(0, 50)`` figure pixels
    (``test_empty_annotation_get_window_extent``).
``tightbbox_log_domain``  log y axis with ``ylim=(1, 100)`` and unclipped
    texts at ``(0, -5)`` (outside the log domain), ``(nan, 10)`` and
    ``(0.5, 10)``: ``fig.get_tightbbox()`` in inches, which leaves the first
    two out (``test_text_tightbbox_outside_scale_domain``).
``nonfinite_positions``  ``ax.text(0, nan, "nan")``, ``ax.text(inf, 0,
    "inf")``: ``"drawn"`` once the figure draws (``test_nonfinite_pos``).
``parse_math``  ``{"parse_math_false", "parse_math_true", "rc_default",
    "rc_false"}``: ``"drawn"`` or ``"error"`` for drawing ``ax.text(0, 0,
    r"$ \\wrong{math} $")`` with ``parse_math=False``, ``True``, the rc
    default and ``text.parse_math = False`` (``test_parse_math``,
    ``test_parse_math_rcparams``).

Left out: ``test_char_index_at`` (a private helper of ``TextBox``, whose
behaviour the ``widgets`` area covers), ``test_ha_for_angle`` /
``test_va_for_angle`` (private; the ``xtick``/``ytick`` rotation modes are
covered by ``text_metrics`` extents), ``test_two_2line_texts`` (line
spacing is covered by ``text_metrics``), ``test_text_stale`` and
``test_text_size_binding`` (the ``mpl_text`` area),
``test_update_mutate_input``, ``test_fontproperties_kwarg_precedence``,
``test_validate_linespacing``, ``test_text_language_invalid``,
``test_text_repr`` and the metrics-cache tests (Python object model and
caches), ``test_invalid_color`` (colour parsing is the ``colors`` area) and
the usetex tests (no TeX).
"""

import numpy as np
from matplotlib.backend_bases import MouseEvent
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.gridspec import GridSpec
from matplotlib.text import Annotation, Text
from matplotlib.transforms import Affine2D
import matplotlib as mpl

from spec_support import ProbeSet

probes = ProbeSet("text_behaviour")

CONTAINS_POINTS = [(0.5, 0.6), (0.5, 0.5), (0.45, 0.45), (0.61, 0.605), (0.3, 0.3),
                   (0.62, 0.62), (0.7, 0.4)]
LONG = "This is a very long text that should be wrapped multiple times."
SUPTITLE = "suptitle that is clearly too long in this case"
WRONG_MATH = r"$ \wrong{math} $"


def _figure(**kw):
    fig = Figure(**kw)
    FigureCanvasAgg(fig)
    return fig


def _bbox(bbox):
    return bbox.get_points().tolist()


def _affine(ops):
    t = Affine2D()
    for name, *params in ops:
        getattr(t, name)(*params)
    return t


def _get_rotation(rotation, transform=None):
    if transform is None:
        return Text(0, 0, "foo", rotation=rotation).get_rotation()
    ax = _figure().add_subplot()
    return ax.text(0, 0, "test", rotation=rotation, transform=_affine(transform),
                   transform_rotates_text=True).get_rotation()


def _wrapped(figsize, subfigure, x, y, text, kw):
    fig = _figure(figsize=figsize)
    parent = fig.add_subfigure(GridSpec(3, 3, figure=fig)[1, 1]) if subfigure else fig
    artist = parent.text(x, y, text, wrap=True, **kw)
    fig.canvas.draw()
    return artist._get_wrapped_text()


def _annotate_coords(xycoords):
    fig = _figure()
    fig.add_subplot().annotate("xy", (0, 0), xytext=(0.5, 0.5), xycoords=xycoords)
    fig.canvas.draw()
    return True


def _annotation_contains():
    fig = _figure()
    ax = fig.add_subplot()
    ann = ax.annotate("hello", xy=(.4, .4), xytext=(.6, .6), arrowprops={"arrowstyle": "->"})
    fig.canvas.draw()
    hits = []
    for point in CONTAINS_POINTS:
        x, y = ax.transData.transform(point)
        event = MouseEvent("button_press_event", fig.canvas, x, y)
        event.x, event.y = x, y  # keep the exact position (events round to pixels)
        hits.append(bool(ann.contains(event)[0]))
    return hits


def _set_position():
    fig = _figure()
    ax = fig.add_subplot()
    ann = ax.annotate("test", (0, 0), xytext=(0, 0), textcoords="figure pixels")
    fig.canvas.draw()
    before = _bbox(ann.get_window_extent())
    ann.set_position((15, 15))
    fig.canvas.draw()
    return {"before": before, "after": _bbox(ann.get_window_extent())}


def _non_default_dpi():
    fig = _figure()
    ax = fig.add_subplot()
    texts = [ax.text(0.5, 0.5, s, ha="left", va="bottom") for s in ["", "O"]]
    fig.canvas.draw()
    return [[_bbox(t.get_window_extent()), _bbox(t.get_window_extent(dpi=fig.dpi * 10))]
            for t in texts]


def _null_rotation():
    fig = _figure()
    ax = fig.add_subplot()
    pairs = []
    for ha in ["center", "right", "left"]:
        for va in ["center", "top", "bottom", "baseline", "center_baseline"]:
            kw = dict(rotation=0, va=va, ha=ha)
            pairs.append((ha, va, ax.text(.5, .5, "test", rotation_mode="anchor", **kw),
                          ax.text(.5, .5, "test", rotation_mode="default", **kw)))
    fig.canvas.draw()
    return [[ha, va, _bbox(t0.get_window_extent()), _bbox(t1.get_window_extent())]
            for ha, va, t0, t1 in pairs]


def _annotation_update():
    fig = _figure()
    ax = fig.add_subplot()
    ann = ax.annotate("annotation", xy=(0.5, 0.5))
    before = _bbox(ann.get_window_extent(fig.canvas.get_renderer()))
    fig.tight_layout()
    return {"before": before, "after": _bbox(ann.get_window_extent(fig.canvas.get_renderer()))}


def _suptitle_wrapped():
    wrapped = _figure(figsize=(3, 3))
    wrapped.suptitle(SUPTITLE, wrap=True)
    explicit = _figure(figsize=(3, 3))
    explicit.suptitle("suptitle that is clearly\ntoo long in this case")
    return {"wrapped": _bbox(wrapped._suptitle.get_window_extent()),
            "explicit": _bbox(explicit._suptitle.get_window_extent())}


def _offset_fontsize():
    fig = _figure()
    ax = fig.add_subplot()
    anns = [ax.annotate("test", xy=(0.5, 0.5), xytext=xytext, fontsize=10, xycoords="data",
                        textcoords=coords)
            for xytext, coords in [((10, 10), "offset points"), ((1, 1), "offset fontsize")]]
    return [_bbox(ann.get_window_extent()) for ann in anns]


def _antialiased():
    text = Text(.5, .5, "foo\nbar")
    observed = {"text": text.get_antialiased()}
    text.set_antialiased(False)
    observed["set_false"] = text.get_antialiased()
    text.set_antialiased(True)
    observed["set_true"] = text.get_antialiased()
    observed["annotation"] = Annotation("foo\nbar", (.5, .5)).get_antialiased()
    observed["annotation_false"] = Annotation("foo\nbar", (.5, .5),
                                              antialiased=False).get_antialiased()
    return observed


def _annotation_text_only():
    fig = _figure()
    ann = fig.add_subplot().annotate("test", xy=(0, 0), xycoords="figure pixels")
    text = fig.text(0, 0, "test")
    fig.canvas.draw()
    return {"annotation": _bbox(ann.get_window_extent()), "text": _bbox(text.get_window_extent())}


def _annotation_with_arrow():
    fig = _figure(dpi=100)
    ax = fig.add_subplot()
    text = ax.text(s="test", x=0, y=0)
    ann = ax.annotate("test", xy=(0.0, 50.0), xytext=(50.0, 50.0), xycoords="figure pixels",
                      arrowprops={"facecolor": "black", "width": 2, "headwidth": 21,
                                  "shrink": 0.0})
    fig.canvas.draw()
    return {"text": _bbox(text.get_window_extent()), "annotation": _bbox(ann.get_window_extent())}


def _arrow_annotation():
    fig = _figure(figsize=(2, 2), dpi=100)
    ann = fig.add_subplot().annotate(
        "", xy=(0.0, 50.0), xytext=(50.0, 50.0), xycoords="figure pixels",
        arrowprops={"facecolor": "black", "width": 8, "headwidth": 10, "shrink": 0.0})
    fig.canvas.draw()
    return _bbox(ann.get_window_extent())


def _empty_annotation():
    fig = _figure(figsize=(2, 2), dpi=100)
    ann = fig.add_subplot().annotate("", xy=(0.0, 50.0), xytext=(0.0, 50.0),
                                     xycoords="figure pixels")
    fig.canvas.draw()
    return _bbox(ann.get_window_extent())


def _tightbbox_log_domain():
    fig = _figure()
    ax = fig.add_subplot()
    ax.set_yscale("log")
    ax.set_ylim(1, 100)
    for x, y, s in [(0, -5, "invalid"), (np.nan, 10, "nan"), (0.5, 10, "valid")]:
        ax.text(x, y, s, clip_on=False)
    return _bbox(fig.get_tightbbox(fig.canvas.get_renderer()))


def _nonfinite_positions():
    fig = _figure()
    ax = fig.add_subplot()
    ax.text(0, np.nan, "nan")
    ax.text(np.inf, 0, "inf")
    fig.canvas.draw()
    return "drawn"


def _draw_outcome(rc, **kw):
    with mpl.rc_context(rc):
        fig = _figure()
        fig.add_subplot().text(0, 0, WRONG_MATH, **kw)
        try:
            fig.canvas.draw()
        except ValueError:
            return "error"
    return "drawn"


def _parse_math():
    return {"parse_math_false": _draw_outcome({}, parse_math=False),
            "parse_math_true": _draw_outcome({}, parse_math=True),
            "rc_default": _draw_outcome({}),
            "rc_false": _draw_outcome({"text.parse_math": False})}


SCENES = {
    "annotation_contains": _annotation_contains,
    "set_position": _set_position,
    "non_default_dpi": _non_default_dpi,
    "null_rotation": _null_rotation,
    "annotation_update": _annotation_update,
    "suptitle_wrapped": _suptitle_wrapped,
    "offset_fontsize": _offset_fontsize,
    "antialiased": _antialiased,
    "annotation_text_only": _annotation_text_only,
    "annotation_with_arrow": _annotation_with_arrow,
    "arrow_annotation": _arrow_annotation,
    "empty_annotation": _empty_annotation,
    "tightbbox_log_domain": _tightbbox_log_domain,
    "nonfinite_positions": _nonfinite_positions,
    "parse_math": _parse_math,
}

ROTATIONS = ["horizontal", "vertical", 15.0, 16.7, 77.4, 67, 16, 41, None, 360.0, 377.0,
             897.2, -30.0, -720.5, 0.0, 359.9, np.nan, np.inf, "hozirontal",
             "invalid string", [90]]


def test_rotations():
    for rotation in ROTATIONS:
        probes.probe("text.get_rotation", _get_rotation, rotation=rotation)
    for rotation, transform in [(0.0, [["rotate_deg", 30]]), (10.0, [["rotate_deg", 30]]),
                                (0.0, [["rotate_deg", -45]]), (45.0, [["scale", 1, 3]]),
                                (30.0, [["scale", -1, 1]]), (0.0, [["skew_deg", 20, 0]])]:
        probes.probe("text.get_rotation", _get_rotation, rotation=rotation, transform=transform)


WRAPS = [
    ((18, 18), True, 0.7, 0.7, LONG, {"rotation": 0, "ha": "left"}),
    ((18, 18), True, 0.5, 0.7, LONG, {"rotation": 95, "ha": "left"}),
    ((18, 18), True, 0.3, 0.7, LONG, {"rotation": 0, "ha": "right"}),
    ((18, 18), True, 0.3, 0.7, LONG, {"rotation": 185, "ha": "left"}),
    ((18, 18), True, 0.5, 0.7, LONG, {"rotation": 0, "ha": "center"}),
    ((5, 4), False, 0, 0.5, r"This is a very $\overline{\mathrm{long}}$ line of Mathtext.",
     {"size": 40}),
    ((6, 4), False, 9.5, 8, "Alonglineoftexttowrap", {}),
    ((6, 4), False, 0, 0, "non wrapped text", {}),
]


def test_wraps():
    for figsize, subfigure, x, y, text, kw in WRAPS:
        probes.probe("text.wrapped", _wrapped, figsize=figsize, subfigure=subfigure, x=x, y=y,
                     text=text, kw=kw)


def test_annotate_coords():
    for xycoords in ["data", "axes fraction", "figure pixels", "subfigure points", "foo",
                     "foo bar", "offset foo", "axes foo", "offset points", "offset fraction"]:
        probes.probe("text.annotate_coords", _annotate_coords, xycoords=xycoords)


TOLERANCES = {"non_default_dpi": 5e-2}


def test_scenes():
    for scene in SCENES:
        probes.probe("text.scene", lambda scene: SCENES[scene](), scene=scene,
                     tol=TOLERANCES.get(scene))
