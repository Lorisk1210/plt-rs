"""
What matplotlib's `.RendererSVG` does with the arguments of its drawing
methods before it writes them, for `.RendererSVG` to do before pltrs
(Rust) writes the SVG: the same attributes read and the same values
computed and formatted, in the same order, so that wrong arguments fail as
there, with matplotlib's errors. Ported from matplotlib 3.11.2's
``backends/backend_svg.py`` under the Matplotlib License.
"""

import numpy as np

from .. import rcParams
from ..colors import rgb2hex
from ..transforms import Affine2D

#: The SVG line caps of matplotlib's cap styles.
_STROKE_CAPS = {'butt': 'butt', 'projecting': 'square', 'round': 'round'}


def short_float(x):
    """Format *x* as matplotlib writes SVG numbers (``_short_float_fmt``),
    failing as it does on what is no number."""
    f'{x:f}'  # noqa: B018


# The arguments of `.RendererSVG`'s drawing methods, used as matplotlib's
# renderer uses them. No docstring: `RendererSVG` shows `.RendererBase`'s, as
# matplotlib's does.
class SvgArgumentsMixin:

    def _flipped(self, transform):
        """*transform* flipped to SVG's downward y (``_make_flip_transform``)."""
        return transform + Affine2D().scale(1, -1).translate(0, self.height)

    def _clip_arguments(self, gc):
        """What matplotlib reads of the clip of *gc* (``_get_clip_attrs``)."""
        cliprect = gc.get_clip_rectangle()
        clippath, clippath_trans = gc.get_clip_path()
        if clippath is not None:
            self._flipped(clippath_trans)
        elif cliprect is not None:
            x, y, w, h = cliprect.bounds
            self.height - (y + h)  # noqa: B018

    def _style_arguments(self, gc, rgbFace):
        """What matplotlib reads and formats of *gc* and *rgbFace* for the
        style of a path (``_get_style_dict``)."""
        forced_alpha = gc.get_forced_alpha()
        if gc.get_hatch() is not None:
            if (rgbFace is not None and len(rgbFace) == 4 and rgbFace[3] != 1.0
                    and not forced_alpha):
                short_float(rgbFace[3])
        elif rgbFace is not None:
            if tuple(rgbFace[:3]) != (0, 0, 0):
                rgb2hex(rgbFace)
            if len(rgbFace) == 4 and rgbFace[3] != 1.0 and not forced_alpha:
                short_float(rgbFace[3])
        if forced_alpha and gc.get_alpha() != 1.0:
            short_float(gc.get_alpha())
        offset, seq = gc.get_dashes()
        if seq is not None:
            for val in seq:
                short_float(val)
            short_float(float(offset))
        linewidth = gc.get_linewidth()
        if linewidth:
            rgb = gc.get_rgb()
            rgb2hex(rgb)
            if not forced_alpha and rgb[3] != 1.0:
                short_float(rgb[3])
            if linewidth != 1.0:
                short_float(linewidth)
            gc.get_joinstyle()
            if gc.get_capstyle() != 'butt':
                _STROKE_CAPS[gc.get_capstyle()]  # noqa: B018

    def _path_arguments(self, gc, path, transform, rgbFace):
        """The arguments of ``draw_path`` as matplotlib reads them."""
        self._flipped(transform)
        if rgbFace is None and gc.get_hatch_path() is None:
            path.should_simplify  # noqa: B018  (the path clipped is simplified)
        path.vertices  # noqa: B018
        gc.get_sketch_params()
        gc.get_url()
        self._clip_arguments(gc)
        self._style_arguments(gc, rgbFace)

    def _markers_arguments(self, gc, marker_path, marker_trans, path, trans,
                           rgbFace):
        """The arguments of ``draw_markers`` as matplotlib reads them;
        whether there are markers to draw."""
        if not len(path.vertices):
            return False
        if marker_path is not None:  # (None is an empty path to the extension)
            marker_path.vertices  # noqa: B018
        marker_trans + Affine2D().scale(1.0, -1.0)  # noqa: B018
        self._style_arguments(gc, rgbFace)
        self._clip_arguments(gc)
        gc.get_url()
        self._flipped(trans)
        return True

    def _image_arguments(self, gc, x, y, im, transform):
        """The arguments of ``draw_image`` as matplotlib reads them;
        whether there is an image to draw."""
        h, w = im.shape[:2]
        if w == 0 or h == 0:
            return False
        self._clip_arguments(gc)
        gc.get_url()
        gc.get_gid()
        if rcParams['svg.image_inline'] and np.asarray(im).dtype != np.uint8:
            # matplotlib's writer has Pillow encode the image, which takes
            # bytes or one channel.
            from PIL import Image
            Image.fromarray(np.asarray(im))
        if transform is None:
            w = 72.0 * w / self.image_dpi
            h = 72.0 * h / self.image_dpi
            short_float(x)
            short_float(-(self.height - y - h))
        else:
            alpha = gc.get_alpha()
            if alpha != 1.0:
                short_float(alpha)
            (Affine2D().scale(1 / w, 1 / h) + transform
             + Affine2D().translate(x, y).scale(1, -1).translate(0, self.height))
        return True

    def _gouraud_arguments(self, gc, triangles_array, colors_array, transform):
        """The arguments of ``draw_gouraud_triangles`` as matplotlib reads
        them."""
        self._clip_arguments(gc)
        flipped = self._flipped(transform.frozen())
        for points, colors in zip(triangles_array, colors_array):
            self._gouraud_triangle_arguments(flipped.transform(points), colors)

    @staticmethod
    def _gouraud_triangle_arguments(points, colors):
        """Compute with a triangle's corners *points* and *colors* as
        matplotlib's ``_draw_gouraud_triangle`` does (one gradient towards
        each corner over the triangle in their average colour), for its
        errors and warnings."""
        avg_color = np.average(colors, axis=0)
        if avg_color[-1] == 0:
            return  # matplotlib skips a transparent triangle.
        for i in range(3):
            x1, y1 = points[i]
            x2, y2 = points[(i + 1) % 3]
            x3, y3 = points[(i + 2) % 3]
            if x2 == x3:
                xb, yb = x2, y1
            elif y2 == y3:
                xb, yb = x1, y2
            else:
                m1 = (y2 - y3) / (x2 - x3)
                m2 = -(1.0 / m1)
                b2 = y1 - (m2 * x1)
                xb = (-(y2 - (m1 * x2)) + b2) / (m1 - m2)
                yb = m2 * xb + b2
            for value in (x1, y1, xb, yb):
                short_float(value)
            rgb2hex(avg_color)
            short_float(colors[i][-1])
            rgb2hex(colors[i])
        for value in (x1, y1, x2, y2, x3, y3):
            short_float(value)
        rgb2hex(avg_color)

    def _text_arguments(self, gc, x, y, s, prop, angle, mtext=None):
        """The arguments of ``draw_text`` as matplotlib reads them before
        it lays the text out (as paths, or as text)."""
        self._clip_arguments(gc)
        gc.get_url()
        as_paths = rcParams['svg.fonttype'] == 'path'
        if as_paths:
            s.replace("&", "&amp;")  # The text, written as a comment first.
        rgb2hex(gc.get_rgb())
        if as_paths:
            prop.get_size_in_points()
            if mtext is not None:
                mtext.get_fontfeatures()
                mtext.get_language()
        alpha = gc.get_alpha() if gc.get_forced_alpha() else gc.get_rgb()[3]
        if alpha != 1:
            short_float(alpha)
        # The transform of the text, ``translate(x, y) rotate(-angle)``.
        rotation = -angle
        for value in (x, y, rotation):
            short_float(value)
