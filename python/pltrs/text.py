"""
Classes for including text in a figure (matplotlib's ``text``).

pltrs (Rust) keeps every property of a text and lays it out and draws it
(``pltrs::text``); the `Text` methods here keep matplotlib's signatures,
aliases and validation, and its keyword handling.
"""

from collections.abc import Sequence
import contextlib
import datetime
import errno
from numbers import Real
import os
import sys
import weakref

import numpy as np

from . import _docstring
from . import (_api, _live_transforms, _mirrors, _text_coords, _text_overrides,
               _unusable, _values, cbook, units)
from . import _text_metrics as _text_metrics_module
# pltrs (Rust) warns through it about the glyphs no font has.
from . import _text_helpers  # noqa: F401
from ._annotation_coords import CoordsMixin, PositionedMixin
from ._annotation_kept import KeptMixin, annotate_arguments, warn_textcoords
from ._artist_copying import check_copyable
from ._native import annotation as _annotation, artist as _artist, text as _text
from .artist import Artist
from .font_manager import FontManager, FontProperties, fontManager  # noqa: F401
from .textpath import TextPath, TextToPath  # noqa: F401 (logically located here)
from .transforms import Bbox, BboxBase, Transform

#: The keywords `Text` takes explicitly, in the order matplotlib's
#: ``Text._reset_visual_defaults`` applies them, before the other properties.
_INIT_PROPERTIES = (
    "color", "fontproperties", "usetex", "parse_math", "wrap",
    "verticalalignment", "horizontalalignment", "multialignment", "rotation",
    "transform_rotates_text", "linespacing", "rotation_mode", "antialiased",
)

#: What ``None`` stands for in the explicit keywords of `Text`: an rcParam
#: (by name), or a value.
_INIT_DEFAULTS = {
    "color": ("rc", "text.color"),
    "parse_math": ("rc", "text.parse_math"),
    "antialiased": ("rc", "text.antialiased"),
    "linespacing": ("value", "normal"),
}

#: The aliases of the box properties `set_bbox` takes (`FancyBboxPatch`'s).
_BOX_ALIASES = {
    "fc": "facecolor", "ec": "edgecolor", "lw": "linewidth",
    "ls": "linestyle", "aa": "antialiased",
}

#: The colours of a text's box, which None leaves at the patch's default.
_BOX_COLORS = frozenset({"facecolor", "edgecolor", "color"})


def _check_bbox_arguments(rectprops):
    """
    Raise the `TypeError` matplotlib's `Text.set_bbox` raises for box
    properties naming arguments it gives the box itself: it makes the box
    as ``FancyBboxPatch((0, 0), 1, 1, boxstyle=..., transform=...,
    **props)``.
    """
    from .patches import FancyBboxPatch
    if "transform" in rectprops:
        raise TypeError(
            f"{FancyBboxPatch.__module__}.{FancyBboxPatch.__qualname__}() got "
            "multiple values for keyword argument 'transform'")
    for key in rectprops:
        if key in ("xy", "width", "height"):
            raise TypeError(
                f"FancyBboxPatch.__init__() got multiple values for argument '{key}'")


def _rc(name):
    """The rcParam *name* (the package's rcParams, imported once it exists)."""
    from . import rcParams
    return rcParams[name]


def _get_text_metrics_with_cache(renderer, text, fontprop, ismath, dpi):
    """
    Call ``renderer.get_text_width_height_descent``, caching the results.

    pltrs (Rust) measures every line it lays out through this function
    while the program replaces it (see `._text_overrides`), and through the
    renderer's cache otherwise (see `._text_metrics`).
    """
    # hit the outer cache layer and get the function to compute the metrics
    # for this renderer instance
    get_text_metrics = _get_text_metrics_function(renderer)
    # call the function to compute the metrics and return
    #
    # We pass a copy of the fontprop because FontProperties is both mutable and
    # has a `__hash__` that depends on that mutable state.  This is not ideal
    # as it means the hash of an object is not stable over time which leads to
    # very confusing behavior when used as keys in dictionaries or hashes.
    return get_text_metrics(text, fontprop.copy(), ismath, dpi)


def _get_text_metrics_function(input_renderer, _cache=weakref.WeakKeyDictionary()):
    """
    Helper function to provide a two-layered cache for font metrics


    To get the rendered size of a size of string we need to know:
      - what renderer we are using
      - the current dpi of the renderer
      - the string
      - the font properties
      - is it math text or not

    We do this as a two-layer cache with the outer layer being tied to a
    renderer instance and the inner layer handling everything else.

    The outer layer is implemented as `.WeakKeyDictionary` keyed on the
    renderer.  As long as someone else is holding a hard ref to the renderer
    we will keep the cache alive, but it will be automatically dropped when
    the renderer is garbage collected.

    The inner layer is a least-recently-used cache with a large maximum size
    (such that we expect very few cache misses in actual use cases), kept by
    pltrs (Rust), which measures the lines it lays out with the renderer
    through it (see `._text_metrics`).  As the dpi is mutable on the
    renderer, it is part of the cache key on the inner layer.

    This function takes a renderer and returns a function that can be used to
    get the font metrics.

    Parameters
    ----------
    input_renderer : maplotlib.backend_bases.RendererBase
        The renderer to set the cache up for.

    _cache : dict, optional
        We are using the mutable default value to attach the cache to the function.

        In principle you could pass a different dict-like to this function to inject
        a different cache, but please don't.  This is an internal function not meant to
        be reused outside of the narrow context we need it for.

        There is a possible race condition here between threads, we may need to drop the
        mutable default and switch to a threadlocal variable in the future.

    """
    if (_text_metrics := _cache.get(input_renderer, None)) is None:
        # The cache refers to its renderer weakly: closing over a hard-ref
        # would create an unbreakable reference cycle.
        _text_metrics = _text_metrics_module.TextMetrics(weakref.ref(input_renderer))
        _cache[input_renderer] = _text_metrics
    return _text_metrics


def _layout_renderer(figure):
    """The renderer *figure* is laid out with, whose text metrics cache
    measures its texts: the one its canvas prints with where that is not
    the canvas' own (`.backend_pdf.RendererPdf` under the PDF canvas), else
    the figure's (`.Figure._get_renderer`)."""
    canvas = figure.get_figure(root=True).canvas
    if hasattr(canvas, "_print_renderer"):
        return canvas._print_renderer()
    return figure._get_renderer()


def _check_metrics_renderer(renderer):
    """Fail as matplotlib's text metrics fail for *renderer*, which they
    look up in a weak cache and then ask for the metrics of a line
    (``_get_text_metrics_with_cache``)."""
    weakref.ref(renderer)
    if not hasattr(renderer, "get_text_width_height_descent"):
        raise AttributeError(f"{type(renderer).__name__!r} object has no "
                             f"attribute 'get_text_width_height_descent'")


def _measuring_with(renderer):
    """Lay out and draw with *renderer*'s text metrics cache installed, so
    that the lines pltrs (Rust) measures meanwhile go through it."""
    return _text_metrics_module.measuring_with(
        renderer, _get_text_metrics_function(renderer))


@contextlib.contextmanager
def _measured_by(renderer, dpi):
    """Measure with *renderer*, which a program hands a measuring method
    (``get_window_extent(renderer)``, ``get_tightbbox(renderer)``), for a
    layout at *dpi*: its text metrics cache installed, and the lines and
    the pads matplotlib converts with the renderer measured at its
    resolution where that is not *dpi*, as matplotlib measures with the
    renderer (the texts are placed at the figure's)."""
    own = getattr(renderer, "dpi", None)
    at = None if own is None or dpi is None or own == dpi else float(own)
    with _measuring_with(renderer), _text_metrics_module.measured_at(at):
        yield


def _measured_by_given(renderer, artist):
    """`_measured_by` *renderer* for a layout of *artist*'s figure when the
    program gave a renderer, else nothing."""
    figure = artist.get_figure(root=True)
    if renderer is None or figure is None:
        return contextlib.nullcontext()
    return _measured_by(renderer, figure.dpi)


class _TextFont(_mirrors.Mirroring):
    """The font of *text*, which its `.FontProperties` object stands for."""

    def __init__(self, text):
        self.text = text

    def push(self, fontproperties):
        _text.set_fontproperties(self.text._native, FontManager._props(fontproperties))


