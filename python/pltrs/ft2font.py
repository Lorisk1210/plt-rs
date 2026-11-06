"""
Font rendering (matplotlib's ``ft2font``): pltrs (Rust) ports FreeType itself,
in the version matplotlib's baseline images are checked against here.

`FT2Font` is a face of a font file as FreeType opens it: pltrs-font reads,
hints and lays out its glyphs and pltrs-render rasterizes them
(``_native.ft2font``). The flag and enumeration classes have FreeType's
values, as matplotlib's (which takes them from FreeType's headers).
"""

from enum import Enum, Flag
import io
import os

import numpy as np

from . import _api
from ._native import ft2font as _ft2font

#: The FreeType version pltrs (Rust) ports.
__freetype_version__ = "2.14.3"
#: How the FreeType matplotlib's baseline images need was built: pltrs
#: (Rust) is that version, built in.
__freetype_build_type__ = "local"
#: The libraqm version whose layout pltrs (Rust) follows.
__libraqm_version__ = "0.10.5"

#: How many bytes a font file object is read by at a time.
_READ_SIZE = 1 << 16

#: Character codes are plain ints.
CharacterCodeType = int
#: Glyph indices are plain ints.
GlyphIndexType = int


class FaceFlags(Flag):
    """Flags returned by `FT2Font.face_flags` (FreeType's ``FT_FACE_FLAG_*``)."""

    SCALABLE = 1 << 0
    FIXED_SIZES = 1 << 1
    FIXED_WIDTH = 1 << 2
    SFNT = 1 << 3
    HORIZONTAL = 1 << 4
    VERTICAL = 1 << 5
    KERNING = 1 << 6
    FAST_GLYPHS = 1 << 7
    MULTIPLE_MASTERS = 1 << 8
    GLYPH_NAMES = 1 << 9
    EXTERNAL_STREAM = 1 << 10
    HINTER = 1 << 11
    CID_KEYED = 1 << 12
    TRICKY = 1 << 13
    COLOR = 1 << 14
    VARIATION = 1 << 15
    SVG = 1 << 16
    SBIX = 1 << 17
    SBIX_OVERLAY = 1 << 18


class Kerning(Enum):
    """Kerning modes for `FT2Font.get_kerning` (FreeType's ``FT_KERNING_*``)."""

    DEFAULT = 0
    UNFITTED = 1
    UNSCALED = 2


class LoadFlags(Flag):
    """Flags for `FT2Font.load_char`, `FT2Font.load_glyph` and
    `FT2Font.set_text` (FreeType's ``FT_LOAD_*``)."""

    DEFAULT = 0
    NO_SCALE = 1 << 0
    NO_HINTING = 1 << 1
    RENDER = 1 << 2
    NO_BITMAP = 1 << 3
    VERTICAL_LAYOUT = 1 << 4
    FORCE_AUTOHINT = 1 << 5
    CROP_BITMAP = 1 << 6
    PEDANTIC = 1 << 7
    IGNORE_GLOBAL_ADVANCE_WIDTH = 1 << 9
    NO_RECURSE = 1 << 10
    IGNORE_TRANSFORM = 1 << 11
    MONOCHROME = 1 << 12
    LINEAR_DESIGN = 1 << 13
    NO_AUTOHINT = 1 << 15
    COLOR = 1 << 20
    COMPUTE_METRICS = 1 << 21
    BITMAP_METRICS_ONLY = 1 << 22
    NO_SVG = 1 << 24
    # The following should be unique, but the above can be OR'd together.
    TARGET_NORMAL = 0 << 16
    TARGET_LIGHT = 1 << 16
    TARGET_MONO = 2 << 16
    TARGET_LCD = 3 << 16
    TARGET_LCD_V = 4 << 16


class RenderMode(Enum):
    """Render modes (FreeType's ``FT_RENDER_MODE_*``)."""

    NORMAL = 0
    LIGHT = 1
    MONO = 2
    LCD = 3
    LCD_V = 4
    SDF = 5


