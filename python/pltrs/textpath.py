"""
Paths made of the outlines of text (matplotlib's ``textpath``).

pltrs (Rust) typesets and outlines the text (``pltrs::text::text_path``, and
``text_path_tex`` for TeX); this module keeps matplotlib's classes.
"""

from collections import OrderedDict

from .font_manager import FontManager, FontProperties
from .path import Path
from .transforms import Affine2D
from ._native import textpath as _textpath


def _glyph_repr(psname, glyph):
    """Return a unique id for the glyph index *glyph* of the font whose
    PostScript name is *psname* (matplotlib's ``_get_glyph_repr``)."""
    # Imported here, not with the module: `urllib.parse` costs `import
    # pltrs.pyplot` a few milliseconds, and only vector output asks.
    import urllib.parse
    return urllib.parse.quote(f"{psname}-{glyph:x}")


class TextToPath:
    """A class that converts strings to paths."""

    FONT_SCALE = 100.
    DPI = 72

    def get_text_path(self, prop, s, ismath=False, *, features=None, language=None):
        """
        Convert text *s* to path (a tuple of vertices and codes for
        matplotlib.path.Path).

        Parameters
        ----------
        prop : `~matplotlib.font_manager.FontProperties`
            The font properties for the text.
        s : str
            The text to be converted.
        ismath : {False, True, "TeX"}
            If True, use mathtext parser.  If "TeX", use tex for rendering.
        features : sequence of str, optional
            The OpenType features of the text, in HarfBuzz's syntax.
        language : str or sequence of (str, int, int), optional
            The language of the text in a format accepted by libraqm, namely `a BCP47
            language code <https://www.w3.org/International/articles/language-tags/>`_,
            or such codes for ranges of the text.

        Returns
        -------
        verts : list
            A list of arrays containing the (x, y) coordinates of the
            vertices.
        codes : list
            A list of path codes.
        """
        if language is None or isinstance(language, str):
            language = (language, [])
        else:
            language = (None, list(language))
        return _textpath.get_text_path(
            s, FontManager._props(prop), ismath == "TeX",
            None if features is None else list(features), language)

    def get_glyphs_tex(self, prop, s, glyph_map=None,
                       return_new_glyphs_only=False):
        """Convert the string *s* to vertices and codes using usetex mode."""
        glyphs, paths, boxes = _textpath.get_glyphs_tex(
            s, FontManager._props(prop))
        paths = {_glyph_repr(psname, index): (vertices, codes)
                 for psname, index, vertices, codes in paths}

        if glyph_map is None:
            glyph_map = OrderedDict()

        if return_new_glyphs_only:
            glyph_map_new = OrderedDict()
        else:
            glyph_map_new = glyph_map

        glyph_reprs, xpositions, ypositions, sizes = [], [], [], []

        for psname, index, x, y, size in glyphs:
            glyph_repr = _glyph_repr(psname, index)
            if glyph_repr not in glyph_map:
                glyph_map_new[glyph_repr] = paths[glyph_repr]

            glyph_reprs.append(glyph_repr)
            xpositions.append(x)
            ypositions.append(y)
            sizes.append(size)

        myrects = []

        for ox, oy, h, w in boxes:
            vert1 = [(ox, oy), (ox + w, oy), (ox + w, oy + h),
                     (ox, oy + h), (ox, oy), (0, 0)]
            code1 = [Path.MOVETO,
                     Path.LINETO, Path.LINETO, Path.LINETO, Path.LINETO,
                     Path.CLOSEPOLY]
            myrects.append((vert1, code1))

        return (list(zip(glyph_reprs, xpositions, ypositions, sizes)),
                glyph_map_new, myrects)


text_to_path = TextToPath()


class TextPath(Path):
    """
    Create a path from the text.
    """

    def __init__(self, xy, s, size=None, prop=None,
                 _interpolation_steps=1, usetex=False):
        r"""
        Create a path from the text. Note that it simply is a path,
        not an artist. You need to use the `.PathPatch` (or other artists)
        to draw this path onto the canvas.

        Parameters
        ----------
        xy : tuple or array of two float values
            Position of the text. For no offset, use ``xy=(0, 0)``.

        s : str
            The text to convert to a path.

        size : float, optional
            Font size in points. Defaults to the size specified via the font
            properties *prop*.

        prop : `~matplotlib.font_manager.FontProperties`, optional
            Font property. If not provided, will use a default
            `.FontProperties` with parameters from the
            :ref:`rcParams<customizing-with-dynamic-rc-settings>`.

        _interpolation_steps : int, optional
            (Currently ignored)

        usetex : bool, default: False
            Whether to use tex rendering.
        """
        prop = FontProperties._from_any(prop)
        if size is None:
            size = prop.get_size_in_points()

        self._xy = xy
        self.set_size(size)

        self._cached_vertices = None
        super().__init__(
            *text_to_path.get_text_path(
                prop, s, ismath="TeX" if usetex else False),
            _interpolation_steps=_interpolation_steps,
            readonly=True)
        self._should_simplify = False

    def set_size(self, size):
        """Set the text size."""
        self._size = size
        self._invalid = True

    def get_size(self):
        """Get the text size."""
        return self._size

    @property
    def vertices(self):
        """
        Return the cached path after updating it if necessary.
        """
        self._revalidate_path()
        return self._cached_vertices

    @property
    def codes(self):
        """
        Return the codes
        """
        return self._codes

    def _revalidate_path(self):
        """
        Update the path if necessary.

        The path for the text is initially create with the font size of
        `.FONT_SCALE`, and this path is rescaled to other size when necessary.
        """
        if self._invalid or self._cached_vertices is None:
            tr = (Affine2D()
                  .scale(self._size / text_to_path.FONT_SCALE)
                  .translate(*self._xy))
            self._cached_vertices = tr.transform(self._vertices)
            self._cached_vertices.flags.writeable = False
            self._invalid = False