def _rotation_degrees(s):
    """The rotation `Text.set_rotation` takes *s* for, in degrees in
    [0, 360): a number, 'horizontal' (or None) or 'vertical'."""
    if isinstance(s, Real):
        return float(s) % 360
    if cbook._str_equal(s, 'horizontal') or s is None:
        return 0.
    if cbook._str_equal(s, 'vertical'):
        return 90.
    raise ValueError("rotation must be 'vertical', 'horizontal' or "
                     f"a number, not {s}")


@_docstring.interpd
@_api.define_aliases({
    "color": ["c"],
    "fontproperties": ["font", "font_properties"],
    "fontfamily": ["family"],
    "fontname": ["name"],
    "fontsize": ["size"],
    "fontstretch": ["stretch"],
    "fontstyle": ["style"],
    "fontvariant": ["variant"],
    "fontweight": ["weight"],
    "horizontalalignment": ["ha"],
    "verticalalignment": ["va"],
    "multialignment": ["ma"],
})
@_mirrors.mirrorable
class Text(Artist):
    """Handle storing and drawing of text in window or data coordinates."""

    def set_clip_box(self, clipbox):
        # docstring inherited: matplotlib's override has none.
        super().set_clip_box(clipbox)

    def set_clip_path(self, path, transform=None):
        # docstring inherited: matplotlib's override has none.
        super().set_clip_path(path, transform)

    def set_clip_on(self, b):
        # docstring inherited: matplotlib's override has none.
        super().set_clip_on(b)

    zorder = 3
    _charsize_cache = dict()
    #: What a mirror reads refreshed from its part (see `._mirrors`): the
    #: text and the place a tick label's draw gives it.
    _refreshed_getters = ("get_text", "get_position", "get_unitless_position")

    def __repr__(self):
        x, y = self.get_position()
        # The text as set, which a subclass's `get_text` may resolve.
        return f"Text({x}, {y}, {Text.get_text(self)!r})"

    def __init__(self,
                 x=0, y=0, text='', *,
                 color=None,           # defaults to rc params
                 verticalalignment='baseline',
                 horizontalalignment='left',
                 multialignment=None,
                 fontproperties=None,  # defaults to FontProperties()
                 rotation=None,
                 linespacing=None,
                 rotation_mode=None,
                 usetex=None,          # defaults to rcParams['text.usetex']
                 wrap=False,
                 transform_rotates_text=False,
                 parse_math=None,    # defaults to rcParams['text.parse_math']
                 antialiased=None,  # defaults to rcParams['text.antialiased']
                 **kwargs
                 ):
        """
        Create a `.Text` instance at *x*, *y* with string *text*.

        The text is aligned relative to the anchor point (*x*, *y*) according
        to ``horizontalalignment`` (default: 'left') and ``verticalalignment``
        (default: 'baseline'). See also
        :doc:`/gallery/text_labels_and_annotations/text_alignment`.

        While Text accepts the 'label' keyword argument, by default it is not
        added to the handles of a legend.

        Valid keyword arguments are:

        %(Text:kwdoc)s
        """
        kwargs = {**cbook._given_keywords(Text.__init__, locals()), **kwargs}
        self._native = _text.new(0.0, 0.0, "" if text is None else str(text))
        super().__init__()
        _place(self, (x, y))
        self._init_properties(kwargs)

    def _convert_units_for(self, axes):
        # A coordinate in units converts once the text joins *axes*, as
        # matplotlib converts its position when drawn.
        for index, name in enumerate(_POSITION):
            given = _unusable.kept(self, name, _NOT_KEPT)
            if given is not _NOT_KEPT:
                self._set_text_coordinate(index, given, axes)

    def _char_index_at(self, x):
        """
        Calculate the index closest to the coordinate x in display space.

        The position of text[index] is assumed to be the sum of the widths
        of all preceding characters text[:index].

        This works only on single line texts.
        """
        text = self.get_text()
        if not text:
            return 0
        fontproperties = str(self.get_fontproperties())
        charsize_cache = Text._charsize_cache.setdefault(fontproperties, {})
        for char in set(text):
            if char not in charsize_cache:
                self.set_text(char)
                bb = self.get_window_extent()
                charsize_cache[char] = bb.x1 - bb.x0
        self.set_text(text)
        bb = self.get_window_extent()
        size_accum = np.cumsum([0] + [charsize_cache[x] for x in text])
        std_x = x - bb.x0
        return (np.abs(size_accum - std_x)).argmin()

    def _init_properties(self, kwargs):
        """
        Apply the keywords a new text was made with, as ``Text.__init__``
        does: its explicit keywords first, in matplotlib's order, then the
        other properties through `update`.
        """
        kwargs = dict(kwargs)
        for name in _INIT_PROPERTIES:
            if name not in kwargs:
                continue
            value = kwargs.pop(name)
            if value is None and name in _INIT_DEFAULTS:
                kind, default = _INIT_DEFAULTS[name]
                value = _rc(default) if kind == "rc" else default
            if name == "multialignment" and value is None:
                _text.set_multialignment(self._native, None)
            else:
                getattr(self, f"set_{name}")(value)
        self.update(kwargs)

    def set_bbox(self, rectprops):
        """
        Draw a box behind/around the text.

        This can be used to set a background and/or a frame around the text.
        It's realized through a `.FancyBboxPatch` behind the text (see also
        `.Text.get_bbox_patch`). The bbox patch is None by default and only
        created when needed.

        Parameters
        ----------
        rectprops : dict with properties for `.FancyBboxPatch` or None
             The default boxstyle is 'square'. The mutation
             scale of the `.patches.FancyBboxPatch` is set to the fontsize.

             Pass ``None`` to remove the bbox patch completely.

        Examples
        --------
        ::

            t.set_bbox(dict(facecolor='red', alpha=0.5))
        """
        if rectprops is None:
            _text.set_bbox(self._native, None)
            return
        # matplotlib copies the properties first (``rectprops.copy()``),
        # then pops the box style and pad off the copy.
        given = rectprops.copy()
        rectprops.copy().pop("boxstyle", None)
        _check_bbox_arguments(given)
        # A colour of None is the patch's default, as matplotlib's
        # `.FancyBboxPatch` takes it.
        props = {_BOX_ALIASES.get(key, key): value for key, value in given.items()
                 if not (value is None and _BOX_ALIASES.get(key, key) in _BOX_COLORS)}
        boxstyle = props.get("boxstyle")
        if boxstyle is not None:
            from .patches import BoxStyle
            from ._patch_styles import _native_spec
            # As matplotlib's: *pad* completes a style name lacking one, and
            # is ignored otherwise.
            pad = props.pop("pad", None)
            if isinstance(boxstyle, str):
                if "pad" not in boxstyle:
                    boxstyle += ",pad=%0.2f" % (0.3 if pad is None else pad)
                boxstyle = BoxStyle(boxstyle)
            props["boxstyle"] = _native_spec(boxstyle, BoxStyle)
        # The style object, which a style of the program's is known by.
        self._bbox_style = boxstyle
        _text.set_bbox(self._native, props)

    def update_bbox_position_size(self, renderer):
        """
        Update the location and the size of the bbox.

        This method should be used when the position and size of the bbox needs
        to be updated before actually drawing the bbox.
        """
        if self.get_bbox_patch() is not None:
            # matplotlib lays the text out with *renderer* to size the box.
            _check_metrics_renderer(renderer)
        # pltrs: the patch of `get_bbox_patch` moves where pltrs (Rust) draws
        # the box in the figure's current layout.
        from ._text_box import place
        place(self)

    def update_from(self, other):
        # docstring inherited
        check_copyable(other)
        self._took_from(other, _text.update_from(self._native, other._native))
        if "_picker" in other.__dict__:
            self._picker = other._picker
        self.stale = True

    def get_bbox_patch(self):
        """
        Return the bbox Patch, or None if the `.patches.FancyBboxPatch`
        is not made.

        For more details see `.Text.set_bbox`.
        """
        # pltrs: the patch has the box's look (box style, colours, hatch,
        # alpha, path effects), and its setters change the box; the text
        # places it when drawn, so it stays on the unit square.
        from ._text_box import mirror
        return mirror(self)

    def set_backgroundcolor(self, color):
        """
        Set the background color of the text.

        This is realized through the bbox (see `.set_bbox`),
        creating the bbox patch if needed.

        Parameters
        ----------
        color : :mpltype:`color`

        See Also
        --------
        .set_bbox : To change the position of the bounding box
        """
        if color is None:
            # The patch's default colours, as matplotlib's patch takes None.
            patch = self.get_bbox_patch()
            if patch is None:
                self.set_bbox(dict(facecolor=None, edgecolor=None))
            else:
                patch.set_facecolor(None)
            return
        from .colors import to_rgba
        # matplotlib's box patch fails to take a colour that is no colour.
        to_rgba(color)
        _text.set_backgroundcolor(self._native, color)

    def _decouple(self, removed):
        # docstring inherited; a part of an Axes (its title) that an Axes
        # listing it took out is drawn there still: the Axes fails to draw
        # where matplotlib's visible text, without its figure, lays itself
        # out at the figure's resolution.
        axes = self.axes if _artist.is_part(self._native) else None
        super()._decouple(removed)
        if axes is not None:
            self._drawn_by = axes
            _unusable.keep(axes, self._without_figure, self,
                           lambda text: text.get_visible() and getattr(None, "dpi"),
                           measured=False)

    def _recouple(self):
        # docstring inherited
        super()._recouple()
        axes = self.__dict__.pop("_drawn_by", None)
        if axes is not None:
            _unusable.drop(axes, self._without_figure)

    @property
    def _without_figure(self):
        """The name of the problem of the Axes drawing the text, a part of
        it, while the text has no figure (`_decouple`)."""
        return f"text {id(self)} without its figure"

    def update(self, kwargs):
        ret = []
        kwargs = cbook.normalize_kwargs(kwargs, Text)
        sentinel = object()  # bbox can be None, so use another sentinel.
        fontproperties = kwargs.pop("fontproperties", sentinel)
        if fontproperties is not sentinel:
            ret.append(self.set_fontproperties(fontproperties))
        bbox = kwargs.pop("bbox", sentinel)
        ret.extend(super().update(kwargs))
        if bbox is not sentinel:
            ret.append(self.set_bbox(bbox))
        return ret

    def contains(self, mouseevent):
        """
        Return whether the mouse event occurred inside the axis-aligned
        bounding-box of the text.
        """
        if not self.get_visible():
            return False, {}
        return _text.contains(self._native, mouseevent.x, mouseevent.y), {}

    def get_rotation(self):
        """Return the text angle in degrees in the range [0, 360)."""
        return _text.get_rotation(self._native)

    def get_transform_rotates_text(self):
        """
        Return whether rotations of the transform affect the text direction.
        """
        return _values.given_flag(self, "transform_rotates_text",
                                  _text.get_transform_rotates_text(self._native))

    def set_rotation_mode(self, m):
        """
        Set text rotation mode.

        Parameters
        ----------
        m : {None, 'default', 'anchor', 'xtick', 'ytick'}
            If ``"default"``, the text will be first rotated, then aligned according
            to their horizontal and vertical alignments.  If ``"anchor"``, then
            alignment occurs before rotation. "xtick" and "ytick" adjust the
            horizontal/vertical alignment so that the text is visually pointing
            towards its anchor point. This is primarily used for rotated tick
            labels and positions them nicely towards their ticks. Passing
            ``None`` will set the rotation mode to ``"default"``.
        """
        if m is None:
            m = "default"
        else:
            _api.check_in_list(("anchor", "default", "xtick", "ytick"),
                               rotation_mode=m)
        _text.set_rotation_mode(self._native, m)

    def get_rotation_mode(self):
        """Return the text rotation mode."""
        return _text.get_rotation_mode(self._native)

    def set_antialiased(self, antialiased):
        """
        Set whether to use antialiased rendering.

        Parameters
        ----------
        antialiased : bool

        Notes
        -----
        Antialiasing will be determined by :rc:`text.antialiased`
        and the parameter *antialiased* will have no effect if the text contains
        math expressions.
        """
        _text.set_antialiased(self._native, bool(antialiased))
        _values.remember(self, "antialiased", antialiased)

    def get_antialiased(self):
        """Return whether antialiased rendering is used."""
        # As given, as matplotlib keeps it (``1``, ``'False'``).
        return _values.given_flag(self, "antialiased",
                                  _text.get_antialiased(self._native))

    @property
    def _antialiased(self):
        return self.get_antialiased()

    def get_wrap(self):
        """Return whether the text can be wrapped."""
        return _values.given_flag(self, "wrap", _text.get_wrap(self._native))

    def set_wrap(self, wrap):
        """
        Set whether the text can be wrapped.

        Wrapping makes sure the text is confined to the (sub)figure box. It
        does not take into account any other artists.

        Parameters
        ----------
        wrap : bool

        Notes
        -----
        Wrapping does not work together with
        ``savefig(..., bbox_inches='tight')`` (which is also used internally
        by ``%matplotlib inline`` in IPython/Jupyter). The 'tight' setting
        rescales the canvas to accommodate all content and happens before
        wrapping.
        """
        _text.set_wrap(self._native, bool(wrap))
        _values.remember(self, "wrap", wrap)

    def _get_wrapped_text(self):
        """
        Return a copy of the text string with new lines added so that the
        text is wrapped relative to the parent figure (if `get_wrap` is True).
        """
        return _text.get_wrapped_text(self._native)

    def _get_layout(self, renderer):
        """
        Return

        - the rotated, axis-aligned text bbox;
        - a list of ``(line, (width, ascent, descent), xy)`` tuples for each line;
        - a ``(xy, (width, height))`` pair of the lower-left corner and size of the
          rotated, *text-aligned* text box (i.e. describing how to draw the
          text-surrounding box).

        pltrs (Rust) lays the text out, in pixels relative to its position,
        measuring its lines through the text metrics cache of *renderer*.
        """
        with _measuring_with(renderer):
            extents, lines, frame = _text.get_layout(self._native)
        x0, y0, x1, y1 = extents
        return Bbox([[x0, y0], [x1, y1]]), lines, frame

    def get_color(self):
        """Return the color of the text."""
        return _values.given(self, "color", _text.get_color(self._native),
                             same=_values.same_color)

    def get_fontproperties(self):
        """Return the `.font_manager.FontProperties`."""
        current = FontProperties._from_native(_text.get_fontproperties(self._native))
        kept = self.__dict__.get("_font_object")
        if kept is None or kept != current or not _mirrors.is_attached(kept):
            kept = self._font_object = _mirrors.attach_apart(current, _TextFont(self))
        return kept

    @property
    def _fontproperties(self):
        return self.get_fontproperties()

    def get_fontfamily(self):
        """
        Return the list of font families used for font lookup.

        See Also
        --------
        .font_manager.FontProperties.get_family
        """
        return self.get_fontproperties().get_family()

    def get_fontname(self):
        """
        Return the font name as a string.

        See Also
        --------
        .font_manager.FontProperties.get_name
        """
        return self.get_fontproperties().get_name()

    def get_fontstyle(self):
        """
        Return the font style as a string.

        See Also
        --------
        .font_manager.FontProperties.get_style
        """
        return self.get_fontproperties().get_style()

    def get_fontsize(self):
        """
        Return the font size as an integer.

        See Also
        --------
        .font_manager.FontProperties.get_size_in_points
        """
        return self.get_fontproperties().get_size_in_points()

    def get_fontvariant(self):
        """
        Return the font variant as a string.

        See Also
        --------
        .font_manager.FontProperties.get_variant
        """
        return self.get_fontproperties().get_variant()

    def get_fontweight(self):
        """
        Return the font weight as a string or a number.

        See Also
        --------
        .font_manager.FontProperties.get_weight
        """
        return self.get_fontproperties().get_weight()

    def get_stretch(self):
        """
        Return the font stretch as a string or a number.

        See Also
        --------
        .font_manager.FontProperties.get_stretch
        """
        return self.get_fontproperties().get_stretch()

    def get_horizontalalignment(self):
        """
        Return the horizontal alignment as a string.  Will be one of
        'left', 'center' or 'right'.
        """
        return _text.get_horizontalalignment(self._native)

    def get_unitless_position(self):
        """Return the (x, y) unitless position of the text."""
        # A coordinate pltrs (Rust) could not hold converts (and fails) as
        # matplotlib converts it here (see `_set_text_coordinate`).
        return tuple(
            value if (given := _unusable.kept(self, name, _NOT_KEPT)) is _NOT_KEPT
            else self._unitless_coordinate(index, given)
            for index, (name, value) in enumerate(
                zip(_POSITION, _text.get_position(self._native))))

    def get_position(self):
        """Return the (x, y) position of the text."""
        return _given_position(self, _text.get_position(self._native))

    def get_text(self):
        """Return the text string."""
        return _text.get_text(self._native)

    def get_verticalalignment(self):
        """
        Return the vertical alignment as a string.  Will be one of
        'top', 'center', 'bottom', 'baseline' or 'center_baseline'.
        """
        return _text.get_verticalalignment(self._native)

    def get_window_extent(self, renderer=None, dpi=None):
        """
        Return the `.Bbox` bounding the text, in display units.

        In addition to being used internally, this is useful for specifying
        clickable regions in a png file on a web page.

        Parameters
        ----------
        renderer : Renderer, optional
            A renderer is needed to compute the bounding box.  If the artist
            has already been drawn, the renderer is cached; thus, it is only
            necessary to pass this argument when calling `get_window_extent`
            before the first draw.  In practice, it is usually easier to
            trigger a draw first, e.g. by calling
            `~.Figure.draw_without_rendering` or ``plt.show()``.

        dpi : float, optional
            The dpi value for computing the bbox, defaults to
            ``self.get_figure(root=True).dpi`` (*not* the renderer dpi); should be set
            e.g. if to match regions with a figure saved with a custom dpi value.
        """
        if not self.get_visible():
            return Bbox.unit()
        if dpi is None:
            _unusable.check(self, measuring=True)
        figure = self.get_figure(root=True)
        if figure is None and dpi is None and not self.__dict__.get("_stand_in"):
            # matplotlib reads the resolution of the text's figure first.
            raise AttributeError("'NoneType' object has no attribute 'dpi'")
        if self.get_text() == '':
            # (A text outside any figure, such as the stand-in a replaced
            # `Text.draw` draws, is measured at its own place.)
            with (contextlib.nullcontext() if dpi is None or figure is None
                  else cbook._setattr_cm(figure, dpi=dpi)):
                tx, ty = self.get_transform().transform(self.get_unitless_position())
                return Bbox.from_bounds(tx, ty, 0, 0)
        renderer = self._measuring_renderer(renderer, figure)
        if renderer is None:
            return self._native_extent(self._text_extent, dpi)
        laid_out_at = dpi if dpi is not None or figure is None else figure.dpi
        with _measured_by(renderer, laid_out_at):
            return self._native_extent(self._text_extent, dpi)

    def _measuring_renderer(self, renderer, figure):
        """The renderer the text is measured with: *renderer* when given,
        else the one it was last measured with unless it was drawn since,
        else its figure's, which it keeps (matplotlib's ``Text._renderer``,
        which ``draw`` sets too)."""
        draws = None if self._figure is None else _artist.draw_count(self._native)
        if renderer is None:
            kept = self.__dict__.get("_measured_with")
            if kept is not None and kept[1] == draws:
                return kept[0]
            if figure is None:
                return None
            renderer = _layout_renderer(figure)
        self._measured_with = (renderer, draws)
        # matplotlib looks the renderer's metrics up in a weak cache
        # (``_get_text_metrics_function``), which refuses what cannot be
        # referred to weakly.
        weakref.ref(renderer)
        return renderer

    def _text_extent(self):
        """The display box of the text where its transform puts it now (the
        figure not laid out first), as ``(x0, y0, x1, y1)`` or None."""
        points = _artist.window_extent_as_is(self._native)
        return None if points is None else tuple(np.ravel(points))

    def _native_extent(self, measure, dpi=None):
        """The box *measure* gives, measured at *dpi* (default: the
        figure's), the unit box when it gives none."""
        figure = self.get_figure(root=True)
        if dpi is None or dpi == figure.dpi:
            extent = measure()
        else:
            old_dpi = figure.dpi
            figure.dpi = dpi
            try:
                extent = measure()
            finally:
                figure.dpi = old_dpi
        if extent is None:
            self._raise_layout_error()
            return Bbox.unit()
        x0, y0, x1, y1 = extent
        return Bbox([[x0, y0], [x1, y1]])

    def _raise_layout_error(self):
        """Raise what keeps pltrs (Rust) from measuring the text, as
        matplotlib's measuring raises it: the `FileNotFoundError` of a font
        file the text's properties name (``fname``) that is not there
        (``FT2Font`` opening it), else the text's layout error (malformed
        mathtext's `ValueError`)."""
        path = self.get_fontproperties().get_file()
        if path is not None and not os.path.exists(path):
            raise FileNotFoundError(errno.ENOENT, os.strerror(errno.ENOENT), os.fspath(path))
        _text.get_layout(self._native)

    def get_tightbbox(self, renderer=None):
        if (not self.get_visible() or self.get_text() == ""
                or not _text.in_data_domain(self._native)):
            return Bbox.null()
        return self.get_window_extent(renderer)

    def draw(self, renderer):
        # docstring inherited
        # matplotlib draws no empty text, which it returns from before it
        # converts the position.
        if self.get_text() != '':
            super().draw(renderer)

    def set_color(self, color):
        """
        Set the foreground color of the text

        Parameters
        ----------
        color : :mpltype:`color`
        """
        from .colors import _check_color_like
        if not cbook._str_equal(color, "auto"):
            _check_color_like(color=color)
        _text.set_color(self._native, color)
        _values.remember(self, "color", color)

    def set_horizontalalignment(self, align):
        """
        Set the horizontal alignment relative to the anchor point.

        See also :doc:`/gallery/text_labels_and_annotations/text_alignment`.

        Parameters
        ----------
        align : {'left', 'center', 'right'}
        """
        _api.check_in_list(['center', 'right', 'left'], align=align)
        _text.set_horizontalalignment(self._native, align)

    def set_multialignment(self, align):
        """
        Set the text alignment for multiline texts.

        The layout of the bounding box of all the lines is determined by the
        horizontalalignment and verticalalignment properties. This property
        controls the alignment of the text lines within that box.

        Parameters
        ----------
        align : {'left', 'right', 'center'}
        """
        _api.check_in_list(['center', 'right', 'left'], align=align)
        _text.set_multialignment(self._native, align)

    def set_linespacing(self, spacing):
        """
        Set the line spacing.

        Parameters
        ----------
        spacing : 'normal' or float, default: 'normal'
            If 'normal', then the line spacing is automatically determined by font
            metrics for each line individually.

            If a float, then line spacing will be fixed to this multiple of the font
            size for every line.
        """
        if cbook._str_equal(spacing, 'normal'):
            _text.set_linespacing(self._native, None)
        else:
            _api.check_isinstance(Real, spacing=spacing)
            _text.set_linespacing(self._native, float(spacing))
            _values.remember(self, "linespacing", spacing)

    def get_linespacing(self):
        """Get the line spacing."""
        spacing = _text.get_linespacing(self._native)
        return ('normal' if spacing is None
                else _values.given(self, "linespacing", spacing))

    def _update_font(self, method, value):
        """Change the font properties with their setter *method*."""
        fp = self.get_fontproperties()
        getattr(fp, method)(value)
        _text.set_fontproperties(self._native, FontManager._props(fp))

    def set_fontfamily(self, fontname):
        """
        Set the font family.  Can be either a single string, or a list of
        strings in decreasing priority.  Each string may be either a real font
        name or a generic font class name.  If the latter, the specific font
        names will be looked up in the corresponding rcParams.

        If a `Text` instance is constructed with ``fontfamily=None``, then the
        font is set to :rc:`font.family`, and the
        same is done when `set_fontfamily()` is called on an existing
        `Text` instance.

        Parameters
        ----------
        fontname : {FONTNAME, 'serif', 'sans-serif', 'cursive', 'fantasy', \
'monospace'}

        See Also
        --------
        .font_manager.FontProperties.set_family
        """
        self._update_font("set_family", fontname)

    def set_fontvariant(self, variant):
        """
        Set the font variant.

        Parameters
        ----------
        variant : {'normal', 'small-caps'}

        See Also
        --------
        .font_manager.FontProperties.set_variant
        """
        self._update_font("set_variant", variant)

    def set_fontstyle(self, fontstyle):
        """
        Set the font style.

        Parameters
        ----------
        fontstyle : {'normal', 'italic', 'oblique'}

        See Also
        --------
        .font_manager.FontProperties.set_style
        """
        self._update_font("set_style", fontstyle)

    def set_fontsize(self, fontsize):
        """
        Set the font size.

        Parameters
        ----------
        fontsize : float or {'xx-small', 'x-small', 'small', 'medium', \
'large', 'x-large', 'xx-large'}
            If a float, the fontsize in points. The string values denote sizes
            relative to the default font size.

        See Also
        --------
        .font_manager.FontProperties.set_size
        """
        self._update_font("set_size", fontsize)

    def get_math_fontfamily(self):
        """
        Return the font family name for math text rendered by Matplotlib.

        The default value is :rc:`mathtext.fontset`.

        See Also
        --------
        set_math_fontfamily
        """
        return self.get_fontproperties().get_math_fontfamily()

    def set_math_fontfamily(self, fontfamily):
        """
        Set the font family for math text rendered by Matplotlib.

        This does only affect Matplotlib's own math renderer. It has no effect
        when rendering with TeX (``usetex=True``).

        Parameters
        ----------
        fontfamily : str
            The name of the font family.

            Available font families are defined in the
            :ref:`default matplotlibrc file
            <customizing-with-matplotlibrc-files>`.

        See Also
        --------
        get_math_fontfamily
        """
        self._update_font("set_math_fontfamily", fontfamily)

    def set_fontweight(self, weight):
        """
        Set the font weight.

        Parameters
        ----------
        weight : {a numeric value in range 0-1000, 'ultralight', 'light', \
'normal', 'regular', 'book', 'medium', 'roman', 'semibold', 'demibold', \
'demi', 'bold', 'heavy', 'extra bold', 'black'}

        See Also
        --------
        .font_manager.FontProperties.set_weight
        """
        self._update_font("set_weight", weight)

    def set_fontstretch(self, stretch):
        """
        Set the font stretch (horizontal condensation or expansion).

        Parameters
        ----------
        stretch : {a numeric value in range 0-1000, 'ultra-condensed', \
'extra-condensed', 'condensed', 'semi-condensed', 'normal', 'semi-expanded', \
'expanded', 'extra-expanded', 'ultra-expanded'}

        See Also
        --------
        .font_manager.FontProperties.set_stretch
        """
        self._update_font("set_stretch", stretch)

    def set_position(self, xy):
        """
        Set the (*x*, *y*) position of the text.

        Parameters
        ----------
        xy : (float, float)
        """
        self.set_x(xy[0])
        self.set_y(xy[1])

    def set_x(self, x):
        """
        Set the *x* position of the text.

        Parameters
        ----------
        x : float
        """
        self._set_text_coordinate(0, x)

    def set_y(self, y):
        """
        Set the *y* position of the text.

        Parameters
        ----------
        y : float
        """
        self._set_text_coordinate(1, y)

    def _set_text_coordinate(self, index, value, axes=None):
        """
        Set the coordinate *index* of the position (0 for x, 1 for y) to
        *value*, which reads back as given. matplotlib keeps it unchecked
        and converts it wherever it draws or measures the text
        (`_unitless_coordinate`, with the units of *axes*, by default the
        text's): a value that fails to convert is kept (see `._unusable`),
        and the text fails there as matplotlib's does.
        """
        try:
            number = self._unitless_coordinate(index, value, axes)
        except (TypeError, ValueError):
            self._keep_unusable_coordinate(index, value)
        else:
            position = list(_text.get_position(self._native))
            position[index] = number
            _text.set_position(self._native, *position)
            self._remember_coordinate(index, value)
            _unusable.drop(self, _POSITION[index])
        self.stale = True

    def _keep_unusable_coordinate(self, index, value):
        """Keep *value*, the coordinate *index* of the position, which
        pltrs (Rust) cannot hold (see `._unusable`)."""
        name = _POSITION[index]
        _values.keep(self, name, value)
        _unusable.keep(self, name, value,
                       lambda given: self._unitless_coordinate(index, given))

    def _remember_coordinate(self, index, value):
        """Have the coordinate *index* of the position read back as
        *value*: a number while pltrs (Rust) still holds it, a value in
        units whatever it converts to."""
        if units._is_natively_supported(value):
            _values.remember(self, _POSITION[index], value)
        else:
            _values.keep(self, _POSITION[index], value)

    def _unitless_coordinate(self, index, value, axes=None):
        """*value*, the coordinate *index* of the position, as matplotlib
        draws it: NaN when masked, else converted with the units of *axes*
        (by default the text's) into a float."""
        if np.ma.is_masked(value):
            return np.nan
        artist = self if axes is None else axes
        convert = artist.convert_xunits if index == 0 else artist.convert_yunits
        return float(convert(value))

    def set_rotation(self, s):
        """
        Set the rotation of the text.

        Parameters
        ----------
        s : float or {'vertical', 'horizontal'}
            The rotation angle in degrees in mathematically positive direction
            (counterclockwise). 'horizontal' equals 0, 'vertical' equals 90.
        """
        _text.set_rotation(self._native, _rotation_degrees(s))

    def set_transform_rotates_text(self, t):
        """
        Whether rotations of the transform affect the text direction.

        Parameters
        ----------
        t : bool
        """
        _text.set_transform_rotates_text(self._native, bool(t))
        _values.remember(self, "transform_rotates_text", t)

    def set_verticalalignment(self, align):
        """
        Set the vertical alignment relative to the anchor point.

        See also :doc:`/gallery/text_labels_and_annotations/text_alignment`.

        Parameters
        ----------
        align : {'baseline', 'bottom', 'center', 'center_baseline', 'top'}
        """
        _api.check_in_list(
            ['top', 'bottom', 'center', 'baseline', 'center_baseline'],
            align=align)
        _text.set_verticalalignment(self._native, align)

    def set_text(self, s):
        r"""
        Set the text string *s*.

        It may contain newlines (``\n``) or math in LaTeX syntax.

        Parameters
        ----------
        s : object
            Any object gets converted to its `str` representation, except for
            ``None`` which is converted to an empty string.
        """
        _text.set_text(self._native, '' if s is None else str(s))

    # matplotlib's attribute holding the string, which programs read
    # (plotnine's layout skips texts without one).
    _text = property(get_text, set_text, doc="The text string.")

    def _preprocess_math(self, s):
        """
        Return the string *s* after mathtext preprocessing, and the kind of
        mathtext support needed.

        - If *self* is configured to use TeX, return *s* unchanged except that
          a single space gets escaped, and the flag "TeX".
        - Otherwise, if *s* is mathtext (has an even number of unescaped dollar
          signs) and ``parse_math`` is not set to False, return *s* and the
          flag True.
        - Otherwise, return *s* with dollar signs unescaped, and the flag
          False.
        """
        if self.get_usetex():
            if s == " ":
                s = r"\ "
            return s, "TeX"
        elif not self.get_parse_math():
            return s, False
        elif cbook.is_math_text(s):
            return s, True
        else:
            return s.replace(r"\$", "$"), False

    def set_fontproperties(self, fp):
        """
        Set the font properties that control the text.

        Parameters
        ----------
        fp : `.font_manager.FontProperties` or `str` or `pathlib.Path`
            If a `str`, it is interpreted as a fontconfig pattern parsed by
            `.FontProperties`.  If a `pathlib.Path`, it is interpreted as the
            absolute path to a font file.
        """
        _text.set_fontproperties(self._native, FontManager._props(fp))
        # matplotlib's text holds a copy of *fp* from now on.
        self.__dict__.pop("_font_object", None)

    @_docstring.kwarg_doc("bool, default: :rc:`text.usetex`")
    def set_usetex(self, usetex):
        """
        Parameters
        ----------
        usetex : bool or None
            Whether to render using TeX, ``None`` means to use
            :rc:`text.usetex`.
        """
        if usetex is None:
            usetex = _rc('text.usetex')
        _text.set_usetex(self._native, bool(usetex))

    def get_usetex(self):
        """Return whether this `Text` object uses TeX for rendering."""
        return _text.get_usetex(self._native)

    def set_parse_math(self, parse_math):
        """
        Override switch to disable any mathtext parsing for this `Text`.

        Parameters
        ----------
        parse_math : bool
            If False, this `Text` will never use mathtext.  If True, mathtext
            will be used if there is an even number of unescaped dollar signs.
        """
        _text.set_parse_math(self._native, bool(parse_math))

    def get_parse_math(self):
        """Return whether mathtext parsing is considered for this `Text`."""
        return _text.get_parse_math(self._native)

    def set_fontname(self, fontname):
        """
        Alias for `set_fontfamily`.

        One-way alias only: the getter differs.

        Parameters
        ----------
        fontname : {FONTNAME, 'serif', 'sans-serif', 'cursive', 'fantasy', \
'monospace'}

        See Also
        --------
        .font_manager.FontProperties.set_family

        """
        self.set_fontfamily(fontname)

    def get_fontfeatures(self):
        """
        Return a tuple of font feature tags to enable.
        """
        return getattr(self, "_features", None)

    def set_fontfeatures(self, features):
        """
        Set the feature tags to enable on the font.

        Parameters
        ----------
        features : list of str, or tuple of str, or None
            A list of feature tags to be used with the associated font. These strings
            are eventually passed to HarfBuzz, and so all `string formats supported by
            hb_feature_from_string()
            <https://harfbuzz.github.io/harfbuzz-hb-common.html#hb-feature-from-string>`__
            are supported. Note though that subranges are not explicitly supported and
            behaviour may change in the future.

            For example, if your desired font includes Stylistic Sets which enable
            various typographic alternates including one that you do not wish to use
            (e.g., Contextual Ligatures), then you can pass the following to enable one
            and not the other::

                fp.set_features([
                    'ss01',  # Use Stylistic Set 1.
                    '-clig',  # But disable Contextural Ligatures.
                ])

            Available font feature tags may be found at
            https://learn.microsoft.com/en-us/typography/opentype/spec/featurelist
        """
        _api.check_isinstance((Sequence, None), features=features)
        if features is not None:
            features = tuple(features)
        # Tags that are no strings are kept unchecked, as matplotlib keeps
        # them until it lays the text out.
        if features is None or all(isinstance(tag, str) for tag in features):
            _text.set_fontfeatures(self._native,
                                   None if features is None else list(features))
        self._features = features

    def get_language(self):
        """Return the language this Text is in."""
        tag, ranges = _text.get_language(self._native)
        return tuple(ranges) if ranges else tag

    def set_language(self, language):
        """
        Set the language of the text.

        Parameters
        ----------
        language : str or None
            The language of the text in a format accepted by libraqm, namely `a BCP47
            language code <https://www.w3.org/International/articles/language-tags/>`_.

            If None, then defaults to :rc:`text.language`.
        """
        _api.check_isinstance((Sequence, str, None), language=language)
        if language is None:
            language = _rc('text.language')

        if not cbook.is_scalar_or_string(language):
            language = tuple(language)
            for val in language:
                if not isinstance(val, tuple) or len(val) != 3:
                    raise TypeError('language must be list of tuple, not {language!r}')
                sublang, start, end = val
                if not isinstance(sublang, str):
                    raise TypeError(
                        'sub-language specification must be str, not {sublang!r}')
                if not isinstance(start, int):
                    raise TypeError('start location must be int, not {start!r}')
                if not isinstance(end, int):
                    raise TypeError('end location must be int, not {end!r}')
            _text.set_language(self._native, None, list(language))
        else:
            _text.set_language(self._native, language, [])

    def _ha_for_angle(self, angle):
        """
        Determines horizontal alignment ('ha') for rotation_mode "xtick" based
        on the angle of rotation in degrees and the vertical alignment.
        """
        anchor_at_bottom = self.get_verticalalignment() == 'bottom'
        if (angle <= 10 or 85 <= angle <= 95 or 350 <= angle or
                170 <= angle <= 190 or 265 <= angle <= 275):
            return 'center'
        elif 10 < angle < 85 or 190 < angle < 265:
            return 'left' if anchor_at_bottom else 'right'
        return 'right' if anchor_at_bottom else 'left'

    def _va_for_angle(self, angle):
        """
        Determines vertical alignment ('va') for rotation_mode "ytick" based
        on the angle of rotation in degrees and the horizontal alignment.
        """
        anchor_at_left = self.get_horizontalalignment() == 'left'
        if (angle <= 10 or 350 <= angle or 170 <= angle <= 190
                or 80 <= angle <= 100 or 260 <= angle <= 280):
            return 'center'
        elif 190 < angle < 260 or 10 < angle < 80:
            return 'baseline' if anchor_at_left else 'top'
        return 'top' if anchor_at_left else 'baseline'


