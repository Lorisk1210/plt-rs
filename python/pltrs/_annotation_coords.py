"""
matplotlib's coordinate systems of annotations (``text._AnnotationBase``),
computed from the transform trees, and the transform an `.Annotation` is
given when it is positioned.

matplotlib's ``Annotation.update_positions``, which its ``draw`` and
``get_window_extent`` run, sets the annotation's transform to that of its
text coordinates (``_get_xy_transform(renderer, anncoords)``): before a
first draw or measure, ``get_transform()`` is the transform the
annotation was made with (`.IdentityTransform` from ``annotate``), and
after it, ``ax.transData`` for text in data coordinates or an `.Affine2D`
placing points, pixels or fractions. pltrs (Rust) places the annotation
itself, so `PositionedMixin` makes that transform when it is read, once
for each draw (pltrs (Rust) counts the draws) or measure since.
"""

from ._native import artist as _artist
from .artist import Artist
from .transforms import (Affine2D, BboxBase, BboxTransformTo, Transform,
                         blended_transform_factory)


class CoordsMixin:
    """matplotlib's coordinate methods of ``_AnnotationBase``."""

    def _get_xy(self, renderer, xy, coords):
        x, y = xy
        xcoord, ycoord = coords if isinstance(coords, tuple) else (coords, coords)
        if xcoord == 'data':
            x = float(self.convert_xunits(x))
        if ycoord == 'data':
            y = float(self.convert_yunits(y))
        return self._get_xy_transform(renderer, coords).transform((x, y))

    def _get_xy_transform(self, renderer, coords):

        if isinstance(coords, tuple):
            xcoord, ycoord = coords
            tr1 = self._get_xy_transform(renderer, xcoord)
            tr2 = self._get_xy_transform(renderer, ycoord)
            return blended_transform_factory(tr1, tr2)
        elif callable(coords):
            tr = coords(renderer)
            if isinstance(tr, BboxBase):
                return BboxTransformTo(tr)
            elif isinstance(tr, Transform):
                return tr
            else:
                raise TypeError(
                    f"xycoords callable must return a BboxBase or Transform, not a "
                    f"{type(tr).__name__}")
        elif isinstance(coords, Artist):
            bbox = coords.get_window_extent(renderer)
            return BboxTransformTo(bbox)
        elif isinstance(coords, BboxBase):
            return BboxTransformTo(coords)
        elif isinstance(coords, Transform):
            return coords
        elif not isinstance(coords, str):
            raise TypeError(
                f"'xycoords' must be an instance of str, tuple[str, str], Artist, "
                f"Transform, or Callable, not a {type(coords).__name__}")

        if coords == 'data':
            return self.axes.transData
        elif coords == 'polar':
            from .projections import PolarAxes
            return PolarAxes.PolarTransform() + self.axes.transData

        try:
            bbox_name, unit = coords.split()
        except ValueError:  # i.e. len(coords.split()) != 2.
            raise ValueError(f"{coords!r} is not a valid coordinate") from None

        bbox0, xy0 = None, None

        # if unit is offset-like
        if bbox_name == "figure":
            bbox0 = self.get_figure(root=False).figbbox
        elif bbox_name == "subfigure":
            bbox0 = self.get_figure(root=False).bbox
        elif bbox_name == "axes":
            bbox0 = self.axes.bbox

        # reference x, y in display coordinate
        if bbox0 is not None:
            xy0 = bbox0.p0
        elif bbox_name == "offset":
            xy0 = self._get_position_xy(renderer)
        else:
            raise ValueError(f"{coords!r} is not a valid coordinate")

        if unit == "points":
            tr = Affine2D().scale(
                self.get_figure(root=True).dpi / 72)  # dpi/72 dots per point
        elif unit == "pixels":
            tr = Affine2D()
        elif unit == "fontsize":
            tr = Affine2D().scale(
                self.get_size() * self.get_figure(root=True).dpi / 72)
        elif unit == "fraction":
            tr = Affine2D().scale(*bbox0.size)
        else:
            raise ValueError(f"{unit!r} is not a recognized unit")

        return tr.translate(*xy0)

    def _get_position_xy(self, renderer):
        """Return the pixel position of the annotated point."""
        return self._get_xy(renderer, self.xy, self.xycoords)

    def _check_xy(self, renderer=None):
        """Check whether the annotation at *xy_pixel* should be drawn."""
        if renderer is None:
            renderer = self.get_figure(root=True)._get_renderer()
        b = self.get_annotation_clip()
        if b or (b is None and self.xycoords == "data"):
            # check if self.xy is inside the Axes.
            xy_pixel = self._get_position_xy(renderer)
            return self.axes.contains_point(xy_pixel)
        return True


class PositionedMixin:
    """
    The transform of an `.Annotation` as matplotlib's ``update_positions``
    sets it: that of its text coordinates once it was drawn or measured.
    """

    def _positioned(self):
        """
        The transform of the text coordinates the annotation was last
        positioned with, made once for each draw or measure; None before
        the first (and outside any figure).
        """
        if self._figure is None:
            return None
        key = (_artist.draw_count(self._native),
               self.__dict__.get("_measures", 0))
        if key == (0, 0):
            return None
        made = self.__dict__.get("_positioned_transform")
        if made is None or made[0] != key:
            # Its figure's renderer, also once it was removed from it.
            renderer = self._home_figure(root=True)._get_renderer()
            made = self._positioned_transform = (
                key, self._get_xy_transform(renderer, self.anncoords))
        return made[1]

    def _keep_drawn_state(self):
        # The transform of its text coordinates, made with its Axes.
        self._positioned()

    def _measured(self):
        """Record that the annotation was positioned to be measured (its
        ``get_window_extent``)."""
        self._measures = self.__dict__.get("_measures", 0) + 1

    def get_transform(self):
        """Return the `.Transform` instance used by this artist: that of
        the text coordinates once the annotation was drawn or measured."""
        positioned = self._positioned()
        return super().get_transform() if positioned is None else positioned

    def is_transform_set(self):
        """
        Return whether the Artist has an explicitly set transform: an
        annotation's is set once it was drawn or measured.
        """
        return self._positioned() is not None or super().is_transform_set()
