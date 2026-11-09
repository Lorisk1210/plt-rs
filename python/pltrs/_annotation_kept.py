"""
The point and the coordinate systems of an `.Annotation` as matplotlib
keeps them: unchecked. matplotlib places the point with them only where it
draws or measures the annotation (``_check_xy``, ``update_positions``),
and fails there. pltrs (Rust) holds only what it can place a point with,
so the annotation keeps what pltrs (Rust) cannot hold (see `._unusable`),
pltrs (Rust) holding ``'data'`` and the origin meanwhile, and fails as
matplotlib's does wherever matplotlib's would place the point.
"""

from . import _api, _unusable, _values, units
from ._native import annotation as _annotation


def native_point(xy, figure):
    """The point *xy* as `_native.annotation` takes it for an annotation
    in *figure*, or None for one pltrs (Rust) cannot hold (category names
    outside any figure, where no Axes' units convert them)."""
    from .text import _point
    try:
        point = _point(xy)
    except (TypeError, ValueError):
        return None
    if figure is None and any(isinstance(value, str) for value in point):
        return None
    return point


def native_coords(coords, figure):
    """The coordinate systems *coords* of an annotation in *figure* (None
    outside any) as `_native.annotation` takes them, or None for systems
    pltrs (Rust) cannot hold."""
    from .text import _native_coords
    try:
        return _native_coords(coords, figure)
    except (TypeError, ValueError):
        return None


def annotate_arguments(xy, xytext, xycoords, textcoords, arrowprops,
                       annotation_clip, figure):
    """
    The arguments `_native.annotation` makes an annotation in *figure*
    with, from those of `.Annotation`: the points and coordinate systems
    pltrs (Rust) cannot hold replaced by the origin and ``'data'``, which
    the annotation keeps (`KeptMixin._keep_given`).
    """
    from .text import _native_arrowprops
    origin = (0.0, 0.0)
    return (native_point(xy, figure) or origin,
            None if xytext is None else native_point(xytext, figure) or origin,
            native_coords(xycoords, figure) or "data",
            None if textcoords is None else native_coords(textcoords, figure) or "data",
            None if arrowprops is None else _native_arrowprops(arrowprops),
            annotation_clip)


def warn_textcoords(xytext, textcoords, xycoords):
    """Warn as matplotlib's `.Annotation` warns about *textcoords* given
    without *xytext*."""
    if xytext is None and textcoords is not None and textcoords != xycoords:
        _api.warn_external("You have used the `textcoords` kwarg, but "
                           "not the `xytext` kwarg.  This can lead to "
                           "surprising results.")


#: The setters of `_native.annotation` of each coordinate system.
_COORDS_SETTERS = {"xycoords": "set_xycoords", "anncoords": "set_textcoords"}