class OffsetFrom:
    """Callable helper class for working with `Annotation`."""

    def __init__(self, artist, ref_coord, unit="points"):
        """
        Parameters
        ----------
        artist : `~matplotlib.artist.Artist` or `.BboxBase` or `.Transform`
            The object to compute the offset from.

        ref_coord : (float, float)
            If *artist* is an `.Artist` or `.BboxBase`, this values is
            the location to of the offset origin in fractions of the
            *artist* bounding box.

            If *artist* is a transform, the offset origin is the
            transform applied to this value.

        unit : {'points, 'pixels'}, default: 'points'
            The screen units to use (pixels or points) for the offset input.
        """
        self._artist = artist
        x, y = ref_coord  # Make copy when ref_coord is an array (and check the shape).
        self._ref_coord = x, y
        self.set_unit(unit)

    def set_unit(self, unit):
        """
        Set the unit for input to the transform used by ``__call__``.

        Parameters
        ----------
        unit : {'points', 'pixels'}
        """
        _api.check_in_list(["points", "pixels"], unit=unit)
        self._unit = unit

    def get_unit(self):
        """Return the unit for input to the transform used by ``__call__``."""
        return self._unit

    def __call__(self, renderer):
        """
        Return the offset transform.

        Parameters
        ----------
        renderer : `RendererBase`
            The renderer to use to compute the offset

        Returns
        -------
        `Transform`
            Maps (x, y) in pixel or point units to screen units
            relative to the given artist.
        """
        from .transforms import Affine2D
        if isinstance(self._artist, Artist):
            bbox = self._artist.get_window_extent(renderer)
            xf, yf = self._ref_coord
            x = bbox.x0 + bbox.width * xf
            y = bbox.y0 + bbox.height * yf
        elif isinstance(self._artist, BboxBase):
            bbox = self._artist
            xf, yf = self._ref_coord
            x = bbox.x0 + bbox.width * xf
            y = bbox.y0 + bbox.height * yf
        elif isinstance(self._artist, Transform):
            x, y = self._artist.transform(self._ref_coord)
        else:
            _api.check_isinstance((Artist, BboxBase, Transform), artist=self._artist)
        scale = 1 if self._unit == "pixels" else renderer.points_to_pixels(1)
        return Affine2D().scale(scale).translate(x, y)

    def _native_coords(self, figure):
        """The coordinates as `_native.annotation` takes them: pltrs (Rust)
        offsets from an artist itself, and from a box or a transform
        through the transform this gives (see `._text_coords`)."""
        if isinstance(self._artist, Artist):
            return ("offset_from", self._artist._native, self._ref_coord,
                    self._unit)
        return _text_coords.native(self, figure)


