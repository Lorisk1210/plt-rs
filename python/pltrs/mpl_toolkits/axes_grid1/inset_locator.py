"""
A collection of functions and objects for creating or placing inset axes.

matplotlib's ``mpl_toolkits.axes_grid1.inset_locator`` (from matplotlib
3.11.2, under the Matplotlib License): the locators make pltrs (Rust) place
the insets anew at every layout pass, and the patches outline and join
boxes pltrs (Rust) works out at every draw (``_native.axes_grid1``).
"""

import numbers

from pltrs import _api, _docstring, _live_transforms, _unusable
from pltrs.artist import Artist
from pltrs.axes import _program_locators
from pltrs.offsetbox import AnchoredOffsetbox, OffsetBox
from pltrs._offsetbox_trees import Unusable, number
from pltrs.patches import Patch, Rectangle
from pltrs.path import Path
from pltrs.transforms import Bbox
from pltrs.transforms import BboxBase, IdentityTransform, Transform, TransformedBbox
from pltrs._native import artist as _artist
from pltrs._native import axes_grid1 as _axes_grid1

from . import axes_size as Size
from .parasite_axes import HostAxes


def _native_size(value):
    """The native size of *value*: inches, or ``"30%"`` of the anchor box."""
    if isinstance(value, str):
        return float(value.rstrip('%')), True
    return float(value), False


def _native_placement(loc, bbox_to_anchor, bbox_transform, borderpad):
    """The native placement of an inset: its location code, the anchor box
    (a point or a rectangle, None for the parent axes) and its transform,
    and the padding in font sizes; `.Unusable` for a value matplotlib keeps
    unchecked and fails on when it places the inset (see
    `AnchoredLocatorBase._place`)."""
    if not (isinstance(loc, numbers.Integral) and 1 <= loc <= 10):
        raise Unusable
    if bbox_transform is not None and not isinstance(bbox_transform, Transform):
        raise Unusable
    if isinstance(bbox_to_anchor, BboxBase):
        spanning = getattr(bbox_to_anchor, "_spanning", None)
        if spanning is not None:
            bbox_to_anchor, bbox_transform = (0, 0, 1, 1), spanning
        else:
            bbox_to_anchor, bbox_transform = bbox_to_anchor.bounds, None
    elif bbox_to_anchor is not None:
        try:
            bbox_to_anchor = [number(v) for v in bbox_to_anchor]
        except TypeError:
            raise Unusable from None
    try:
        borderpad = tuple(map(number, borderpad))
    except TypeError:
        borderpad = (number(borderpad),) * 2
    if len(borderpad) != 2:
        raise Unusable
    return (int(loc), bbox_to_anchor,
            None if bbox_transform is None
            else _live_transforms.to_native(bbox_transform),
            borderpad)