class KeptMixin:
    """
    The point and coordinate systems of an `.Annotation` kept as given, and
    how it fails where matplotlib's draw places them: the annotated point,
    *xy*, with *xycoords* to clip the annotation to its Axes
    (``_check_xy``) and to point the arrow at it; the text position with
    *anncoords* at every draw and measure.
    """

    def _point_numbers(self, xy, coords):
        """The point *xy* in *coords* in numbers, its coordinates in
        ``'data'`` coordinates converted with the units of the annotation's
        Axes, as matplotlib's ``_get_xy`` converts them."""
        x, y = xy
        xcoord, ycoord = coords if isinstance(coords, tuple) else (coords, coords)
        x = self.convert_xunits(x) if xcoord == 'data' else x
        y = self.convert_yunits(y) if ycoord == 'data' else y
        return float(x), float(y)

    def _clips_xy(self):
        """Whether matplotlib's draw places the annotated point to clip
        the annotation to its Axes (``_check_xy``)."""
        clip = self.get_annotation_clip()
        return bool(clip or (clip is None and self.xycoords == "data"))

    def _places_xy(self):
        """Whether matplotlib's draw places the annotated point: to clip
        the annotation to its Axes, or to point the arrow at it
        (``update_positions``)."""
        return self._clips_xy() or self.arrowprops is not None

    def _kept_order(self):
        """The order in which matplotlib's draw computes with the values
        the annotation keeps (see `._unusable.check`): the point (to clip),
        the text coordinates and position, the point (for the arrow)."""
        if self._clips_xy():
            return ("xy", "xycoords", "anncoords", "x", "y")
        return ("anncoords", "x", "y", "xy", "xycoords")

    def _place_xy(self, xy):
        """Fail as matplotlib's annotation fails to place the point *xy*,
        where it places it."""
        if self._places_xy():
            self._get_xy(None, xy, self.xycoords)

    def _transform_with(self, coords):
        """Fail as matplotlib fails to make the transform of *coords*
        (``_get_xy_transform``)."""
        self._get_xy_transform(None, coords)

    def _place_xy_with(self, coords):
        """Fail as matplotlib fails to place the annotated point in
        *coords*, where it places it."""
        if self._places_xy():
            self._get_xy(None, self.xy, coords)

    def _keep_given(self, xy, position):
        """Keep the point *xy* and the text *position* the annotation was
        made with as given, and those of them and of its coordinate systems
        that pltrs (Rust) could not hold as unusable (see
        `annotate_arguments`)."""
        if native_point(xy, self.figure) is None:
            self._keep_unusable_xy(xy)
        else:
            self._remember_xy(xy)
        for index, value in enumerate(position):
            self._given_coordinate(index, value)
        for name, coords in (("xycoords", self._xycoords), ("anncoords", self._textcoords)):
            if native_coords(coords, self.figure) is None:
                self._keep_coords(name, coords)

    def _hold_xy(self, xy):
        """Make *xy* the annotated point, as given: handed to pltrs (Rust)
        converted from the units of the annotation's Axes, as matplotlib
        converts it where it places it (``_get_xy``), else kept as
        unusable."""
        try:
            numbers = self._point_numbers(xy, self.xycoords)
        except (TypeError, ValueError):
            self._keep_unusable_xy(xy)
        else:
            _annotation.set_xy(self._native, *numbers)
            self._remember_xy(xy)
            _unusable.drop(self, "xy")

    def _keep_unusable_xy(self, xy):
        """Keep the annotated point *xy*, which pltrs (Rust) cannot hold."""
        _values.keep(self, "xy", xy)
        _unusable.keep(self, "xy", xy, self._place_xy)

    def _remember_xy(self, xy):
        """Have the annotated point read back as *xy*, given in numbers
        while pltrs (Rust) still holds them (it moves a dragged point), in
        units whatever they convert to."""
        if all(units._is_natively_supported(value) for value in xy):
            _values.remember(self, "xy", xy)
        else:
            _values.keep(self, "xy", xy)

    def _hold_coords(self, name, coords):
        """Make *coords* the coordinate systems *name* (``'xycoords'`` or
        ``'anncoords'``), handed to pltrs (Rust), else kept as unusable."""
        native = native_coords(coords, self.figure)
        if native is None:
            self._keep_coords(name, coords)
        else:
            getattr(_annotation, _COORDS_SETTERS[name])(self._native, native)
            _unusable.drop(self, name)

    def _keep_coords(self, name, coords):
        """Keep *coords*, the coordinate systems *name*, which pltrs (Rust)
        cannot hold: matplotlib transforms the text position with
        ``anncoords`` at every draw and measure, the annotated point with
        ``xycoords`` where it places it."""
        use = self._transform_with if name == "anncoords" else self._place_xy_with
        _unusable.keep(self, name, coords, use)

    def _given_coordinate(self, index, value):
        """Keep *value*, the coordinate *index* of the text position the
        annotation was made with, as given; as unusable when pltrs (Rust)
        could not hold it (see `.Text._set_text_coordinate`)."""
        from .text import _coordinate
        try:
            _coordinate(value)
        except (TypeError, ValueError):
            self._keep_unusable_coordinate(index, value)
        else:
            self._remember_coordinate(index, value)
