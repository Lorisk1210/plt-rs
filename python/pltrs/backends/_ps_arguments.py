"""
What matplotlib's `.RendererPS` does with the arguments of its drawing
methods and graphics state setters before it writes them, for
`.RendererPS` to do before pltrs (Rust) writes the PostScript: the same
attributes read and the same values formatted, in the same order, so that
wrong arguments fail as there, with matplotlib's errors. Ported from
matplotlib 3.11.2's ``backends/backend_ps.py`` under the Matplotlib
License.
"""

import numpy as np

from .. import _pybind
from ._native_drawing import affine_matrix

#: The PostScript codes of the line joins and caps, by matplotlib's names
#: (and the codes themselves, which it takes for backcompat).
JOINS = {'miter': 0, 'round': 1, 'bevel': 2, 0: 0, 1: 1, 2: 2}
CAPS = {'butt': 0, 'round': 1, 'projecting': 2, 0: 0, 1: 1, 2: 2}


def check_numbers(*values):
    """Format *values* as matplotlib writes the numbers of the graphics
    state (``_nums_to_str``), failing as it does on what is no number."""
    for value in values:
        f"{value:1.3f}"  # noqa: B018


def check_coordinates(*values):
    """Format *values* as matplotlib writes coordinates (``:g``), failing
    as it does on what is no number."""
    for value in values:
        f"{value:g}"  # noqa: B018


def _check_layout(s, prop, features, language):
    """Look the font of *prop* up and lay *s* out with it as matplotlib's
    PostScript renderer does (``_get_font_ttf``, then ``FT2Font._layout``
    through ``_text_helpers.layout``), for their errors."""
    from ..font_manager import FontProperties, fontManager, get_font
    from ..ft2font import FT2Font, LoadFlags
    font_prop = FontProperties._from_any(prop)
    if font_prop is not prop or not isinstance(s, str):
        # (pltrs (Rust) looks the fonts of font properties up itself.)
        font = get_font(fontManager._find_fonts_by_props(font_prop))
        prop.get_size_in_points()
        FT2Font._layout.check(font, s, LoadFlags.NO_HINTING,
                              features=features, language=language)