class AnchoredLocatorBase(AnchoredOffsetbox):
    def __init__(self, bbox_to_anchor, offsetbox, loc,
                 borderpad=0.5, bbox_transform=None):
        super().__init__(
            loc, pad=0., child=None, borderpad=borderpad,
            bbox_to_anchor=bbox_to_anchor, bbox_transform=bbox_transform
        )
        self._anchor_args = (loc, bbox_to_anchor, bbox_transform, borderpad)
        # The axes pltrs (Rust) places with the locator, and how often it
        # had been drawn then.
        self._located = None
        # What pltrs (Rust) was told to place the axes with.
        self._placed = None

    @property
    def axes(self):
        """The axes the locator was called with: matplotlib's draw of the
        axes it locates calls it, so that it is that axes once drawn."""
        given = Artist.axes.fget(self)
        if given is None and self._located is not None:
            located, draws = self._located
            if _artist.draw_count(located._native) > draws:
                return located
        return given

    @axes.setter
    def axes(self, ax):
        OffsetBox.axes.fset(self, ax)

    def _placement(self):
        """What the locator places the axes with, which matplotlib's draw
        of the axes reads again whenever it calls the locator."""
        return (self.loc, self.borderpad, self.pad, self.prop, self._child,
                self._bbox_to_anchor, self._bbox_to_anchor_transform)

    def _locate_by_program(self, renderer):
        """Before every layout of a draw (see `.axes._program_locators`):
        once the program changed what the locator places the axes with
        since pltrs (Rust) was told, have the axes placed by calling the
        locator from then on, as matplotlib's draw of the axes calls it
        (failing where it fails on what the program gave)."""
        if self._located is None or all(
                now is placed for now, placed in zip(self._placement(), self._placed)):
            return
        located = self._located[0]
        _program_locators.forget(self, located._figure)
        _program_locators.follow(located, located._figure)
        located._locate_by_program(renderer)

    def draw(self, renderer):
        raise RuntimeError("No draw method should be called")

    def __call__(self, ax, renderer):
        fig = ax.get_figure(root=False)
        if renderer is None:
            renderer = fig._get_renderer()
        self.axes = ax
        located = None if self._located is None else self._located[0]
        if located is not None and ax is not located:
            # Every draw of the axes the locator places calls it with that
            # axes in matplotlib, which it can no longer take: pltrs places
            # the axes by calling it from now on.
            _program_locators.follow(located, located._figure)
        bbox = self.get_window_extent(renderer)
        px, py = self.get_offset(bbox.width, bbox.height, 0, 0, renderer)
        bbox_canvas = Bbox.from_bounds(px, py, bbox.width, bbox.height)
        tr = fig.transSubfigure.inverted()
        return TransformedBbox(bbox_canvas, tr)

    def get_window_extent(self, renderer=None):
        # The locator is no box pltrs (Rust) lays out: matplotlib's extent
        # of an offset box, its get_bbox at its get_offset.
        return OffsetBox.get_window_extent(self, renderer)

    def _locate(self, ax):
        """Remember that pltrs (Rust) places *ax* with the locator as it is
        now, and check before every layout whether it still is so."""
        self._located = (ax, _artist.draw_count(ax._native))
        self._placed = self._placement()
        _program_locators.follow(self, ax._figure)

    def _native_placement(self):
        """The native placement of the inset: where it is anchored
        (`.Unusable` for what pltrs (Rust) cannot place it with)."""
        return _native_placement(self.loc, *self._anchor_args[1:])

    def _place(self, ax):
        """Make pltrs (Rust) place *ax* with this locator at every layout;
        while it cannot take what the locator places the axes with (which
        matplotlib keeps unchecked and fails on as it draws the axes),
        place the axes by calling the locator, as matplotlib does."""
        try:
            self._place_natively(ax)
        except Unusable:
            _program_locators.follow(ax, ax._figure)
            return
        self._locate(ax)


class AnchoredSizeLocator(AnchoredLocatorBase):
    def __init__(self, bbox_to_anchor, x_size, y_size, loc,
                 borderpad=0.5, bbox_transform=None):
        super().__init__(
            bbox_to_anchor, None, loc,
            borderpad=borderpad, bbox_transform=bbox_transform
        )

        self.x_size = Size.from_any(x_size)
        self.y_size = Size.from_any(y_size)
        self._sizes = (_native_size(x_size), _native_size(y_size))
        self._parent_axes = None

    def _placement(self):
        # docstring inherited
        return (*super()._placement(), self.x_size, self.y_size)

    def get_bbox(self, renderer):
        bbox = self.get_bbox_to_anchor()
        dpi = renderer.points_to_pixels(72.)

        r, a = self.x_size.get_size(renderer)
        width = bbox.width * r + a * dpi
        r, a = self.y_size.get_size(renderer)
        height = bbox.height * r + a * dpi

        fontsize = renderer.points_to_pixels(self.prop.get_size_in_points())
        pad = self.pad * fontsize

        return Bbox.from_bounds(0, 0, width, height).padded(pad)

    def _place_natively(self, ax):
        """Have pltrs (Rust) place *ax* with this locator at every layout."""
        parent = ax if self._parent_axes is None else self._parent_axes
        _axes_grid1.place_sized_inset(parent._native, ax._native, self._sizes,
                                      self._native_placement())


