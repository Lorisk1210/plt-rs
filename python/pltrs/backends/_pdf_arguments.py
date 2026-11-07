"""
What matplotlib's `.RendererPdf` does with the arguments of its drawing
methods before it writes anything, for `.RendererPdf` to do before pltrs
(Rust) draws: the same attributes read and the same values computed, in
the same order, so that wrong arguments fail as there, with matplotlib's
errors. Ported from matplotlib 3.11.2's ``backends/backend_pdf.py`` under
the Matplotlib License.
"""

import math
from numbers import Real

import numpy as np

from .. import rcParams
from ._native_drawing import affine_matrix


def check_finite(*values):
    """Refuse the numbers among *values* that are not finite, as
    matplotlib's ``pdfRepr`` refuses to write them."""
    for value in values:
        if isinstance(value, Real) and not math.isfinite(value):
            raise ValueError("Can only output finite numbers in PDF")


class PdfArgumentsMixin:
    """The arguments of `.RendererPdf`'s drawing methods, used as
    matplotlib's renderer uses them."""

    def _prepare_gc(self, gc, fillcolor=None):
        """What matplotlib's ``check_gc`` does with *gc* and *fillcolor*
        before it writes the change of graphics state (which pltrs (Rust)
        writes)."""
        orig_fill = getattr(gc, '_fillcolor', (0., 0., 0.))
        gc._fillcolor = fillcolor

        orig_alphas = getattr(gc, '_effective_alphas', (1.0, 1.0))

        if gc.get_rgb() is None:
            # It should not matter what color here since linewidth should be
            # 0 unless affected by global settings in rcParams, hence setting
            # zero alpha just in case.
            gc.set_foreground((0, 0, 0, 0), isRGBA=True)

        if gc._forced_alpha:
            gc._effective_alphas = (gc._alpha, gc._alpha)
        elif fillcolor is None or len(fillcolor) < 4:
            gc._effective_alphas = (gc._rgb[3], 1.0)
        else:
            gc._effective_alphas = (gc._rgb[3], fillcolor[3])

        if fillcolor is not None and not rcParams['pdf.inheritcolor']:
            # The fill colour as ``fillcolor_cmd`` writes it.
            if fillcolor[0] == fillcolor[1] == fillcolor[2]:
                check_finite(fillcolor[0])
            else:
                check_finite(*fillcolor[:3])

        # Restore gc to avoid unwanted side effects
        gc._fillcolor = orig_fill
        gc._effective_alphas = orig_alphas

    def _path_arguments(self, gc, path, transform, rgbFace):
        """The arguments of ``draw_path``, as matplotlib reads them before it
        writes the path (``writePath``)."""
        self._prepare_gc(gc, rgbFace)
        if rgbFace is None and gc.get_hatch_path() is None:
            path.should_simplify  # noqa: B018  (the path clipped is simplified)
        path.vertices  # noqa: B018
        affine_matrix(transform)

    def _markers_reused(self, gc, marker_path, path, rgbFace):
        """Whether matplotlib places the markers as one template (many
        markers), reading the graphics context as it does then; else it
        draws each marker as a path (`.RendererBase.draw_markers`)."""
        len_marker_path = len(marker_path)
        uses = len(path)
        if len_marker_path * uses < len_marker_path + uses + 5:
            return False
        self._prepare_gc(gc, rgbFace)
        gc.fill(rgbFace)
        gc.stroke()
        return True

    def _image_arguments(self, gc, x, y, im, transform):
        """The arguments of ``draw_image`` as matplotlib reads them; whether
        there is an image to draw."""
        h, w = im.shape[:2]
        if w == 0 or h == 0:
            return False
        if transform is None:
            # matplotlib sets the alpha of *gc* to 1 (pltrs (Rust) draws the
            # image with the alpha already applied).
            gc.set_alpha  # noqa: B018
        self._prepare_gc(gc)
        check_finite(x, y)
        return True

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
        assert colors.shape[2] in (1, 4)

        shape = points.shape
        trans.transform(points.reshape((shape[0] * shape[1], 2)))
        if colors.shape[2] == 1 or np.allclose(colors[0, 0, 3], colors[:, :, 3]):
            # matplotlib sets the alpha of *gc* (to 1 for gray levels, else
            # to the triangles' one alpha), which pltrs (Rust) applies.
            gc.set_alpha  # noqa: B018
            self._prepare_gc(gc)
        return True

    def _text_arguments(self, gc, x, y, s, prop, angle, ismath, mtext):
        """The arguments of ``draw_text`` as matplotlib reads them."""
        self._prepare_gc(gc, gc._rgb)
        if ismath:
            self._mathtext_arguments(gc, x, y, s, prop, angle)
            return
        prop.get_size_in_points()
        if mtext is not None:
            mtext.get_fontfeatures()
            mtext.get_language()
        math.radians(angle)
        check_finite(x, y, angle)

    def _mathtext_arguments(self, gc, x, y, s, prop, angle):
        """The arguments of ``draw_mathtext`` as matplotlib reads them."""
        self._text2path.mathtext_parser.parse(s, 72, prop)
        gc.get_url()
        math.radians(angle)
        check_finite(x, y, angle)
        self._prepare_gc(gc, gc._rgb)

    def _tex_arguments(self, gc, x, y, s, prop, angle):
        """The arguments of ``draw_tex`` as matplotlib reads them (before
        and after it has TeX typeset *s*)."""
        prop.get_size_in_points()
        gc.get_url()
        # The transform of the text, ``Affine2D().rotate_deg(angle)
        # .translate(x, y)``, computed as numpy computes it there.
        math.radians(angle)
        matrix = np.eye(3)
        matrix[0, 2] += x
        matrix[1, 2] += y
        self._prepare_gc(gc, gc._rgb)
        check_finite(x, y, angle)