def _native_coords(coords, figure, name="xycoords"):
    """
    *coords*, an annotation coordinate system of an annotation in
    *figure* (None for one outside any figure), in the form
    `_native.annotation` takes: a name, ``("blended", x, y)``,
    ``("artist", native)``, ``("transform", native)`` or an `OffsetFrom`'s
    description.
    """
    if isinstance(coords, str):
        _annotation.check_coords(coords)
        return coords
    if isinstance(coords, tuple):
        xcoords, ycoords = coords
        return ("blended", _native_coords(xcoords, figure, name),
                _native_coords(ycoords, figure, name))
    if isinstance(coords, OffsetFrom):
        return coords._native_coords(figure)
    if isinstance(coords, Artist):
        return ("artist", coords._native)
    if isinstance(coords, Transform):
        return ("transform", _live_transforms.to_native(coords))
    if isinstance(coords, BboxBase):
        from .transforms import BboxTransformTo
        return ("transform", _live_transforms.to_native(BboxTransformTo(coords)))
    if callable(coords):
        return _text_coords.native(coords, figure)
    raise TypeError(
        f"'xycoords' must be an instance of str, tuple[str, str], Artist, "
        f"Transform, or Callable, not a {type(coords).__name__}")


def _coordinate(value):
    """A coordinate of an annotated point as `_native.annotation` takes
    it: a number, a category name, or ``("dates", number, zone)`` for a
    date (`.dates._axis_zone`)."""
    if isinstance(value, (str, bytes)):
        return value.decode() if isinstance(value, bytes) else value
    if isinstance(value, (datetime.date, np.datetime64)):
        from . import dates
        return ("dates", float(dates.date2num(value)), dates._axis_zone(value))
    return float(value)


