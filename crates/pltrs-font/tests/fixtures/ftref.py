"""A ctypes binding to FreeType, for the fixtures of variable fonts at
instances matplotlib's `FT2Font` cannot select.

The library is FreeType 2.14.3, matplotlib's, built as a shared library
(`./configure --without-zlib --without-bzip2 --without-png
--without-harfbuzz --without-brotli && make`) and named by the
`FREETYPE_LIB` environment variable. Only LP64 hosts are supported.
"""

import ctypes
import os
from ctypes import (POINTER, Structure, byref, c_char_p, c_int, c_long,
                    c_short, c_ubyte, c_uint, c_ushort, c_void_p)

NO_SCALE = 1
NO_HINTING = 2
NO_AUTOHINT = 0x8000
FORCE_AUTOHINT = 0x20


class Vector(Structure):
    _fields_ = [("x", c_long), ("y", c_long)]


class Generic(Structure):
    _fields_ = [("data", c_void_p), ("finalizer", c_void_p)]


class BBox(Structure):
    _fields_ = [(name, c_long) for name in ("xMin", "yMin", "xMax", "yMax")]


class GlyphMetrics(Structure):
    _fields_ = [(name, c_long) for name in (
        "width", "height", "horiBearingX", "horiBearingY", "horiAdvance",
        "vertBearingX", "vertBearingY", "vertAdvance")]


class Bitmap(Structure):
    _fields_ = [("rows", c_uint), ("width", c_uint), ("pitch", c_int),
                ("buffer", c_void_p), ("num_grays", c_ushort),
                ("pixel_mode", c_ubyte), ("palette_mode", c_ubyte),
                ("palette", c_void_p)]


class Outline(Structure):
    _fields_ = [("n_contours", c_ushort), ("n_points", c_ushort),
                ("points", POINTER(Vector)), ("tags", POINTER(c_ubyte)),
                ("contours", POINTER(c_ushort)), ("flags", c_int)]


class GlyphSlot(Structure):
    _fields_ = [("library", c_void_p), ("face", c_void_p), ("next", c_void_p),
                ("glyph_index", c_uint), ("generic", Generic),
                ("metrics", GlyphMetrics), ("linearHoriAdvance", c_long),
                ("linearVertAdvance", c_long), ("advance", Vector),
                ("format", c_uint), ("bitmap", Bitmap), ("bitmap_left", c_int),
                ("bitmap_top", c_int), ("outline", Outline),
                ("num_subglyphs", c_uint), ("subglyphs", c_void_p),
                ("control_data", c_void_p), ("control_len", c_long),
                ("lsb_delta", c_long), ("rsb_delta", c_long),
                ("other", c_void_p), ("internal", c_void_p)]


class FaceRec(Structure):
    _fields_ = [("num_faces", c_long), ("face_index", c_long),
                ("face_flags", c_long), ("style_flags", c_long),
                ("num_glyphs", c_long), ("family_name", c_char_p),
                ("style_name", c_char_p), ("num_fixed_sizes", c_int),
                ("available_sizes", c_void_p), ("num_charmaps", c_int),
                ("charmaps", c_void_p), ("generic", Generic), ("bbox", BBox),
                ("units_per_EM", c_ushort), ("ascender", c_short),
                ("descender", c_short), ("height", c_short),
                ("max_advance_width", c_short), ("max_advance_height", c_short),
                ("underline_position", c_short),
                ("underline_thickness", c_short),
                ("glyph", POINTER(GlyphSlot)), ("size", c_void_p),
                ("charmap", c_void_p)]


def _library():
    lib = ctypes.CDLL(os.environ["FREETYPE_LIB"])
    lib.FT_New_Face.argtypes = [c_void_p, c_char_p, c_long, POINTER(POINTER(FaceRec))]
    for name in ("FT_Set_Char_Size", "FT_Load_Glyph", "FT_Set_Var_Design_Coordinates",
                 "FT_Get_Var_Blend_Coordinates", "FT_Set_Named_Instance"):
        getattr(lib, name).restype = c_int
    lib.FT_Set_Char_Size.argtypes = [POINTER(FaceRec), c_long, c_long, c_uint, c_uint]
    lib.FT_Load_Glyph.argtypes = [POINTER(FaceRec), c_uint, c_int]
    lib.FT_Set_Var_Design_Coordinates.argtypes = [POINTER(FaceRec), c_uint, POINTER(c_long)]
    lib.FT_Get_Var_Blend_Coordinates.argtypes = [POINTER(FaceRec), c_uint, POINTER(c_long)]
    lib.FT_Set_Named_Instance.argtypes = [POINTER(FaceRec), c_uint]
    library = c_void_p()
    if lib.FT_Init_FreeType(byref(library)):
        raise RuntimeError("FT_Init_FreeType failed")
    return lib, library


_LIB, _LIBRARY = _library()


class Face:
    """A face of a font file, at an instance and a size."""

    def __init__(self, path, index=0):
        self.face = POINTER(FaceRec)()
        if _LIB.FT_New_Face(_LIBRARY, os.fsencode(path), index, byref(self.face)):
            raise RuntimeError(f"cannot open {path}")

    def set_design(self, coords):
        """Sets the design coordinates (floats, axis order)."""
        values = (c_long * len(coords))(*[round(v * 65536) for v in coords])
        if _LIB.FT_Set_Var_Design_Coordinates(self.face, len(coords), values):
            raise RuntimeError("FT_Set_Var_Design_Coordinates failed")

    def set_named_instance(self, index):
        """Selects named instance `index` (1-based; 0 for the default)."""
        if _LIB.FT_Set_Named_Instance(self.face, index):
            raise RuntimeError("FT_Set_Named_Instance failed")

    def normalized(self, count):
        """The normalized coordinates in 16.16."""
        values = (c_long * count)()
        if _LIB.FT_Get_Var_Blend_Coordinates(self.face, count, values):
            raise RuntimeError("FT_Get_Var_Blend_Coordinates failed")
        return list(values)

    def set_size(self, points, dpi):
        if _LIB.FT_Set_Char_Size(self.face, int(points * 64), 0, dpi, dpi):
            raise RuntimeError("FT_Set_Char_Size failed")

    def metrics(self):
        """Ascender, descender and height in font units."""
        rec = self.face.contents
        return [rec.ascender, rec.descender, rec.height]

    def load(self, glyph, flags):
        """The outline (points as x, y, tag and contour ends), the advance
        and the linear advance of `glyph` loaded with `flags`."""
        if _LIB.FT_Load_Glyph(self.face, glyph, flags):
            return None
        slot = self.face.contents.glyph.contents
        outline = slot.outline
        points = [[outline.points[i].x, outline.points[i].y, outline.tags[i] & 3]
                  for i in range(outline.n_points)]
        ends = [outline.contours[i] for i in range(outline.n_contours)]
        return {"points": points, "ends": ends, "advance": slot.advance.x,
                "linear": slot.linearHoriAdvance}