class AnchoredZoomLocator(AnchoredLocatorBase):
    def __init__(self, parent_axes, zoom, loc,
                 borderpad=0.5,
                 bbox_to_anchor=None,
                 bbox_transform=None):
        self.parent_axes = parent_axes
        self.zoom = zoom
        given = bbox_to_anchor
        if bbox_to_anchor is None:
            bbox_to_anchor = parent_axes.bbox
        super().__init__(
            bbox_to_anchor, None, loc, borderpad=borderpad,
            bbox_transform=bbox_transform)
        # pltrs (Rust) anchors in the parent axes when given no box.
        self._anchor_args = (loc, given, bbox_transform, borderpad)

    def _placement(self):
        # docstring inherited
        return (*super()._placement(), self.parent_axes, self.zoom)

    def get_bbox(self, renderer):
        bb = self.parent_axes.transData.transform_bbox(self.axes.viewLim)
        fontsize = renderer.points_to_pixels(self.prop.get_size_in_points())
        pad = self.pad * fontsize
        return (
            Bbox.from_bounds(
                0, 0, abs(bb.width * self.zoom), abs(bb.height * self.zoom))
            .padded(pad))

    def _place_natively(self, ax):
        """Have pltrs (Rust) place *ax* with this locator at every layout."""
        _axes_grid1.place_zoomed_inset(self.parent_axes._native, ax._native,
                                       number(self.zoom),
                                       self._native_placement())


def _native_bbox(bbox):
    """The native description of the box *bbox*, which pltrs (Rust) works
    out at every draw: an axes' box, or a box in the coordinates of a
    transform; `.Unusable` for no box."""
    if isinstance(bbox, Rectangle):
        bbox = TransformedBbox(Bbox.unit(), bbox.get_transform())
    if not isinstance(bbox, BboxBase):
        raise Unusable
    spanning = getattr(bbox, "_spanning", None)
    if spanning is not None:
        return ("transformed", (0., 0., 1., 1.),
                _live_transforms.to_native(spanning))
    if isinstance(bbox, TransformedBbox):
        return ("transformed", tuple(map(float, bbox._bbox.extents)),
                _live_transforms.to_native(bbox._transform))
    return ("transformed", tuple(map(float, bbox.extents)), None)


def _native_corner(loc):
    """The corner *loc* as pltrs (Rust) takes it, its code; `.Unusable` for
    none of matplotlib's four."""
    if isinstance(loc, numbers.Real) and loc in (1, 2, 3, 4):
        return int(loc)
    raise Unusable


class _KeptGeometry:
    # What `BboxPatch` and `BboxConnector` share: matplotlib keeps their boxes
    # and corners unchecked and fails on them where it computes their path
    # (``get_path``: when it adds the patch to an Axes, measures or draws it);
    # the patch keeps those pltrs (Rust) cannot take, and fails so (see
    # ``pltrs._unusable``). A comment, not a docstring: the patches have
    # none of their own, and ``help`` shows `Patch`'s, as matplotlib's do.

    def _described_bbox(self, bbox):
        """*bbox* as `_native_bbox` describes it; the unit box for one
        pltrs (Rust) cannot take, which the patch keeps."""
        try:
            return _native_bbox(bbox)
        except Unusable:
            self._kept_geometry = True
            return _native_bbox(Bbox.unit())

    def _described_corner(self, loc):
        """The corner *loc* as pltrs (Rust) takes it; the first for one it
        cannot take, which the patch keeps."""
        try:
            return _native_corner(loc)
        except Unusable:
            self._kept_geometry = True
            return 1

    def _keep_geometry(self):
        """Fail as matplotlib's patch does while it has a box or corner
        pltrs (Rust) could not take."""
        if self.__dict__.pop("_kept_geometry", False):
            _unusable.keep(self, "geometry", True, self._use_geometry)

    def _use_geometry(self, _kept):
        """Compute the path as matplotlib's does, failing as it does."""
        self.get_path()

    def _add_to_axes(self, axes, kind, autolim=True):
        if kind == "patch":
            # matplotlib's `add_patch` reads the path for the data limits.
            _unusable.check(self, measuring=True)
        return super()._add_to_axes(axes, kind, autolim)


class BboxPatch(_KeptGeometry, Patch):
    @_docstring.interpd
    def __init__(self, bbox, **kwargs):
        """
        Patch showing the shape bounded by a Bbox.

        Parameters
        ----------
        bbox : `~matplotlib.transforms.Bbox`
            Bbox to use for the extents of this patch.

        **kwargs
            Patch properties. Valid arguments include:

            %(Patch:kwdoc)s
        """
        if "transform" in kwargs:
            raise ValueError("transform should not be set")

        kwargs["transform"] = IdentityTransform()
        self._native = _axes_grid1.bbox_patch(self._described_bbox(bbox))
        super().__init__(**kwargs)
        self.bbox = bbox
        self._keep_geometry()

    def get_path(self):
        # docstring inherited
        x0, y0, x1, y1 = self.bbox.extents
        return Path._create_closed([(x0, y0), (x1, y0), (x1, y1), (x0, y1)])