#: The names `_values` keeps a text's coordinates under.
_POSITION = ("x", "y")

#: What `_unusable.kept` answers for a coordinate it does not keep.
_NOT_KEPT = object()


def _given_position(text, native):
    """
    The position *native* pltrs (Rust) holds for *text* as matplotlib
    keeps it: each coordinate as it was given (`_remember_position`)
    while pltrs (Rust) still holds it, as a numpy float once pltrs (Rust)
    moved the text from there (as matplotlib's layouts move titles and
    tick labels to the numpy floats they compute), and as a float when
    none was given.
    """
    given = text.__dict__.get("_given_values", {})
    return tuple(
        value if name not in given
        else _values.given(text, name, np.float64(value))
        for name, value in zip(_POSITION, native))


def _remember_position(text, xy):
    """Keep the coordinates of *xy* as *text*'s position reads them back
    (`_given_position`), as matplotlib keeps the objects it is given."""
    _values.remember_all(text, dict(zip(_POSITION, xy)), _POSITION)


def _place(text, xy):
    """Give the new *text* its position *xy*, kept as matplotlib keeps it
    (see `Text._set_text_coordinate`)."""
    x, y = xy
    text._set_text_coordinate(0, x)
    text._set_text_coordinate(1, y)


def _point(xy):
    """The point *xy* of an annotation, coordinate by coordinate."""
    x, y = xy
    return (_coordinate(x), _coordinate(y))