class StyleFlags(Flag):
    """Flags returned by `FT2Font.style_flags` (FreeType's
    ``FT_STYLE_FLAG_*``)."""

    NORMAL = 0
    ITALIC = 1 << 0
    BOLD = 1 << 1


def _flags_value(flags, name="flags"):
    """The integer value of load *flags*, a `LoadFlags` or (deprecated) an
    int."""
    if isinstance(flags, LoadFlags):
        return flags.value
    if isinstance(flags, int) and not isinstance(flags, bool):
        _api.warn_deprecated("3.10", name=name, obj_type="parameter as int",
                             alternative="LoadFlags enum values")
        return flags
    raise TypeError(f"{name} must be LoadFlags or int")


def _incompatible(function):
    """pybind11's `TypeError` for arguments of the wrong types."""
    return TypeError(
        f"{function}(): incompatible constructor arguments. The following "
        "argument types are supported")


def _shaping(features, language):
    """The native form of `set_text`'s *features* and *language*."""
    if features is not None:
        if isinstance(features, (str, bytes)) or not hasattr(features, '__iter__'):
            raise TypeError("features must be a sequence of str")
        features = list(features)
        if not all(isinstance(feature, str) for feature in features):
            raise TypeError("features must be a sequence of str")
    if language is None:
        ranges = []
    elif isinstance(language, str):
        ranges = [(language, 0, 2**31 - 1)]
    else:
        ranges = []
        for item in language:
            if (not isinstance(item, tuple) or len(item) != 3
                    or not isinstance(item[0], str)
                    or not isinstance(item[1], int)
                    or not isinstance(item[2], int)):
                raise TypeError(
                    "language must be a str or a list of (str, int, int)")
            ranges.append(item)
    return features, ranges


def _read_only(name, doc):
    """A read-only property giving the value *name* of the object's
    ``_values`` (as matplotlib's C++ classes expose their fields)."""
    return property(lambda self: self._values[name], doc=doc)


def _not_constructible(cls, args, kwargs):
    """Fail as constructing *cls*, a class matplotlib's pybind11 extension
    makes no constructor of, fails: with no arguments, `RuntimeError`;
    with some, pybind11's `TypeError`."""
    if not args and not kwargs:
        raise RuntimeError(f"{cls.__name__} is not constructible")
    given = [", ".join(map(repr, args))] if args else []
    if kwargs:
        given.append("kwargs: " + ", ".join(f"{k}={v!r}" for k, v in kwargs.items()))
    raise TypeError(
        "__init__(): incompatible constructor arguments. The following argument "
        f"types are supported:\n    1. {cls.__module__}.{cls.__qualname__}()\n\n"
        f"Invoked with: {'; '.join(given)}")


class Glyph:
    """
    Information about a single glyph.

    You cannot create instances of this object yourself, but must use
    `.FT2Font.load_char` or `.FT2Font.load_glyph` to generate one. This object may
    be used in a call to `.FT2Font.draw_glyph_to_bitmap`.

    For more information on the various metrics, see `the FreeType documentation
    <https://freetype.org/freetype2/docs/glyphs/glyphs-3.html>`_.
    """

    def __init__(self, *args, **kwargs):
        _not_constructible(type(self), args, kwargs)

    @classmethod
    def _loaded(cls, metrics, index):
        """The glyph `FT2Font.load_char` or `.load_glyph` loaded with
        *metrics* as the *index*-th of the font's glyphs."""
        self = cls.__new__(cls)
        ((width, height, horiBearingX, horiBearingY),
         (horiAdvance, linearHoriAdvance),
         (vertBearingX, vertBearingY, vertAdvance),
         bbox) = metrics
        self._values = dict(
            width=width, height=height, horiBearingX=horiBearingX,
            horiBearingY=horiBearingY, horiAdvance=horiAdvance,
            linearHoriAdvance=linearHoriAdvance, vertBearingX=vertBearingX,
            vertBearingY=vertBearingY, vertAdvance=vertAdvance, bbox=bbox)
        self._index = index
        return self

    width = _read_only("width", "The glyph's width.")
    height = _read_only("height", "The glyph's height.")
    horiBearingX = _read_only(
        "horiBearingX", "Left side bearing for horizontal layout.")
    horiBearingY = _read_only(
        "horiBearingY", "Top side bearing for horizontal layout.")
    horiAdvance = _read_only(
        "horiAdvance", "Advance width for horizontal layout.")
    linearHoriAdvance = _read_only(
        "linearHoriAdvance", "The advance width of the unhinted glyph.")
    vertBearingX = _read_only(
        "vertBearingX", "Left side bearing for vertical layout.")
    vertBearingY = _read_only(
        "vertBearingY", "Top side bearing for vertical layout.")
    vertAdvance = _read_only(
        "vertAdvance", "Advance height for vertical layout.")
    bbox = _read_only("bbox", "The control box of the glyph.")