class PsArgumentsMixin:
    """The arguments of `.RendererPS`'s drawing methods and setters, used
    as matplotlib's renderer uses them."""

    def _check_color(self, r, g, b, store=True):
        """The arguments of ``set_color``, formatted as matplotlib writes
        them (a gray level once, else the three components)."""
        if r == g == b:
            check_numbers(r)
        else:
            check_numbers(r, g, b)

    # Colours of too few components fail as matplotlib's call of `set_color`
    # fails, naming it.
    _check_color.__qualname__ = "RendererPS.set_color"

    def _is_transparent(self, rgb_or_rgba):
        """matplotlib's test of a colour drawn as nothing (pltrs (Rust)
        warns about partial transparency as it writes the colour)."""
        if rgb_or_rgba is None:
            return True  # Consistent with rgbFace semantics.
        if len(rgb_or_rgba) == 4:
            return rgb_or_rgba[3] == 0
        return False

    def _style_arguments(self, gc, rgbFace, *, fill=True, stroke=True):
        """What matplotlib's ``_draw_ps`` reads of *gc* and *rgbFace* to
        paint a path."""
        mightstroke = (gc.get_linewidth() > 0
                       and not self._is_transparent(gc.get_rgb()))
        if self._is_transparent(rgbFace):
            fill = False
        hatch = gc.get_hatch()

        if mightstroke:
            check_numbers(float(gc.get_linewidth()))
            JOINS[gc.get_joinstyle()]  # noqa: B018
            CAPS[gc.get_capstyle()]  # noqa: B018
            offset, seq = gc.get_dashes()
            if seq is not None and len(seq):
                check_numbers(*seq)
                check_numbers(offset)
        if mightstroke or hatch:
            self._check_color(*gc.get_rgb()[:3])
        gc.get_clip_rectangle()
        gc.get_clip_path()
        if fill:
            self._check_color(*rgbFace[:3], store=False)
        if hatch:
            gc.get_hatch_linewidth()
            check_numbers(*gc.get_hatch_color()[:3])

    def _path_arguments(self, gc, path, transform, rgbFace):
        """The arguments of ``draw_path`` as matplotlib reads them."""
        if rgbFace is None and gc.get_hatch_path() is None:
            path.should_simplify  # noqa: B018  (the path clipped is simplified)
        path.vertices  # noqa: B018
        affine_matrix(transform)
        self._style_arguments(gc, rgbFace)

    def _markers_arguments(self, gc, marker_path, marker_trans, path, trans,
                           rgbFace):
        """The arguments of ``draw_markers`` as matplotlib reads them;
        whether there are markers to draw (not of no path, None, which
        matplotlib's ``convert_to_string`` takes as an empty path)."""
        if not self._is_transparent(rgbFace):
            if rgbFace[0] == rgbFace[1] == rgbFace[2]:
                check_numbers(rgbFace[0])
            else:
                check_numbers(*rgbFace[:3])
        lw = gc.get_linewidth()
        alpha = (gc.get_alpha()
                 if gc.get_forced_alpha() or len(gc.get_rgb()) == 3
                 else gc.get_rgb()[3])
        if lw > 0 and alpha > 0:
            f"{lw:.1f}"  # noqa: B018
            JOINS[gc.get_joinstyle()]  # noqa: B018
            CAPS[gc.get_capstyle()]  # noqa: B018
        _pybind.path_iterator(marker_path)
        affine_matrix(marker_trans)
        bool(rgbFace)
        for _ in path.iter_segments(trans, simplify=False):
            pass
        self._style_arguments(gc, rgbFace, fill=False, stroke=False)
        return marker_path is not None

    def _image_arguments(self, gc, x, y, im, transform):
        """The arguments of ``draw_image`` as matplotlib reads them."""
        h, w = im.shape[:2]
        im[::-1, :, :3].tobytes()
        if transform is not None:
            transform.frozen().to_values()
        gc.get_clip_rectangle()
        check_coordinates(x, y)
        f"{w:d} {h:d}"  # noqa: B018

    def _text_arguments(self, gc, x, y, s, prop, angle, ismath, mtext):
        """The arguments of ``draw_text`` (not typeset by TeX) as matplotlib
        reads them; whether it draws the text (no fully transparent text)."""
        if self._is_transparent(gc.get_rgb()):
            return False
        if ismath:
            self._mathtext_arguments(gc, x, y, s, prop, angle)
        else:
            features = language = None
            if mtext is not None:
                features = mtext.get_fontfeatures()
                language = mtext.get_language()
            _check_layout(s, prop, features, language)
            prop.get_size_in_points()
            self._check_color(*gc.get_rgb())
            check_coordinates(x, y, angle)
        return True

    def _mathtext_arguments(self, gc, x, y, s, prop, angle):
        """The arguments of ``draw_mathtext`` as matplotlib reads them."""
        self._text2path.mathtext_parser.parse(s, 72, prop)
        self._check_color(*gc.get_rgb())
        check_coordinates(x, y, angle)

    def _tex_arguments(self, gc, x, y, s, prop, angle):
        """The arguments of ``draw_tex`` as matplotlib reads them."""
        self._is_transparent(gc.get_rgb())
        prop.get_size_in_points()
        check_coordinates(x, y, angle)

    def _gouraud_arguments(self, gc, points, colors, trans):
        """The arguments of ``draw_gouraud_triangles`` as matplotlib checks
        and reads them; whether there are triangles to draw."""
        assert len(points) == len(colors)
        if len(points) == 0:
            return False
        assert points.ndim == 3
        assert points.shape[1] == 3
        assert points.shape[2] == 2
        assert colors.ndim == 3
        assert colors.shape[1] == 3
        assert colors.shape[2] == 4

        shape = points.shape
        flat_points = trans.transform(points.reshape((shape[0] * shape[1], 2)))
        flat_colors = colors.reshape((shape[0] * shape[1], 4))
        if not (np.isfinite(np.asarray(flat_points)).all()
                and np.isfinite(np.asarray(flat_colors[:, :3])).all()):
            # The stream of vertices as matplotlib packs it, for numpy's
            # warnings about what is no number.
            points_min = np.min(flat_points, axis=0) - (1 << 12)
            points_max = np.max(flat_points, axis=0) + (1 << 12)
            factor = np.ceil((2 ** 32 - 1) / (points_max - points_min))
            data = np.empty(
                shape[0] * shape[1],
                dtype=[('flags', 'u1'), ('points', '2>u4'), ('colors', '3u1')])
            data['points'] = (flat_points - points_min) * factor
            data['colors'] = flat_colors[:, :3] * 255.0
        return True