class BboxConnector(_KeptGeometry, Patch):
    @staticmethod
    def get_bbox_edge_pos(bbox, loc):
        """
        Return the ``(x, y)`` coordinates of corner *loc* of *bbox*; parameters
        behave as documented for the `.BboxConnector` constructor.
        """
        x0, y0, x1, y1 = bbox.extents
        if loc == 1:
            return x1, y1
        elif loc == 2:
            return x0, y1
        elif loc == 3:
            return x0, y0
        elif loc == 4:
            return x1, y0

    @staticmethod
    def connect_bbox(bbox1, bbox2, loc1, loc2=None):
        """
        Construct a `.Path` connecting corner *loc1* of *bbox1* to corner
        *loc2* of *bbox2*, where parameters behave as documented as for the
        `.BboxConnector` constructor.
        """
        if isinstance(bbox1, Rectangle):
            bbox1 = TransformedBbox(Bbox.unit(), bbox1.get_transform())
        if isinstance(bbox2, Rectangle):
            bbox2 = TransformedBbox(Bbox.unit(), bbox2.get_transform())
        if loc2 is None:
            loc2 = loc1
        x1, y1 = BboxConnector.get_bbox_edge_pos(bbox1, loc1)
        x2, y2 = BboxConnector.get_bbox_edge_pos(bbox2, loc2)
        return Path([[x1, y1], [x2, y2]])

    @_docstring.interpd
    def __init__(self, bbox1, bbox2, loc1, loc2=None, **kwargs):
        """
        Connect two bboxes with a straight line.

        Parameters
        ----------
        bbox1, bbox2 : `~matplotlib.transforms.Bbox`
            Bounding boxes to connect.

        loc1, loc2 : {1, 2, 3, 4}
            Corner of *bbox1* and *bbox2* to draw the line. Valid values are::

                'upper right'  : 1,
                'upper left'   : 2,
                'lower left'   : 3,
                'lower right'  : 4

            *loc2* is optional and defaults to *loc1*.

        **kwargs
            Patch properties for the line drawn. Valid arguments include:

            %(Patch:kwdoc)s
        """
        if "transform" in kwargs:
            raise ValueError("transform should not be set")

        kwargs["transform"] = IdentityTransform()
        kwargs.setdefault(
            "fill", bool({'fc', 'facecolor', 'color'}.intersection(kwargs)))
        if "_native" not in self.__dict__:
            self._native = _axes_grid1.bbox_connector(
                self._described_bbox(bbox1), self._described_bbox(bbox2),
                self._described_corner(loc1),
                None if loc2 is None else self._described_corner(loc2))
        super().__init__(**kwargs)
        self.bbox1 = bbox1
        self.bbox2 = bbox2
        self.loc1 = loc1
        self.loc2 = loc2
        self._keep_geometry()

    def get_path(self):
        # docstring inherited
        return self.connect_bbox(self.bbox1, self.bbox2,
                                 self.loc1, self.loc2)


class BboxConnectorPatch(BboxConnector):
    @_docstring.interpd
    def __init__(self, bbox1, bbox2, loc1a, loc2a, loc1b, loc2b, **kwargs):
        """
        Connect two bboxes with a quadrilateral.

        The quadrilateral is specified by two lines that start and end at
        corners of the bboxes. The four sides of the quadrilateral are defined
        by the two lines given, the line between the two corners specified in
        *bbox1* and the line between the two corners specified in *bbox2*.

        Parameters
        ----------
        bbox1, bbox2 : `~matplotlib.transforms.Bbox`
            Bounding boxes to connect.

        loc1a, loc2a, loc1b, loc2b : {1, 2, 3, 4}
            The first line connects corners *loc1a* of *bbox1* and *loc2a* of
            *bbox2*; the second line connects corners *loc1b* of *bbox1* and
            *loc2b* of *bbox2*.  Valid values are::

                'upper right'  : 1,
                'upper left'   : 2,
                'lower left'   : 3,
                'lower right'  : 4

        **kwargs
            Patch properties for the line drawn:

            %(Patch:kwdoc)s
        """
        if "transform" in kwargs:
            raise ValueError("transform should not be set")
        self._native = _axes_grid1.bbox_connector_patch(
            self._described_bbox(bbox1), self._described_bbox(bbox2),
            tuple(map(self._described_corner, (loc1a, loc2a, loc1b, loc2b))))
        kept = self.__dict__.pop("_kept_geometry", False)
        super().__init__(bbox1, bbox2, loc1a, loc2a, **kwargs)
        self.loc1b = loc1b
        self.loc2b = loc2b
        self._kept_geometry = kept
        self._keep_geometry()

    def get_path(self):
        # docstring inherited
        path1 = self.connect_bbox(self.bbox1, self.bbox2, self.loc1, self.loc2)
        path2 = self.connect_bbox(self.bbox2, self.bbox1,
                                  self.loc2b, self.loc1b)
        path_merged = [*path1.vertices, *path2.vertices, path1.vertices[0]]
        return Path(path_merged)