class LayoutItem:
    """A glyph of laid out text, as `FT2Font._layout` yields it."""

    def __init__(self, *args, **kwargs):
        _not_constructible(type(self), args, kwargs)

    @classmethod
    def _laid_out(cls, ft_object, char, glyph_index, x, y, prev_kern):
        """The item `FT2Font._layout` made."""
        self = cls.__new__(cls)
        self._values = dict(ft_object=ft_object, char=char,
                            glyph_index=glyph_index, x=x, y=y,
                            prev_kern=prev_kern)
        return self

    ft_object = _read_only("ft_object", "The FT_Face of the item.")
    char = _read_only("char", "The character code for the item.")
    glyph_index = _read_only("glyph_index", "The glyph index for the item.")
    x = _read_only("x", "The x position of the item.")
    y = _read_only("y", "The y position of the item.")
    prev_kern = _read_only(
        "prev_kern", "The kerning between this item and the previous one.")

    def __str__(self):
        return (f"LayoutItem(ft_object={self.ft_object.fname}, "
                f"char={self.char!r}, glyph_index={self.glyph_index}, "
                f"x={self.x}, y={self.y}, prev_kern={self.prev_kern})")


class _PositionedBitmap:
    """A glyph rendered by `FT2Font._render_glyph` and where it goes."""

    def __init__(self, left, top, buffer):
        self.left = left
        self.top = top
        self.buffer = buffer


class FT2Image:
    """
    An image buffer for drawing glyphs.

    .. deprecated:: 3.11
       Use a 2D uint8 ndarray instead.
    """

    def __init__(self, width, height):
        _api.warn_deprecated("3.11", name="FT2Image", obj_type="class",
                             alternative="a 2D uint8 ndarray")
        self._buffer = np.zeros((int(height), int(width)), np.uint8)

    def draw_rect_filled(self, x0, y0, x1, y1):
        """
        Draw a filled rectangle to the image: the end points are included and
        clipped to the image.
        """
        height, width = self._buffer.shape
        self._buffer[min(y0, height):min(y1 + 1, height),
                     min(x0, width):min(x1 + 1, width)] = 255

    def __array__(self, dtype=None, copy=None):
        return self._buffer if dtype is None else self._buffer.astype(dtype)

    def __buffer__(self, flags):
        return memoryview(self._buffer)

    def __release_buffer__(self, buffer):
        buffer.release()