def _is_offset(coords):
    """Whether *coords* (or one of its components) is an offset."""
    if isinstance(coords, tuple):
        return any(map(_is_offset, coords))
    return isinstance(coords, str) and coords.startswith("offset")


def _native_arrowprops(arrowprops):
    """*arrowprops* as `_native.annotation` takes them: aliases resolved,
    styles as their descriptions and artists as their native objects."""
    from .patches import ArrowStyle, ConnectionStyle, FancyArrowPatch
    from ._patch_styles import _native_spec
    props = cbook.normalize_kwargs(arrowprops, FancyArrowPatch)
    props.pop("frac", None)
    if "arrowstyle" in props:
        props["arrowstyle"] = _native_spec(props["arrowstyle"], ArrowStyle)
    if "connectionstyle" in props:
        props["connectionstyle"] = _native_spec(
            props["connectionstyle"], ConnectionStyle)
    from ._text_box import _TextBox
    for key in ("patchA", "patchB"):
        patch = props.get(key)
        if patch is not None:
            # A text's box clips as the text draws it: pltrs (Rust) clips to
            # the box of a text it is given.
            mirror = patch.__dict__.get("_mirror")
            props[key] = (mirror.text if isinstance(mirror, _TextBox) else patch)._native
    return props


class _AnnotationBase(CoordsMixin):
    """
    What `Annotation` and `.AnnotationBbox` share (matplotlib's
    ``_AnnotationBase``): their coordinate systems, and dragging them with
    the mouse.
    """

    def set_annotation_clip(self, b):
        """
        Set the annotation's clipping behavior.

        Parameters
        ----------
        b : bool or None
            - True: The annotation will be clipped when ``self.xy`` is
              outside the Axes.
            - False: The annotation will always be drawn.
            - None: The annotation will be clipped when ``self.xy`` is
              outside the Axes and ``self.xycoords == "data"``.
        """
        self._set_native_annotation_clip(b)

    def get_annotation_clip(self):
        """
        Return the annotation's clipping behavior.

        See `set_annotation_clip` for the meaning of return values.
        """
        return self._native_annotation_clip()

    def draggable(self, state=None, use_blit=False):
        """
        Set whether the annotation is draggable with the mouse.

        Parameters
        ----------
        state : bool or None
            - True or False: set the draggability.
            - None: toggle the draggability.
        use_blit : bool, default: False
            Use blitting for faster image composition. For details see
            :ref:`func-animation`.

        Returns
        -------
        DraggableAnnotation or None
            If the annotation is draggable, the corresponding
            `.DraggableAnnotation` helper is returned.
        """
        from .offsetbox import DraggableAnnotation
        is_draggable = self._draggable is not None
        if state is None:
            state = not is_draggable
        if state:
            if self._draggable is None:
                self._draggable = DraggableAnnotation(self, use_blit)
        else:
            if self._draggable is not None:
                self._draggable.disconnect()
            self._draggable = None
        return self._draggable


