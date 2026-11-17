"""
What matplotlib's `.RendererAgg` does with the arguments of its drawing
methods before it draws, for `.RendererAgg` to do before pltrs (Rust)
draws: the same attributes read and the same values computed, in the
same order, so that wrong arguments fail as there, with matplotlib's
errors. Its Python methods (``draw_text``, ``draw_mathtext``,
``draw_tex``, which draw glyph by glyph through
``_draw_text_glyphs_and_boxes``) are ported from matplotlib 3.11.2's
``backends/backend_agg.py``, and the conversions of its extension's
arguments from ``src/_backend_agg_wrapper.cpp``,
``src/_backend_agg_basic_types.h`` and ``src/py_converters.h``, under the
Matplotlib License.
"""

import math

import numpy as np

from .. import _pybind, rcParams
from ..font_manager import fontManager, get_font
from ..mathtext import MathTextParser


def rgba(value):
    """Convert *value* as the extension converts a colour
    (``type_caster<agg::rgba>``): None, or a sequence of 3 or 4 numbers."""
    if value is None:
        return
    components = tuple(value)
    for index in range(3):
        _cast(components[index], _pybind.double)
    if len(components) not in (3, 4):
        raise ValueError("RGBA value must be 3- or 4-tuple")
    if len(components) == 4:
        _cast(components[3], _pybind.double)


def _cast(value, accepts):
    """Cast *value* as pybind11 does with the caster *accepts*."""
    if not accepts(value):
        raise _pybind.cast_error(value)


def graphics_context(gc):
    """Convert *gc* as the extension converts a graphics context
    (``type_caster<GCAgg>``): its attributes and getters, in that order."""
    _cast(gc._linewidth, _pybind.double)
    _cast(gc._alpha, _pybind.double)
    _cast(gc._forced_alpha, _pybind.boolean)
    rgba(gc._rgb)
    _cast(gc._antialiased, _pybind.boolean)
    gc._capstyle  # noqa: B018
    gc._joinstyle  # noqa: B018
    gc.get_dashes()
    gc._cliprect  # noqa: B018
    gc.get_clip_path()
    gc.get_snap()
    gc.get_hatch_path()
    rgba(gc.get_hatch_color())
    _cast(gc.get_hatch_linewidth(), _pybind.double)
    gc.get_sketch_params()


def path_arguments(gc, path, transform, rgbFace):
    """The arguments of ``draw_path`` as the extension converts them: the
    graphics context, the path, the transform, then the fill colour."""
    from ._native_drawing import affine_matrix
    graphics_context(gc)
    path.vertices, path.codes, path.should_simplify, path.simplify_threshold  # noqa: B018
    affine_matrix(transform)
    rgba(rgbFace)


class AggArgumentsMixin:
    """The arguments of `.RendererAgg`'s text drawing, used as matplotlib's
    renderer uses them."""

    def _glyph_arguments(self, gc, x, y, angle, offsets):
        """What ``_draw_text_glyphs_and_boxes`` computes with *gc*, the
        baseline's start *x*, *y* and *angle* before it draws the first
        glyph, at *offsets* (the first glyph's ``(dx, dy)``, or None for
        no glyph), and after it drew them."""
        cos = math.cos(math.radians(angle))
        sin = math.sin(math.radians(angle))
        if offsets is not None:
            dx, dy = offsets
            # The glyph's transform, cast as numpy casts it.
            (0x10000 * np.array([[cos, -sin], [sin, cos]])  # noqa: B018
             @ [[1, 0], [0, 1]]).round().astype(int)
            round(0x40 * (x + dx * cos - dy * sin))
            round(0x40 * (self.height - y + dx * sin + dy * cos))
            gc.get_antialiased()
            graphics_context(gc)
        gc.get_rgb()

    def _text_arguments(self, gc, x, y, s, prop, angle, ismath, mtext):
        """The arguments of ``draw_text`` as matplotlib reads them."""
        if ismath:
            self._mathtext_arguments(gc, x, y, s, prop, angle)
            return
        from .backend_agg import get_hinting_flag
        font = get_font(fontManager._find_fonts_by_props(prop))
        font.clear()
        font.set_size(prop.get_size_in_points(), self.dpi)
        items = font._layout(
            s, flags=get_hinting_flag(),
            features=mtext.get_fontfeatures() if mtext is not None else None,
            language=mtext.get_language() if mtext is not None else None)
        prop.get_size_in_points()
        self._glyph_arguments(gc, x, y, angle,
                              (items[0].x, items[0].y) if items else None)

    def _mathtext_arguments(self, gc, x, y, s, prop, angle):
        """The arguments of ``draw_mathtext`` as matplotlib reads them."""
        parse = _MATHTEXT.parse(s, self.dpi, prop, antialiased=gc.get_antialiased())
        glyph = parse.glyphs[0] if parse.glyphs else None
        self._glyph_arguments(gc, x, y, angle,
                              None if glyph is None else glyph[4:6])

    def _tex_arguments(self, gc, x, y, s, prop, angle):
        """The arguments of ``draw_tex`` as matplotlib reads them (before
        TeX typesets *s*, and with the glyphs it typesets): TeX runs
        first, so that a missing TeX fails before the other arguments."""
        size = prop.get_size_in_points()
        texmanager = self.get_texmanager()
        if rcParams["text.latex.engine"] == "latex+dvipng":
            texmanager.get_grey(s, size, self.dpi)
        else:
            texmanager.make_dvi(s, size)
        self._glyph_arguments(gc, x, y, angle, (0.0, 0.0) if f"{s}" else None)


#: The parser of matplotlib's Agg renderer, which draws the glyphs of
#: mathtext itself (``RendererAgg.mathtext_parser``).
_MATHTEXT = MathTextParser('path')