class FT2Font:
    """
    An object representing a single font face.

    Outside of the font itself and querying its properties, this object
    provides methods for processing text strings into glyph shapes.

    Commonly used methods include `set_text`, `get_width_height`, and
    `draw_glyphs_to_bitmap`, which in combination are used to render a
    string of text to a bitmap.
    """

    def __init__(self, filename, hinting_factor=None, *, face_index=0,
                 _fallback_list=None, _kerning_factor=None,
                 _warn_if_used=False):
        """
        Parameters
        ----------
        filename : str, bytes, os.PathLike, or io.BinaryIO
            The source of the font data in a format (ttf or ttc) that
            FreeType can read.

        face_index : int, optional
            The index of the face in the font file to load.
        """
        if hinting_factor is not None:
            if not isinstance(hinting_factor, int):
                raise _incompatible("__init__")
            _api.warn_deprecated("3.11", name="hinting_factor",
                                 obj_type="parameter")
        if _kerning_factor is not None:
            if not isinstance(_kerning_factor, int):
                raise _incompatible("__init__")
            _api.warn_deprecated("3.11", name="_kerning_factor",
                                 obj_type="parameter")
        if _fallback_list is not None and (
                not isinstance(_fallback_list, list)
                or not all(isinstance(font, FT2Font) for font in _fallback_list)):
            raise _incompatible("__init__")
        if not isinstance(face_index, int):
            raise _incompatible("__init__")
        source, self._fname = self._source(filename)
        self._fallbacks = list(_fallback_list or [])
        self._warn_if_used = _warn_if_used
        opener = _ft2font.open_file if isinstance(source, str) else _ft2font.open
        self._native = opener(source, face_index,
                              [font._native for font in self._fallbacks])
        self._face_index = face_index
        if _kerning_factor:
            _ft2font.set_kerning_factor(self._native, _kerning_factor)
        ((postscript_name, family_name, style_name),
         (num_faces, num_glyphs, num_charmaps, num_fixed_sizes),
         (face_flags, style_flags, units_per_EM,
          (underline_position, underline_thickness)),
         (ascender, descender, height,
          (max_advance_width, max_advance_height, bbox))
         ) = _ft2font.attributes(self._native)
        self._values = dict(
            postscript_name=postscript_name, family_name=family_name,
            style_name=style_name, num_faces=num_faces, num_glyphs=num_glyphs,
            num_charmaps=num_charmaps, num_fixed_sizes=num_fixed_sizes,
            face_flags=FaceFlags(face_flags),
            style_flags=StyleFlags(style_flags), units_per_EM=units_per_EM,
            underline_position=underline_position,
            underline_thickness=underline_thickness, ascender=ascender,
            descender=descender, height=height,
            max_advance_width=max_advance_width,
            max_advance_height=max_advance_height, bbox=bbox)

    postscript_name = _read_only(
        "postscript_name", "PostScript name of the font.")
    family_name = _read_only("family_name", "Face family name.")
    style_name = _read_only("style_name", "Style name.")
    num_faces = _read_only("num_faces", "Number of faces in file.")
    num_glyphs = _read_only("num_glyphs", "Number of glyphs in the face.")
    num_charmaps = _read_only("num_charmaps", "Number of charmaps in the face.")
    num_fixed_sizes = _read_only(
        "num_fixed_sizes", "Number of bitmap in the face.")
    face_flags = _read_only("face_flags", "Face flags; see `.FaceFlags`.")
    style_flags = _read_only("style_flags", "Style flags; see `.StyleFlags`.")
    units_per_EM = _read_only(
        "units_per_EM", "Number of font units covered by the EM.")
    underline_position = _read_only(
        "underline_position", "Vertical position of the underline bar.")
    underline_thickness = _read_only(
        "underline_thickness", "Thickness of the underline bar.")
    ascender = _read_only("ascender", "Ascender in 26.6 units.")
    descender = _read_only("descender", "Descender in 26.6 units.")
    height = _read_only(
        "height", "Height in 26.6 units; used to compute a default line "
        "spacing (baseline-to-baseline distance).")
    max_advance_width = _read_only(
        "max_advance_width", "Maximum horizontal cursor advance for all glyphs.")
    max_advance_height = _read_only(
        "max_advance_height", "Maximum vertical cursor advance for all glyphs.")
    bbox = _read_only(
        "bbox", "Face global bounding box (xmin, ymin, xmax, ymax).")

    @staticmethod
    def _source(filename):
        """
        The font file *filename* (a path, or a binary file object) as pltrs
        (Rust) opens it: a path, whose bytes it reads once for the faces of
        the file open, or the bytes a file object gives; and the name
        `fname` reports for it.
        """
        if isinstance(filename, (str, bytes, os.PathLike)):
            # Opened as FreeType opens it, for its errors and name.
            with io.open(filename, "rb") as file:
                return os.fsdecode(filename), file.name
        try:
            probe = filename.read(0)
        except Exception:
            probe = None
        if not isinstance(probe, bytes):
            raise TypeError("First argument must be a path to a font file or a "
                            "binary-mode file object")
        chunks = []
        while chunk := filename.read(_READ_SIZE):
            if not isinstance(chunk, bytes) or len(chunk) > _READ_SIZE:
                # FreeType sizes its buffer for the bytes it asks for.
                raise RuntimeError(
                    "In FT2Font: Can not load face (invalid stream operation; "
                    "error code 0x55)")
            chunks.append(chunk)
        return b"".join(chunks), filename

    def __buffer__(self, flags):
        return memoryview(self._image())

    def __release_buffer__(self, buffer):
        buffer.release()

    def _image(self):
        """The image `draw_glyphs_to_bitmap` drew."""
        return _ft2font.get_image(self._native)

    @property
    def fname(self):
        """The original filename for this object."""
        return self._fname

    @property
    def face_index(self):
        """The index of the font in the file."""
        return self._face_index

    @property
    def scalable(self):
        """Whether face is scalable; attributes after this one are only
        defined for scalable faces."""
        return FaceFlags.SCALABLE in self.face_flags

    @property
    def num_named_instances(self):
        """Number of named instances in the face (none: no variable fonts)."""
        return 0

    def clear(self):
        """Clear all the glyphs, reset for a new call to `.set_text`."""
        _ft2font.clear(self._native)
        for font in self._fallbacks:
            font.clear()

    def set_size(self, ptsize, dpi):
        """
        Set the size of the text.

        Parameters
        ----------
        ptsize : float
            The size of the text in points.
        dpi : float
            The DPI used for rendering the text.
        """
        _ft2font.set_size(self._native, float(ptsize), float(dpi))

    def _set_transform(self, matrix, delta):
        """Set the transform glyphs are rendered with by `_render_glyph`:
        a 16.16 matrix and a 26.6 translation."""
        (xx, xy), (yx, yy) = np.asarray(matrix, dtype=np.int64).tolist()
        dx, dy = (int(v) for v in delta)
        _ft2font.set_transform(self._native, ((xx, xy), (yx, yy)), (dx, dy))

    def set_charmap(self, i):
        """
        Make the i-th charmap current.

        Parameters
        ----------
        i : int
            The charmap number in the range [0, `.num_charmaps`).
        """
        _ft2font.set_charmap(self._native, int(i))

    def select_charmap(self, i):
        """
        Select a charmap by its FT_Encoding number.

        Parameters
        ----------
        i : int
            The charmap in the form defined by FreeType.
        """
        _ft2font.select_charmap(self._native, int(i))

    def get_charmap(self):
        """Return a mapping of character codes to glyph indices in the
        font."""
        return dict(_ft2font.get_charmap(self._native))

    def get_char_index(self, codepoint):
        """Return the glyph index corresponding to a character code
        point."""
        return _ft2font.get_char_index(self._native, int(codepoint))

    def get_kerning(self, left, right, mode):
        """
        Get the kerning between two glyphs.

        Parameters
        ----------
        left, right : int
            The glyph indices. Note these are not characters nor character
            codes.
        mode : Kerning
            A kerning mode constant.

        Returns
        -------
        int
            The kerning adjustment between the two glyphs.
        """
        if isinstance(mode, Kerning):
            mode = mode.value
        elif isinstance(mode, int):
            _api.warn_deprecated("3.10", name="mode", obj_type="parameter as int",
                                 alternative="Kerning enum values")
        else:
            raise TypeError("mode must be Kerning or int")
        return _ft2font.get_kerning(self._native, int(left), int(right), mode)

    def get_glyph_name(self, index):
        """Retrieve the ASCII name of a given glyph *index* in a face."""
        return _ft2font.get_glyph_name(self._native, int(index))

    def get_name_index(self, name):
        """Return the glyph index of a given glyph *name*; 0 means
        'undefined character code'."""
        return _ft2font.get_name_index(self._native, name)

    def get_num_glyphs(self):
        """Return the number of loaded glyphs."""
        return _ft2font.get_num_glyphs(self._native)

    def get_sfnt(self):
        """
        Load the entire SFNT names table.

        Returns
        -------
        dict[tuple[int, int, int, int], bytes]
            The SFNT names table; the dictionary keys are tuples of
            (platform-ID, ISO-encoding-scheme, language-code, description)
            and the values are the direct information from the font table.
        """
        return dict(_ft2font.get_sfnt(self._native))

    def get_sfnt_table(self, name):
        """
        Return one of the SFNT tables: 'head', 'maxp', 'OS/2', 'hhea',
        'vhea', 'post' or 'pclt', as a dict, or None if the font lacks it.
        """
        table = _ft2font.get_sfnt_table(self._native, name)
        return None if table is None else dict(table)

    def get_ps_font_info(self):
        """Return the information in the PS Font Info structure."""
        return _ft2font.get_ps_font_info(self._native)

    def _missing(self, codepoint):
        """Warn that no face of the font has *codepoint*."""
        from ._text_helpers import warn_on_missing_glyph
        names = [self.family_name, *(font.family_name for font in self._fallbacks)]
        warn_on_missing_glyph(codepoint, ", ".join(names))

    def load_char(self, charcode, flags=LoadFlags.FORCE_AUTOHINT):
        """
        Load character in current fontfile and set glyph.

        Parameters
        ----------
        charcode : int
            The character code to prepare rendering information for.
        flags : LoadFlags, default: `.LoadFlags.FORCE_AUTOHINT`
            Any bitwise-OR combination of the `.LoadFlags` flags.

        Returns
        -------
        Glyph
            The glyph information corresponding to the specified character.
        """
        loaded = _ft2font.load_char(self._native, int(charcode),
                                    _flags_value(flags))
        if loaded is None:
            self._missing(charcode)
            # matplotlib's message names the line of ft2font.cpp raising it.
            raise RuntimeError(
                "charcode loading (ft2font.cpp line 544) failed to find glyph to load")
        metrics, index, _face = loaded
        return Glyph._loaded(metrics, index)

    def load_glyph(self, glyph_index, flags=LoadFlags.FORCE_AUTOHINT):
        """
        Load glyph index in current fontfile and set glyph.

        Parameters
        ----------
        glyph_index : int
            The glyph index to prepare rendering information for.
        flags : LoadFlags, default: `.LoadFlags.FORCE_AUTOHINT`
            Any bitwise-OR combination of the `.LoadFlags` flags.

        Returns
        -------
        Glyph
            The glyph information corresponding to the specified index.
        """
        metrics, index, _face = _ft2font.load_glyph(
            self._native, int(glyph_index), _flags_value(flags))
        return Glyph._loaded(metrics, index)

    def set_text(self, string, angle=0.0, flags=LoadFlags.FORCE_AUTOHINT, *,
                 features=None, language=None):
        """
        Set the text *string* and *angle*.

        You must call this before `.draw_glyphs_to_bitmap`.

        Returns
        -------
        np.ndarray[double]
            A sequence of x,y glyph positions in 26.6 subpixels; divide by
            64 for pixels.
        """
        shaping = _shaping(features, language)
        flags = _flags_value(flags)
        for codepoint in map(ord, string):
            if (codepoint > 31 and self.get_char_index(codepoint) == 0
                    and not any(font.get_char_index(codepoint)
                                for font in self._fallbacks)):
                self._missing(codepoint)
        for font in self._fallbacks:
            font.clear()
        xys = _ft2font.set_text(self._native, string, float(angle), flags,
                                shaping)
        return np.array(xys, dtype=float).reshape(-1, 2)

    def _layout(self, text, flags, features=None, language=None):
        """
        Lay out *text* with the font and its fallbacks.

        Returns
        -------
        list[LayoutItem]
        """
        shaping = _shaping(features, language)
        fonts = [self, *self._fallbacks]
        for font in self._fallbacks:
            font.clear()
        items = []
        for (face, start, end), glyph, (x, y), kern in _ft2font.layout(
                self._native, text, _flags_value(flags), shaping):
            # Laying out loads each glyph into the font it comes from.
            fonts[face].load_glyph(glyph, flags)
            items.append(LayoutItem._laid_out(fonts[face], text[start:end], glyph,
                                              x, y, kern))
        return items

    def get_width_height(self):
        """
        Get the dimensions of the current string set by `.set_text`.

        Returns
        -------
        width, height : float
            The width and height in 26.6 subpixels of the current string.
        """
        return _ft2font.get_width_height(self._native)

    def get_descent(self):
        """Get the descent of the current string set by `.set_text`, in
        26.6 subpixels."""
        return _ft2font.get_descent(self._native)

    def get_bitmap_offset(self):
        """
        Get the (x, y) offset for the bitmap if ink hangs left or below
        (0, 0), in 26.6 subpixels.
        """
        return _ft2font.get_bitmap_offset(self._native)

    def draw_glyphs_to_bitmap(self, antialiased=True):
        """
        Draw the glyphs that were loaded by `.set_text` to the bitmap.

        The bitmap size will be automatically set to include the glyphs.
        """
        _ft2font.draw_glyphs_to_bitmap(self._native, bool(antialiased))

    def get_image(self):
        """Return the underlying image buffer for this font object."""
        return self._image()

    def draw_glyph_to_bitmap(self, image, x, y, glyph, antialiased=True):
        """
        Draw a single glyph to the bitmap at pixel locations x, y.

        Parameters
        ----------
        image : 2d array of uint8
            The image buffer on which to draw the glyph.
        x, y : int
            The pixel location at which to draw the glyph.
        glyph : Glyph
            The glyph to draw.
        antialiased : bool, default: True
            Whether to render glyphs 8-bit antialiased or in pure
            black-and-white.
        """
        target = np.asarray(image)
        drawn = _ft2font.draw_glyph_to_bitmap(
            self._native, target, (int(x), int(y)), glyph._index,
            bool(antialiased))
        target[...] = drawn

    def _render_glyph(self, index, flags, render_mode):
        """Render glyph *index* at the transform `_set_transform` set."""
        mono = render_mode == RenderMode.MONO
        (left, top), buffer = _ft2font.render_glyph(
            self._native, int(index), _flags_value(flags), mono)
        return _PositionedBitmap(left, top, buffer)

    def get_path(self):
        """
        Get the path data from the currently loaded glyph.

        Returns
        -------
        vertices : np.ndarray[double]
            The (N, 2) array of vertices describing the current glyph.
        codes : np.ndarray[np.uint8]
            The (N, ) array of codes corresponding to the vertices.
        """
        vertices, codes = _ft2font.get_path(self._native)
        return vertices, codes.astype(np.uint8)


#: The deprecated module attributes and what replaces them.
_DEPRECATED = {
    **{f"KERNING_{mode.name}": (mode.value, f"Kerning.{mode.name}")
       for mode in Kerning},
    **{f"LOAD_{name}": (flag, f"LoadFlags.{name}")
       for name, flag in LoadFlags.__members__.items()},
    **{name: (FaceFlags[name], f"FaceFlags.{name}")
       for name in ["SCALABLE", "FIXED_SIZES", "FIXED_WIDTH", "SFNT",
                    "HORIZONTAL", "VERTICAL", "KERNING", "FAST_GLYPHS",
                    "MULTIPLE_MASTERS", "GLYPH_NAMES", "EXTERNAL_STREAM"]},
    **{name: (StyleFlags[name], f"StyleFlags.{name}")
       for name in ["ITALIC", "BOLD"]},
}


def __getattr__(name):
    if name in _DEPRECATED:
        value, alternative = _DEPRECATED[name]
        _api.warn_deprecated("3.10", name=name, obj_type="attribute",
                             alternative=alternative)
        return value
    raise AttributeError(f"module 'matplotlib.ft2font' has no attribute {name!r}")
