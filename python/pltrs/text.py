"""
Classes for including text in a figure (matplotlib's ``text``).

pltrs (Rust) keeps every property of a text and lays it out and draws it
(``pltrs::text``); the `Text` methods here keep matplotlib's signatures,
aliases and validation, and its keyword handling.
"""

from collections.abc import Sequence
from numbers import Real

from . import _api, cbook
from ._native import text as _text
from .artist import Artist
from .font_manager import FontManager, FontProperties
from .transforms import Bbox

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


def _rc(name):
    """The rcParam *name* (the package's rcParams, imported once it exists)."""
    from . import rcParams
    return rcParams[name]


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
class Text(Artist):
    """Handle storing and drawing of text in window or data coordinates."""

    zorder = 3

    def __repr__(self):
        x, y = self.get_position()
        return f"Text({x}, {y}, {self.get_text()!r})"

    def __init__(self, x=0, y=0, text='', **kwargs):
        """
        Create a `.Text` instance at *x*, *y* with string *text*; *kwargs*
        are `.Text` properties.

        The text is outside any figure until an axes or a figure adds it
        (`~.Axes.add_artist`, `.Figure.add_artist`).
        """
        self._native = _text.new(float(x), float(y),
                                 "" if text is None else str(text))
        super().__init__()
        self._init_properties(kwargs)

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
        """
        if rectprops is None:
            _text.set_bbox(self._native, None)
            return
        props = {_BOX_ALIASES.get(key, key): value
                 for key, value in rectprops.items()}
        boxstyle = props.get("boxstyle")
        if boxstyle is not None and not isinstance(boxstyle, str):
            raise NotImplementedError(
                "pltrs: set_bbox(boxstyle=...) takes a box style name")
        _text.set_bbox(self._native, props)

    def set_backgroundcolor(self, color):
        """
        Set the background color of the text.

        This is realized through the bbox (see `.set_bbox`),
        creating the bbox patch if needed.

        Parameters
        ----------
        color : :mpltype:`color`
        """
        _text.set_backgroundcolor(self._native, color)

    def update(self, kwargs):
        """
        Update this text's properties from the dict *kwargs*: the font
        properties first, as they have the lowest priority, the box last, as
        it depends on them.
        """
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
        return _text.get_transform_rotates_text(self._native)

    def set_rotation_mode(self, m):
        """
        Set text rotation mode.

        Parameters
        ----------
        m : {None, 'default', 'anchor', 'xtick', 'ytick'}
            If ``"default"``, the text will be first rotated, then aligned
            according to their horizontal and vertical alignments.  If
            ``"anchor"``, then alignment occurs before rotation. "xtick" and
            "ytick" adjust the horizontal/vertical alignment so that the text
            is visually pointing towards its anchor point. Passing ``None``
            will set the rotation mode to ``"default"``.
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
        """
        _text.set_antialiased(self._native, bool(antialiased))

    def get_antialiased(self):
        """Return whether antialiased rendering is used."""
        return _text.get_antialiased(self._native)

    def get_wrap(self):
        """Return whether the text can be wrapped."""
        return _text.get_wrap(self._native)

    def set_wrap(self, wrap):
        """
        Set whether the text can be wrapped.

        Wrapping makes sure the text is confined to the (sub)figure box. It
        does not take into account any other artists.

        Parameters
        ----------
        wrap : bool
        """
        _text.set_wrap(self._native, bool(wrap))

    def _get_wrapped_text(self):
        """
        Return a copy of the text string with new lines added so that the
        text is wrapped relative to the parent figure (if `get_wrap` is True).
        """
        return _text.get_wrapped_text(self._native)

    def get_color(self):
        """Return the color of the text."""
        return _text.get_color(self._native)

    def get_fontproperties(self):
        """Return the `.font_manager.FontProperties`."""
        return FontProperties._from_native(
            _text.get_fontproperties(self._native))

    def get_fontfamily(self):
        """Return the list of font families used for font lookup."""
        return self.get_fontproperties().get_family()

    def get_fontname(self):
        """Return the font name as a string."""
        return self.get_fontproperties().get_name()

    def get_fontstyle(self):
        """Return the font style as a string."""
        return self.get_fontproperties().get_style()

    def get_fontsize(self):
        """Return the font size as an integer."""
        return self.get_fontproperties().get_size_in_points()

    def get_fontvariant(self):
        """Return the font variant as a string."""
        return self.get_fontproperties().get_variant()

    def get_fontweight(self):
        """Return the font weight as a string or a number."""
        return self.get_fontproperties().get_weight()

    def get_stretch(self):
        """Return the font stretch as a string or a number."""
        return self.get_fontproperties().get_stretch()

    def get_horizontalalignment(self):
        """
        Return the horizontal alignment as a string.  Will be one of
        'left', 'center' or 'right'.
        """
        return _text.get_horizontalalignment(self._native)

    def get_unitless_position(self):
        """Return the (x, y) unitless position of the text."""
        return _text.get_position(self._native)

    def get_position(self):
        """Return the (x, y) position of the text."""
        return _text.get_position(self._native)

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

        Parameters
        ----------
        renderer : Renderer, optional
            pltrs (Rust) measures the text itself: the renderer is not used.

        dpi : float, optional
            The dpi value for computing the bbox, defaults to the figure's
            dpi (*not* the renderer dpi); should be set e.g. if to match
            regions with a figure saved with a custom dpi value.
        """
        if not self.get_visible():
            return Bbox.unit()
        figure = self.get_figure(root=True)
        if dpi is None or dpi == figure.dpi:
            extent = _text.get_window_extent(self._native)
        else:
            old_dpi = figure.dpi
            figure.dpi = dpi
            try:
                extent = _text.get_window_extent(self._native)
            finally:
                figure.dpi = old_dpi
        if extent is None:
            return Bbox.unit()
        x0, y0, x1, y1 = extent
        return Bbox([[x0, y0], [x1, y1]])

    def get_tightbbox(self, renderer=None):
        """Like `get_window_extent`: a text's tight box is its extent."""
        return self.get_window_extent(renderer)

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

    def set_horizontalalignment(self, align):
        """
        Set the horizontal alignment relative to the anchor point.

        Parameters
        ----------
        align : {'left', 'center', 'right'}
        """
        _api.check_in_list(['center', 'right', 'left'], align=align)
        _text.set_horizontalalignment(self._native, align)

    def set_multialignment(self, align):
        """
        Set the text alignment for multiline texts.

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
            If 'normal', then the line spacing is automatically determined by
            font metrics for each line individually.

            If a float, then line spacing will be fixed to this multiple of
            the font size for every line.
        """
        if cbook._str_equal(spacing, 'normal'):
            _text.set_linespacing(self._native, None)
        else:
            _api.check_isinstance(Real, spacing=spacing)
            _text.set_linespacing(self._native, float(spacing))

    def get_linespacing(self):
        """Get the line spacing."""
        spacing = _text.get_linespacing(self._native)
        return 'normal' if spacing is None else spacing

    def _update_font(self, method, value):
        """Change the font properties with their setter *method*."""
        fp = self.get_fontproperties()
        getattr(fp, method)(value)
        _text.set_fontproperties(self._native, FontManager._props(fp))

    def set_fontfamily(self, fontname):
        """
        Set the font family.  Can be either a single string, or a list of
        strings in decreasing priority.  Each string may be either a real font
        name or a generic font class name.

        Parameters
        ----------
        fontname : {FONTNAME, 'serif', 'sans-serif', 'cursive', 'fantasy', \
'monospace'}
        """
        self._update_font("set_family", fontname)

    def set_fontvariant(self, variant):
        """
        Set the font variant.

        Parameters
        ----------
        variant : {'normal', 'small-caps'}
        """
        self._update_font("set_variant", variant)

    def set_fontstyle(self, fontstyle):
        """
        Set the font style.

        Parameters
        ----------
        fontstyle : {'normal', 'italic', 'oblique'}
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
        """
        self._update_font("set_size", fontsize)

    def get_math_fontfamily(self):
        """
        Return the font family name for math text rendered by Matplotlib.

        The default value is :rc:`mathtext.fontset`.
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
        _text.set_position(self._native, float(x), self.get_position()[1])

    def set_y(self, y):
        """
        Set the *y* position of the text.

        Parameters
        ----------
        y : float
        """
        _text.set_position(self._native, self.get_position()[0], float(y))

    def set_rotation(self, s):
        """
        Set the rotation of the text.

        Parameters
        ----------
        s : float or {'vertical', 'horizontal'}
            The rotation angle in degrees in mathematically positive direction
            (counterclockwise). 'horizontal' equals 0, 'vertical' equals 90.
        """
        if isinstance(s, Real):
            rotation = float(s) % 360
        elif cbook._str_equal(s, 'horizontal') or s is None:
            rotation = 0.
        elif cbook._str_equal(s, 'vertical'):
            rotation = 90.
        else:
            raise ValueError("rotation must be 'vertical', 'horizontal' or "
                             f"a number, not {s}")
        _text.set_rotation(self._native, rotation)

    def set_transform_rotates_text(self, t):
        """
        Whether rotations of the transform affect the text direction.

        Parameters
        ----------
        t : bool
        """
        _text.set_transform_rotates_text(self._native, bool(t))

    def set_verticalalignment(self, align):
        """
        Set the vertical alignment relative to the anchor point.

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
            A list of feature tags to be used with the associated font, in
            the string formats HarfBuzz's ``hb_feature_from_string()``
            supports.
        """
        _api.check_isinstance((Sequence, None), features=features)
        if features is not None:
            features = tuple(features)
        _text.set_fontfeatures(self._native,
                               None if features is None else list(features))
        self._features = features

    def get_language(self):
        """Return the language this Text is in."""
        return getattr(self, "_language", _rc('text.language'))

    def set_language(self, language):
        """
        Set the language of the text.

        Parameters
        ----------
        language : str or None
            The language of the text in a format accepted by libraqm, namely
            `a BCP47 language code
            <https://www.w3.org/International/articles/language-tags/>`_.

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
        self._language = language

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