def _add_inset_axes(parent_axes, axes_class, axes_kwargs, axes_locator):
    """Helper function to add an inset axes and disable navigation in it."""
    if axes_class is None:
        axes_class = HostAxes
    if axes_kwargs is None:
        axes_kwargs = {}
    fig = parent_axes.get_figure(root=False)
    inset_axes = axes_class(
        fig, parent_axes.get_position(),
        **{"navigate": False, **axes_kwargs, "axes_locator": axes_locator})
    return fig.add_axes(inset_axes)


@_docstring.interpd
def inset_axes(parent_axes, width, height, loc='upper right',
               bbox_to_anchor=None, bbox_transform=None,
               axes_class=None, axes_kwargs=None,
               borderpad=0.5):
    """
    Create an inset axes with a given width and height.

    Both sizes used can be specified either in inches or percentage.
    For example,::

        inset_axes(parent_axes, width='40%%', height='30%%', loc='lower left')

    creates in inset axes in the lower left corner of *parent_axes* which spans
    over 30%% in height and 40%% in width of the *parent_axes*. Since the usage
    of `.inset_axes` may become slightly tricky when exceeding such standard
    cases, it is recommended to read :doc:`the examples
    </gallery/axes_grid1/inset_locator_demo>`.

    Notes
    -----
    The meaning of *bbox_to_anchor* and *bbox_to_transform* is interpreted
    differently from that of legend. The value of bbox_to_anchor
    (or the return value of its get_points method; the default is
    *parent_axes.bbox*) is transformed by the bbox_transform (the default
    is Identity transform) and then interpreted as points in the pixel
    coordinate (which is dpi dependent).

    Thus, following three calls are identical and creates an inset axes
    with respect to the *parent_axes*::

       axins = inset_axes(parent_axes, "30%%", "40%%")
       axins = inset_axes(parent_axes, "30%%", "40%%",
                          bbox_to_anchor=parent_axes.bbox)
       axins = inset_axes(parent_axes, "30%%", "40%%",
                          bbox_to_anchor=(0, 0, 1, 1),
                          bbox_transform=parent_axes.transAxes)

    Parameters
    ----------
    parent_axes : `matplotlib.axes.Axes`
        Axes to place the inset axes.

    width, height : float or str
        Size of the inset axes to create. If a float is provided, it is
        the size in inches, e.g. *width=1.3*. If a string is provided, it is
        the size in relative units, e.g. *width='40%%'*. By default, i.e. if
        neither *bbox_to_anchor* nor *bbox_transform* are specified, those
        are relative to the parent_axes. Otherwise, they are to be understood
        relative to the bounding box provided via *bbox_to_anchor*.

    loc : str, default: 'upper right'
        Location to place the inset axes.  Valid locations are
        'upper left', 'upper center', 'upper right',
        'center left', 'center', 'center right',
        'lower left', 'lower center', 'lower right'.
        For backward compatibility, numeric values are accepted as well.
        See the parameter *loc* of `.Legend` for details.

    bbox_to_anchor : tuple or `~matplotlib.transforms.BboxBase`, optional
        Bbox that the inset axes will be anchored to. If None,
        a tuple of (0, 0, 1, 1) is used if *bbox_transform* is set
        to *parent_axes.transAxes* or *parent_axes.figure.transFigure*.
        Otherwise, *parent_axes.bbox* is used. If a tuple, can be either
        [left, bottom, width, height], or [left, bottom].
        If the kwargs *width* and/or *height* are specified in relative units,
        the 2-tuple [left, bottom] cannot be used. Note that,
        unless *bbox_transform* is set, the units of the bounding box
        are interpreted in the pixel coordinate. When using *bbox_to_anchor*
        with tuple, it almost always makes sense to also specify
        a *bbox_transform*. This might often be the axes transform
        *parent_axes.transAxes*.

    bbox_transform : `~matplotlib.transforms.Transform`, optional
        Transformation for the bbox that contains the inset axes.
        If None, a `.transforms.IdentityTransform` is used. The value
        of *bbox_to_anchor* (or the return value of its get_points method)
        is transformed by the *bbox_transform* and then interpreted
        as points in the pixel coordinate (which is dpi dependent).
        You may provide *bbox_to_anchor* in some normalized coordinate,
        and give an appropriate transform (e.g., *parent_axes.transAxes*).

    axes_class : `~matplotlib.axes.Axes` type, default: `.HostAxes`
        The type of the newly created inset axes.

    axes_kwargs : dict, optional
        Keyword arguments to pass to the constructor of the inset axes.
        Valid arguments include:

        %(Axes:kwdoc)s

    borderpad : float or (float, float), default: 0.5
        Padding between inset axes and the bbox_to_anchor.
        If a float, the same padding is used for both x and y.
        If a tuple of two floats, it specifies the (x, y) padding.
        The units are axes font size, i.e. for a default font size of 10 points
        *borderpad = 0.5* is equivalent to a padding of 5 points.

        .. versionadded:: 3.11
           The *borderpad* parameter now accepts a tuple of (x, y) paddings.

    Returns
    -------
    inset_axes : *axes_class*
        Inset axes object created.
    """

    if (bbox_transform in [parent_axes.transAxes,
                           parent_axes.get_figure(root=False).transFigure]
            and bbox_to_anchor is None):
        _api.warn_external("Using the axes or figure transform requires a "
                           "bounding box in the respective coordinates. "
                           "Using bbox_to_anchor=(0, 0, 1, 1) now.")
        bbox_to_anchor = (0, 0, 1, 1)
    given = bbox_to_anchor
    if bbox_to_anchor is None:
        bbox_to_anchor = parent_axes.bbox
    if (isinstance(bbox_to_anchor, tuple) and
            (isinstance(width, str) or isinstance(height, str))):
        if len(bbox_to_anchor) != 4:
            raise ValueError("Using relative units for width or height "
                             "requires to provide a 4-tuple or a "
                             "`Bbox` instance to `bbox_to_anchor.")
    locator = AnchoredSizeLocator(
        bbox_to_anchor, width, height, loc=loc,
        bbox_transform=bbox_transform, borderpad=borderpad)
    locator._parent_axes = parent_axes
    # pltrs (Rust) anchors in the parent axes when given no box.
    locator._anchor_args = (loc, given, bbox_transform, borderpad)
    return _add_inset_axes(parent_axes, axes_class, axes_kwargs, locator)