class Annotation(PositionedMixin, KeptMixin, Text, _AnnotationBase):
    """
    An `.Annotation` is a `.Text` that can refer to a specific position *xy*.
    Optionally an arrow pointing from the text to *xy* can be drawn.

    Attributes
    ----------
    xy
        The annotated position.
    xycoords
        The coordinate system for *xy*.
    arrow_patch
        A `.FancyArrowPatch` to point from *xytext* to *xy*.
    """

    def set_figure(self, fig):
        # docstring inherited: matplotlib's override has none.
        super().set_figure(fig)

    def __str__(self):
        return f"Annotation({self.xy[0]:g}, {self.xy[1]:g}, {self.get_text()!r})"

    def __init__(self, text, xy,
                 xytext=None,
                 xycoords='data',
                 textcoords=None,
                 arrowprops=None,
                 annotation_clip=None,
                 **kwargs):
        """
        Annotate the point *xy* with text *text*.

        In the simplest form, the text is placed at *xy*.

        Optionally, the text can be displayed in another position *xytext*.
        An arrow pointing from the text to the annotated point *xy* can then
        be added by defining *arrowprops*.

        Parameters
        ----------
        text : str
            The text of the annotation.

        xy : (float, float)
            The point *(x, y)* to annotate. The coordinate system is determined
            by *xycoords*.

        xytext : (float, float), default: *xy*
            The position *(x, y)* to place the text at. The coordinate system
            is determined by *textcoords*.

        xycoords : single or two-tuple of str or `.Artist` or `.Transform` or \
callable, default: 'data'

            The coordinate system that *xy* is given in. The following types
            of values are supported:

            - One of the following strings:

              ==================== ============================================
              Value                Description
              ==================== ============================================
              'figure points'      Points from the lower left of the figure
              'figure pixels'      Pixels from the lower left of the figure
              'figure fraction'    Fraction of figure from lower left
              'subfigure points'   Points from the lower left of the subfigure
              'subfigure pixels'   Pixels from the lower left of the subfigure
              'subfigure fraction' Fraction of subfigure from lower left
              'axes points'        Points from lower left corner of the Axes
              'axes pixels'        Pixels from lower left corner of the Axes
              'axes fraction'      Fraction of Axes from lower left
              'data'               Use the coordinate system of the object
                                   being annotated (default)
              'polar'              *(theta, r)* if not native 'data'
                                   coordinates
              ==================== ============================================

              Note that 'subfigure pixels' and 'figure pixels' are the same
              for the parent figure, so users who want code that is usable in
              a subfigure can use 'subfigure pixels'.

            - An `.Artist`: *xy* is interpreted as a fraction of the artist's
              `~matplotlib.transforms.Bbox`. E.g. *(0, 0)* would be the lower
              left corner of the bounding box and *(0.5, 1)* would be the
              center top of the bounding box.

            - A `.Transform` to transform *xy* to screen coordinates.

            - A function with one of the following signatures::

                def transform(renderer) -> Bbox
                def transform(renderer) -> Transform

              where *renderer* is a `.RendererBase` subclass.

              The result of the function is interpreted like the `.Artist` and
              `.Transform` cases above.

            - A tuple *(xcoords, ycoords)* specifying separate coordinate
              systems for *x* and *y*. *xcoords* and *ycoords* must each be
              of one of the above described types.

            See :ref:`plotting-guide-annotation` for more details.

        textcoords : single or two-tuple of str or `.Artist` or `.Transform` \
or callable, default: value of *xycoords*
            The coordinate system that *xytext* is given in.

            All *xycoords* values are valid as well as the following strings:

            =================   =================================================
            Value               Description
            =================   =================================================
            'offset points'     Offset, in points, from the *xy* value
            'offset pixels'     Offset, in pixels, from the *xy* value
            'offset fontsize'   Offset, relative to fontsize, from the *xy* value
            =================   =================================================

        arrowprops : dict, optional
            The properties used to draw a `.FancyArrowPatch` arrow between the
            positions *xy* and *xytext*.  Defaults to None, i.e. no arrow is
            drawn.

            For historical reasons there are two different ways to specify
            arrows, "simple" and "fancy":

            **Simple arrow:**

            If *arrowprops* does not contain the key 'arrowstyle' the
            allowed keys are:

            ==========  =================================================
            Key         Description
            ==========  =================================================
            width       The width of the arrow in points
            headwidth   The width of the base of the arrow head in points
            headlength  The length of the arrow head in points
            shrink      Fraction of total length to shrink from both ends
            ?           Any `.FancyArrowPatch` property
            ==========  =================================================

            The arrow is attached to the edge of the text box, the exact
            position (corners or centers) depending on where it's pointing to.

            **Fancy arrow:**

            This is used if 'arrowstyle' is provided in the *arrowprops*.

            Valid keys are the following `.FancyArrowPatch` parameters:

            ===============  ===================================
            Key              Description
            ===============  ===================================
            arrowstyle       The arrow style
            connectionstyle  The connection style
            relpos           See below; default is (0.5, 0.5)
            patchA           Default is bounding box of the text
            patchB           Default is None
            shrinkA          In points. Default is 2 points
            shrinkB          In points. Default is 2 points
            mutation_scale   Default is text size (in points)
            mutation_aspect  Default is 1
            ?                Any `.FancyArrowPatch` property
            ===============  ===================================

            The exact starting point position of the arrow is defined by
            *relpos*. It's a tuple of relative coordinates of the text box,
            where (0, 0) is the lower left corner and (1, 1) is the upper
            right corner. Values <0 and >1 are supported and specify points
            outside the text box. By default (0.5, 0.5), so the starting point
            is centered in the text box.

        annotation_clip : bool or None, default: None
            Whether to clip (i.e. not draw) the annotation when the annotation
            point *xy* is outside the Axes area.

            - If *True*, the annotation will be clipped when *xy* is outside
              the Axes.
            - If *False*, the annotation will always be drawn.
            - If *None*, the annotation will be clipped when *xy* is outside
              the Axes and *xycoords* is 'data'.

        **kwargs
            Additional kwargs are passed to `.Text`.

        Returns
        -------
        `.Annotation`

        See Also
        --------
        :ref:`annotations`

        """
        x, y = xy  # matplotlib unpacks both points as it makes the annotation
        if _is_offset(xycoords):
            raise ValueError("xycoords cannot be an offset coordinate")
        warn_textcoords(xytext, textcoords, xycoords)
        position = (x, y) if xytext is None else xytext
        tx, ty = position
        self._native = _annotation.new(
            '' if text is None else str(text),
            annotate_arguments((x, y), xytext, xycoords, textcoords,
                               arrowprops, annotation_clip, None))
        Artist.__init__(self)
        self._init_annotation((x, y), (tx, ty), xycoords, textcoords, arrowprops, kwargs)

    def _init_annotation(self, xy, position, xycoords, textcoords, arrowprops, kwargs):
        """Record what the annotation was made with (its point *xy*, its
        text *position*, ...), then apply its text properties *kwargs*, as
        ``Annotation.__init__`` does."""
        self._xycoords = xycoords
        self._textcoords = xycoords if textcoords is None else textcoords
        self._keep_given(xy, position)
        self.arrowprops = arrowprops
        self._draggable = None
        self._init_properties(kwargs)
        self._track_coords()

    @property
    def _evaluates_coords(self):
        """Whether a coordinate system of the annotation is one the program
        evaluates at every draw (see `._text_coords`)."""
        return (_text_coords.is_evaluated(self._xycoords)
                or _text_coords.is_evaluated(self._textcoords))

    def _track_coords(self):
        """Have the figure of the annotation evaluate its coordinate
        systems at every draw when the program evaluates one (an
        annotation outside any figure has them evaluated once adopted)."""
        if self._figure is not None:
            from ._computed_transforms import track
            track(self, self._figure)

    def _evaluate_coords(self):
        """Hand pltrs (Rust) the coordinate systems the program evaluates
        as they are now."""
        if _text_coords.is_evaluated(self._xycoords):
            _annotation.set_xycoords(
                self._native, _native_coords(self._xycoords, self.figure))
        if _text_coords.is_evaluated(self._textcoords):
            _annotation.set_textcoords(
                self._native,
                _native_coords(self._textcoords, self.figure, "textcoords"))

    def get_window_extent(self, renderer=None):
        # Return the `.Bbox` bounding the text and the arrow, in display
        # units. The renderer is not used: pltrs (Rust) measures them.
        if not self.get_visible() or (self._figure is not None
                                      and not self._check_xy(renderer)):
            return Bbox.unit()
        self.update_positions(renderer)
        # matplotlib's measures the text then (``Text.get_window_extent``).
        self._measuring_renderer(renderer, self.get_figure(root=True))
        self._measured()
        return self._native_extent(lambda: Text._text_extent(self))

    def contains(self, mouseevent):
        # (pltrs (Rust) hit-tests the text and its arrow.)
        return super().contains(mouseevent)

    def get_tightbbox(self, renderer=None):
        # docstring inherited
        if not self._check_xy(renderer):
            return Bbox.null()
        return super().get_tightbbox(renderer)

    def draw(self, renderer):
        # docstring inherited
        # (Its arrow is drawn whatever its text, not `Text.draw`.)
        Artist.draw(self, renderer)

    def _text_extent(self):
        # The text alone, without the arrow (``Text.get_window_extent``).
        return _annotation.text_extent(self._native)

    @property
    def arrow_patch(self):
        """
        A `.FancyArrowPatch` to point from *xytext* to *xy*, None without
        *arrowprops*.

        pltrs: the patch has the arrow's look (styles, colours, alpha,
        path effects), and its setters change the arrow; the annotation
        places it when drawn (its `~.Artist.get_window_extent` is where the
        arrow is drawn).
        """
        from ._annotation_arrow import mirror
        return mirror(self)

    @property
    def xy(self):
        """The annotated position, in `xycoords`."""
        return _values.given(self, "xy", _annotation.get_xy(self._native))

    @xy.setter
    def xy(self, xy):
        self._hold_xy(xy)

    @property
    def xycoords(self):
        return self._xycoords

    @xycoords.setter
    def xycoords(self, xycoords):
        if _is_offset(xycoords):
            raise ValueError("xycoords cannot be an offset coordinate")
        self._hold_coords("xycoords", xycoords)
        self._xycoords = xycoords
        self._track_coords()

    @property
    def xyann(self):
        """
        The text position.

        See also *xytext* in `.Annotation`.
        """
        return self.get_position()

    @xyann.setter
    def xyann(self, xytext):
        self.set_position(xytext)

    def get_anncoords(self):
        """
        Return the coordinate system to use for `.Annotation.xyann`.

        See also *xycoords* in `.Annotation`.
        """
        return self._textcoords

    def set_anncoords(self, coords):
        """
        Set the coordinate system to use for `.Annotation.xyann`.

        See also *xycoords* in `.Annotation`.
        """
        self._hold_coords("anncoords", coords)
        self._textcoords = coords
        self._track_coords()

    anncoords = property(get_anncoords, set_anncoords, doc="""
        The coordinate system to use for `.Annotation.xyann`.""")

    def _set_native_annotation_clip(self, b):
        """Give pltrs (Rust)'s annotation the clipping behavior *b*."""
        _annotation.set_annotation_clip(self._native, b)

    def _native_annotation_clip(self):
        """The clipping behavior of pltrs (Rust)'s annotation."""
        return _annotation.get_annotation_clip(self._native)

    def update_positions(self, renderer):
        """
        Update the pixel positions of the annotation text and the arrow patch.
        """
        # pltrs (Rust) places them whenever the annotation is drawn or
        # measured; the annotation goes stale, as matplotlib's sets its
        # transform anew. What matplotlib's computes with fails as there:
        # the text coordinates, and for the arrow the text's extent (with
        # *renderer*) and the point.
        _unusable.check(self, measuring=True)
        if self.arrowprops is not None:
            self._measuring_renderer(renderer, self.get_figure(root=True))
        self.stale = True


# pltrs (Rust) measures and draws texts through the replacements of
# _get_text_metrics_with_cache and Text.draw the program makes.
_text_overrides.watch(sys.modules[__name__], Text)


_docstring.interpd.register(Annotation=Annotation.__init__.__doc__)


# What matplotlib's module also has from its imports, which programs
# import from it.
import functools  # noqa: E402, F401
import itertools  # noqa: E402, F401
import logging  # noqa: E402, F401
import math  # noqa: E402, F401
from . import artist, colors as mcolors  # noqa: E402, F401
from .font_manager import get_font  # noqa: E402, F401
from .patches import FancyArrowPatch, FancyBboxPatch, Rectangle  # noqa: E402, F401
from .transforms import Affine2D, BboxTransformTo, IdentityTransform  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