@_docstring.interpd
def zoomed_inset_axes(parent_axes, zoom, loc='upper right',
                      bbox_to_anchor=None, bbox_transform=None,
                      axes_class=None, axes_kwargs=None,
                      borderpad=0.5):
    """
    Create an anchored inset axes by scaling a parent axes. For usage, also see
    :doc:`the examples </gallery/axes_grid1/inset_locator_demo2>`.

    Parameters
    ----------
    parent_axes : `~matplotlib.axes.Axes`
        Axes to place the inset axes.

    zoom : float
        Scaling factor of the data axes. *zoom* > 1 will enlarge the
        coordinates (i.e., "zoomed in"), while *zoom* < 1 will shrink the
        coordinates (i.e., "zoomed out").

    loc : str, default: 'upper right'
        Location to place the inset axes.  Valid locations are
        'upper left', 'upper center', 'upper right',
        'center left', 'center', 'center right',
        'lower left', 'lower center', 'lower right'.
        For backward compatibility, numeric values are accepted as well.
        See the parameter *loc* of `.Legend` for details.

    bbox_to_anchor : tuple or `~matplotlib.transforms.BboxBase`, optional
        Bbox that the inset axes will be anchored to. If None,
        *parent_axes.bbox* is used. If a tuple, can be either
        [left, bottom, width, height], or [left, bottom].
        If the kwargs *width* and/or *height* are specified in relative units,
        the 2-tuple [left, bottom] cannot be used. Note that
        the units of the bounding box are determined through the transform
        in use. When using *bbox_to_anchor* it almost always makes sense to
        also specify a *bbox_transform*. This might often be the axes transform
        *parent_axes.transAxes*.

    bbox_transform : `~matplotlib.transforms.Transform`, optional
        Transformation for the bbox that contains the inset axes.
        If None, a `.transforms.IdentityTransform` is used (i.e. pixel
        coordinates). This is useful when not providing any argument to
        *bbox_to_anchor*. When using *bbox_to_anchor* it almost always makes
        sense to also specify a *bbox_transform*. This might often be the
        axes transform *parent_axes.transAxes*. Inversely, when specifying
        the axes- or figure-transform here, be aware that not specifying
        *bbox_to_anchor* will use *parent_axes.bbox*, the units of which are
        in display (pixel) coordinates.

    axes_class : `~matplotlib.axes.Axes` type, default: `.HostAxes`
        The type of the newly created inset axes.

    axes_kwargs : dict, optional
        Keyword arguments to pass to the constructor of the inset axes.
        Valid arguments include:

        %(Axes:kwdoc)s

    borderpad : float, default: 0.5
        Padding between inset axes and the bbox_to_anchor.
        The units are axes font size, i.e. for a default font size of 10 points
        *borderpad = 0.5* is equivalent to a padding of 5 points.

    Returns
    -------
    inset_axes : *axes_class*
        Inset axes object created.
    """

    return _add_inset_axes(
        parent_axes, axes_class, axes_kwargs,
        AnchoredZoomLocator(
            parent_axes, zoom=zoom, loc=loc,
            bbox_to_anchor=bbox_to_anchor, bbox_transform=bbox_transform,
            borderpad=borderpad))


@_docstring.interpd
def mark_inset(parent_axes, inset_axes, loc1, loc2, **kwargs):
    """
    Draw a box to mark the location of an area represented by an inset axes.

    This function draws a box in *parent_axes* at the bounding box of
    *inset_axes*, and shows a connection with the inset axes by drawing lines
    at the corners, giving a "zoomed in" effect.

    Parameters
    ----------
    parent_axes : `~matplotlib.axes.Axes`
        Axes which contains the area of the inset axes.

    inset_axes : `~matplotlib.axes.Axes`
        The inset axes.

    loc1, loc2 : {1, 2, 3, 4}
        Corners to use for connecting the inset axes and the area in the
        parent axes.

    **kwargs
        Patch properties for the lines and box drawn:

        %(Patch:kwdoc)s

    Returns
    -------
    pp : `~matplotlib.patches.Patch`
        The patch drawn to represent the area of the inset axes.

    p1, p2 : `~matplotlib.patches.Patch`
        The patches connecting two corners of the inset axes and its area.
    """
    from pltrs import _handles, cbook
    from pltrs._artist_kwargs import check_known
    rect = TransformedBbox(inset_axes.viewLim, parent_axes.transData)
    # matplotlib makes the outline with the properties first.
    check_known(BboxPatch, kwargs)
    try:
        corners = (_native_corner(loc1), _native_corner(loc2))
    except Unusable:
        # matplotlib's adds the outline, then fails to find the corners of
        # the connectors as it adds them.
        kwargs.setdefault(
            "fill", bool({'fc', 'facecolor', 'color'}.intersection(kwargs)))
        parent_axes.add_patch(BboxPatch(rect, **kwargs))
        for loc in (loc1, loc2):
            BboxConnector.connect_bbox(inset_axes.bbox, rect, loc)
        raise
    props = cbook.normalize_kwargs(kwargs, Patch)
    pp, p1, p2 = _axes_grid1.mark_inset(parent_axes._native, inset_axes._native,
                                        corners, props)
    figure = parent_axes.figure
    pp = _handles.wrap(pp, BboxPatch, figure)
    pp.bbox = rect
    p1, p2 = (_handles.wrap(p, BboxConnector, figure) for p in (p1, p2))
    for p, loc in ((p1, loc1), (p2, loc2)):
        p.bbox1, p.bbox2, p.loc1, p.loc2 = inset_axes.bbox, rect, loc, None
    return pp, p1, p2
