"""
Classes for the efficient drawing of large collections of objects that
share most properties, e.g., a large number of line segments or
polygons (matplotlib's ``collections``).

A collection wraps the native handle of a pltrs (Rust) collection.
Colours cross to pltrs (Rust) as matplotlib keeps them, RGBA arrays;
the face and edge colours read back as matplotlib keeps them: as set,
after the defaults and alpha, until a draw maps the values onto them
(`Collection.update_scalarmappable`). Widths read back as given.
"""

import contextlib
import functools
import math
from numbers import Real
import operator
import warnings

import numpy as np

from . import _docstring
from . import (_api, _artist_copying, _collection_draw_checks, _live_transforms,
               _owned_paths, _program_paths, _refused_colors, _unusable, _values, cbook,
               colorizer as mcolorizer, colors as mcolors, lines as mlines, path as mpath,
               transforms as mtransforms)
from ._enums import CapStyle, JoinStyle
from ._program_classes import takes_from_program
from ._native import artist as _artist, collections as _collections
from ._native import tri as _tri
from .artist import Artist, _check_array_alpha, allow_rasterization
from .path import Path
from . import _collection_arguments


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from . import rcParams
    return rcParams


def _rgba(colors):
    """*colors* (one colour or a sequence) as an ``(N, 4)`` RGBA array."""
    return mcolors.to_rgba_array(colors)


def _same_paths(kept, native):
    """Whether the native paths *native*, ``(vertices, codes)`` pairs, are
    those *kept* were read as."""
    return len(kept) == len(native) and all(
        np.array_equal(v0, v1) and np.array_equal(c0, c1)
        for (v0, c0), (v1, c1) in zip(kept, native))


def _edge_spec(colors):
    """Edge colours in native form: ``None``, ``'face'`` or RGBA rows."""
    if colors is None or cbook._str_equal(colors, 'face'):
        return colors
    return _rgba(colors)


def _linestyle_list(ls):
    """*ls* (one line style or a sequence) as the list of styles it gives,
    as given."""
    try:
        mlines._get_dash_pattern(ls)
        return [ls]
    except ValueError:
        styles = list(ls)
        for style in styles:
            mlines._get_dash_pattern(style)
        return styles


def _event_positions(positions, lineoffset, linelength):
    """The sorted positions of the events of an `.EventCollection`, checked
    as matplotlib's ``set_positions`` checks them computing their segments
    with the line *lineoffset* and *linelength* (None for no positions)."""
    if positions is None:
        positions = []
    if np.ndim(positions) != 1:
        raise ValueError('positions must be one-dimensional')
    segments = np.empty((len(positions), 2, 2))
    segments[:, :, 0] = np.sort(positions)[:, None]
    segments[:, 0, 1] = lineoffset + linelength / 2
    segments[:, 1, 1] = lineoffset - linelength / 2
    return segments[:, 0, 0]


def _checked_linestyles(ls):
    """*ls* as the list of styles `.Collection.set_linestyle` takes from
    it, failing as it does."""
    try:
        return _linestyle_list(ls)
    except ValueError as err:
        raise ValueError(f'Do not know how to convert {ls!r} to dashes') from err


def _linestyle_specs(ls):
    """*ls* (one line style or a sequence) as a list of native styles."""
    return [_native_style(style) for style in _linestyle_list(ls)]


def _same_linestyles(given, native):
    """Whether the *native* styles are the styles *given*, cycled over
    them (one style given for every level of a contour set, say)."""
    def pattern(style):
        offset, dashes = mlines._get_dash_pattern(style)
        return offset, None if dashes is None else list(dashes)
    return (len(given) > 0 and len(native) % len(given) == 0
            and all(pattern(given[i % len(given)]) == pattern(style)
                    for i, style in enumerate(native)))


def _native_style(style):
    """A line style as the native side takes it (tuples of floats)."""
    if isinstance(style, tuple):
        offset, seq = style
        return (offset, None if seq is None else tuple(seq))
    return style


def _native_kwargs(kwargs):
    """
    Collection keyword arguments, whose aliases are resolved, in the form
    the native collection properties take.
    """
    native = {}
    for key, value in kwargs.items():
        if key in ("color", "facecolor"):
            native[key] = None if value is None else _rgba(value)
        elif key == "edgecolor":
            native[key] = _edge_spec(value)
        elif key == "hatchcolor":
            native[key] = value if cbook._str_equal(value, 'edge') else _rgba(value)
        elif key == "linewidth":
            native[key] = [float(w) for w in np.atleast_1d(value)]
        elif key == "linestyle":
            native[key] = _linestyle_specs(value)
        elif key == "antialiased":
            native[key] = [bool(a) for a in np.atleast_1d(value)]
        elif key == "alpha" and np.iterable(value):
            native[key] = np.asarray(value, float).ravel().tolist()
        else:
            native[key] = value
    return native


def _scale_of(sizes):
    """Take the square root of the marker areas *sizes*, as matplotlib's
    ``set_sizes`` scales the items by it, so that numpy warns of a negative
    area as it does there (under the program's `numpy.errstate`); None is
    no sizes, which it does not scale."""
    if sizes is not None:
        np.sqrt(np.asarray(sizes))


def _drawn_linewidths(widths):
    """Fail as matplotlib's renderers fail to draw a collection whose line
    *widths* are no numbers: they convert them to a float array."""
    np.asarray(widths, dtype=float)


def _refuse_rank(array):
    """Raise what matplotlib's collections raise for a value *array* of
    more than one dimension (but meshes) when they map it to colours."""
    raise ValueError('Collections can only map rank 1 arrays')


def _no_paths(paths):
    """The paths of a collection given None for them: None, as matplotlib
    keeps them (networkx makes its empty node collections so)."""
    return None


def _paths_container(paths):
    """What `Collection.get_paths` hands the paths given as *paths* back
    in: a tuple or a list, as given, or None for None."""
    if paths is None:
        return _no_paths
    return tuple if isinstance(paths, tuple) else list


@_api.define_aliases({
    "antialiased": ["antialiaseds", "aa"],
    "edgecolor": ["edgecolors", "ec"],
    "facecolor": ["facecolors", "fc"],
    "linestyle": ["linestyles", "dashes", "ls"],
    "linewidth": ["linewidths", "lw"],
    "offset_transform": ["transOffset"],
})
class Collection(mcolorizer.ColorizingArtist):
    r"""
    Base class for Collections. Must be subclassed to be usable.

    A Collection represents a sequence of `.Patch`\es that can be drawn
    more efficiently together than individually. For example, when a single
    path is being drawn repeatedly at different offsets, the renderer can
    typically execute a ``draw_marker()`` call much more efficiently than a
    series of repeated calls to ``draw_path()`` with the offsets put in
    one-by-one.

    Most properties of a collection can be configured per-element. Therefore,
    Collections have "plural" versions of many of the properties of a `.Patch`
    (e.g. `.Collection.get_paths` instead of `.Patch.get_path`). Exceptions are
    the *zorder*, *hatch*, *pickradius*, *capstyle* and *joinstyle* properties,
    which can only be set globally for the whole collection.

    Besides these exceptions, all properties can be specified as single values
    (applying to all elements) or sequences of values. The property of the
    ``i``\th element of the collection is::

      prop[i % len(prop)]

    Each Collection can optionally be used as its own `.ScalarMappable` by
    passing the *norm* and *cmap* parameters to its constructor. If the
    Collection's `.ScalarMappable` matrix ``_A`` has been set (via a call
    to `.Collection.set_array`), then at draw time this internal scalar
    mappable will be used to set the ``facecolors`` and ``edgecolors``,
    ignoring those that were manually passed in.
    """
    # pltrs: the colorizer is Python's, as matplotlib's is; pltrs (Rust) maps
    # the values with a copy of its colormap and norm, handed over whenever it
    # changes (`changed`).

    # Whether to draw an edge by default.  Set on a
    # subclass-by-subclass basis.
    _edge_default = False

    @_docstring.interpd
    def __init__(self, *,
                 edgecolors=None,
                 facecolors=None,
                 hatchcolors=None,
                 linewidths=None,
                 linestyles='solid',
                 capstyle=None,
                 joinstyle=None,
                 antialiaseds=None,
                 offsets=None,
                 offset_transform=None,
                 norm=None,
                 cmap=None,
                 colorizer=None,
                 pickradius=5.0,
                 hatch=None,
                 urls=None,
                 zorder=1,
                 **kwargs
                 ):
        """
        Parameters
        ----------
        edgecolors : :mpltype:`color` or list of colors, default: :rc:`patch.edgecolor`
            Edge color for each patch making up the collection. The special
            value 'face' can be passed to make the edgecolor match the
            facecolor.
        facecolors : :mpltype:`color` or list of colors, default: :rc:`patch.facecolor`
            Face color for each patch making up the collection.
        hatchcolors : :mpltype:`color` or list of colors, default: :rc:`hatch.color`
            Hatch color for each patch making up the collection. The color
            can be set to the special value 'edge' to make the hatchcolor match the
            edgecolor.
        linewidths : float or list of floats, default: :rc:`patch.linewidth`
            Line width for each patch making up the collection.
        linestyles : str or tuple or list thereof, default: 'solid'
            Valid strings are ['solid', 'dashed', 'dashdot', 'dotted', '-',
            '--', '-.', ':']. Dash tuples should be of the form::

                (offset, onoffseq),

            where *onoffseq* is an even length tuple of on and off ink lengths
            in points. For examples, see
            :doc:`/gallery/lines_bars_and_markers/linestyles`.
        capstyle : `.CapStyle`-like, default: 'butt'
            Style to use for capping lines for all paths in the collection.
            Allowed values are %(CapStyle)s.
        joinstyle : `.JoinStyle`-like, default: 'round'
            Style to use for joining lines for all paths in the collection.
            Allowed values are %(JoinStyle)s.
        antialiaseds : bool or list of bool, default: :rc:`patch.antialiased`
            Whether each patch in the collection should be drawn with
            antialiasing.
        offsets : (float, float) or list thereof, default: (0, 0)
            A vector by which to translate each patch after rendering (default
            is no translation). The translation is performed in screen (pixel)
            coordinates (i.e. after the Artist's transform is applied).
        offset_transform : `~.Transform`, default: `.IdentityTransform`
            A single transform which will be applied to each *offsets* vector
            before it is used.
        cmap, norm
            Data normalization and colormapping parameters. See
            `.ScalarMappable` for a detailed description.
        hatch : str, optional
            Hatching pattern to use in filled paths, if any. Valid strings are
            ['/', '\\', '|', '-', '+', 'x', 'o', 'O', '.', '*']. See
            :doc:`/gallery/shapes_and_collections/hatch_style_reference` for
            the meaning of each hatch type.
        pickradius : float, default: 5.0
            If ``pickradius <= 0``, then `.Collection.contains` will return
            ``True`` whenever the test point is inside of one of the polygons
            formed by the control points of a Path in the Collection. On the
            other hand, if it is greater than 0, then we instead check if the
            test point is contained in a stroke of width ``2*pickradius``
            following any of the Paths in the Collection.
        urls : list of str, default: None
            A URL for each patch to link to once drawn. Currently only works
            for the SVG backend. See :doc:`/gallery/misc/hyperlinks_sgskip` for
            examples.
        zorder : float, default: 1
            The drawing order, shared by all Patches in the Collection. See
            :doc:`/gallery/misc/zorder_demo` for all defaults and examples.
        **kwargs
            Remaining keyword arguments will be used to set properties as
            ``Collection.set_{key}(val)`` for each key-value pair in *kwargs*.
        """
        if not hasattr(self, "_native"):
            # The base class made directly: a collection of no paths yet.
            self._native = _collections.new_path_collection([])
        Artist.__init__(self)
        self._attach_colorizer(self._get_colorizer(cmap, norm, colorizer))
        if offset_transform is not None:
            self.set_offset_transform(offset_transform)
        self.set_facecolor(facecolors)
        self.set_edgecolor(edgecolors)
        self.set_linewidth(linewidths)
        self.set_linestyle(linestyles)
        self.set_antialiased(antialiaseds)
        self.set_pickradius(pickradius)
        self.set_urls(urls)
        self.set_hatch(hatch)
        self.set_hatchcolor(hatchcolors)
        self.set_zorder(zorder)
        if capstyle:
            self.set_capstyle(capstyle)
        if joinstyle:
            self.set_joinstyle(joinstyle)
        if offsets is not None:
            _collection_arguments.set_made_offsets(self, offsets)
        self._internal_update(kwargs)

    @classmethod
    def _from_native(cls, native, figure):
        collection = super()._from_native(native, figure)
        collection._attach_native_mapping()
        return collection

    def _native_clim(self):
        """The limits of the mapping pltrs (Rust) made."""
        return _collections.get_clim(self._native)

    def _drawn_clim(self):
        # docstring inherited
        return self._native_clim()

    def _native_cmap(self):
        """The colormap of the mapping pltrs (Rust) made, as
        ``_native.cm`` describes it (see `.cm._from_native`)."""
        return _collections.get_cmap(self._native)

    def _native_array(self):
        """The values pltrs (Rust) maps, masked where invalid, or None."""
        array = _collections.get_array(self._native)
        # The array is a fresh copy: masking it in place spares another
        # (as matplotlib's ``set_array`` masks it, with no mask array when
        # every value is valid).
        return None if array is None else cbook.safe_masked_invalid(array)

    def _attach_native_mapping(self, cmap=None, norm=None, colorizer=None):
        """
        Give the collection the Python colorizer of the mapping pltrs (Rust)
        made: *colorizer*, or one of *cmap* (by default the colormap pltrs
        used) and *norm*, with the limits pltrs found, and the values it
        maps.
        """
        if colorizer is None:
            if cmap is None:
                from .cm import _from_native
                cmap = _from_native(self._native_cmap())
            colorizer = mcolorizer.Colorizer(cmap, norm)
        vmin, vmax = self._native_clim()
        # A norm with its limits keeps them (it may not let them be set).
        if (vmin is not None or vmax is not None) and not colorizer.norm.scaled():
            colorizer.set_clim(vmin, vmax)
        if getattr(self, "_colorizer", None) is not None:
            self._colorizer.callbacks.disconnect(self._id_colorizer)
        self._attach_colorizer(colorizer)
        self._A = self._native_array()

    def _mapping_changed(self):
        # docstring inherited
        if self._maps_variates():
            self._map_variates()
        else:
            _collections.set_mapping(self._native, self.cmap, self._colorizer.norm,
                                     self._handing_norm())

    def _map_variates_through(self, colorizer, A):
        """Map the values *A* of several variates through *colorizer*, a
        bivariate or multivariate one, in place of the mapping pltrs (Rust)
        made the collection with."""
        self._colorizer.callbacks.disconnect(self._id_colorizer)
        self._attach_colorizer(colorizer)
        self.set_array(A)

    def _maps_variates(self):
        """Whether the collection maps several variates to colours (a
        bivariate or multivariate colormap)."""
        return self.norm.n_components > 1

    def _map_variates(self):
        """
        Give pltrs (Rust) the colours the bivariate or multivariate mapping
        gives the values (matplotlib's ``_mapped_colors``), which its norms
        and colormap compute, to put on the sides the mapping colours.
        """
        colors = None
        if self._A is not None:
            alpha = self.get_alpha()
            if np.iterable(alpha):
                alpha = np.reshape(alpha, self._A.shape)
            colors = np.reshape(self.to_rgba(self._A, alpha), (-1, 4))
        _collections.set_program_colors(self._native, colors)

    def update_scalarmappable(self):
        """
        Update colors from the scalar mappable array, if any.

        Assign colors to edges and faces based on the array and/or
        colors that were directly set, as appropriate.
        """
        if self._A is None:
            _collections.update_scalarmappable(self._native)
            return
        if self._maps_variates():
            self._map_variates()
        if self._A.ndim > 1 and not isinstance(self, (QuadMesh, PolyQuadMesh)):
            _refuse_rank(self._A)
        alpha = self._get("alpha_array")
        if alpha is not None and len(alpha) != self._A.size:
            alpha = np.asarray(self.__dict__.get("_array_alpha", alpha))
            raise ValueError(f'Data array shape, {self._A.shape} '
                             'is incompatible with alpha array shape, '
                             f'{alpha.shape}. '
                             'This can occur with the deprecated '
                             'behavior of the "flat" shading option, '
                             'in which a row and/or column of the data '
                             'array is dropped.')
        _collections.update_scalarmappable(self._native)

    def get_offset_transform(self):
        """Return the `.Transform` instance used by this artist offset."""
        given = self.__dict__.get("_offset_transform")
        if given is not None:
            return given
        return _live_transforms.of_artist(
            self, _collections.get_offset_transform(self._native), "offset")

    def set_offset_transform(self, offset_transform):
        """
        Set the artist offset transform.

        Parameters
        ----------
        offset_transform : `.Transform`
        """
        if (offset_transform is None or isinstance(offset_transform, mtransforms.Transform)
                or hasattr(offset_transform, '_as_mpl_transform')):
            _collections.set_offset_transform(
                self._native, _live_transforms.to_native(offset_transform))
            _unusable.drop(self, "offset_transform")
        else:
            # matplotlib keeps it, and fails to use it as it draws and
            # measures the collection; pltrs (Rust) keeps the one it had.
            _unusable.keep(self, "offset_transform", offset_transform,
                           operator.attrgetter("is_affine"))
        self._offset_transform = offset_transform

    def _get(self, name):
        """The native property *name*."""
        return _collections.get(self._native, name)

    #: The kind of sequence the paths were given in (matplotlib keeps them
    #: as given: ``scatter`` gives its marker as a tuple).
    _paths_container = list

    def get_paths(self):
        # The paths of an `EllipseCollection` before it is first drawn.
        unit_paths = self.__dict__.get("_unit_paths")
        if unit_paths is not None:
            return unit_paths
        if _values.is_kept(self, "paths"):
            return self.__dict__["_given_values"]["paths"]
        native = _collections.get_paths(self._native)
        kept = self.__dict__.get("_kept_paths")
        if kept is not None and _same_paths(kept[0], native):
            return kept[1]
        paths = [Path(vertices, codes) for vertices, codes in native]
        if _collections.has_single_path(self._native):
            paths = paths[0]
            _owned_paths.hand_out([paths], self._push_kept_paths)
        else:
            paths = self._paths_container(paths)
            if paths is not None:
                _owned_paths.hand_out(paths, self._push_kept_paths)
        self._kept_paths = (native, paths)
        return paths

    def _push_kept_paths(self):
        """Hand pltrs (Rust) the paths handed out again, one of which the
        program changed (see `._owned_paths`); they stay those handed out."""
        kept = self.__dict__.get("_kept_paths")
        if kept is None:
            return
        paths = kept[1]
        if isinstance(paths, Path):
            _collections.set_single_path(self._native, paths)
        else:
            _collections.set_paths(self._native, list(paths))
        self._kept_paths = (_collections.get_paths(self._native), paths)

    _paths = property(get_paths, doc="The paths of the items.")

    def set_paths(self, paths):
        self.__dict__.pop("_unit_paths", None)
        if not (paths is None or isinstance(paths, Path)
                or (np.iterable(paths) and all(isinstance(path, Path) for path in paths))):
            # matplotlib keeps them as given; pltrs (Rust) keeps the paths it
            # had.
            _values.keep(self, "paths", paths)
            return
        _values.remember(self, "paths", paths)
        if isinstance(paths, Path):
            _collections.set_single_path(self._native, paths)
        else:
            self._paths_container = _paths_container(paths)
            _collections.set_paths(self._native, [] if paths is None else list(paths))
        self.__dict__.pop("_kept_paths", None)
        if isinstance(paths, list) and all(isinstance(path, Path) for path in paths):
            # The list itself, as matplotlib's collection keeps it.
            _owned_paths.hand_out(paths, self._push_kept_paths)
            self._kept_paths = (_collections.get_paths(self._native), paths)
        _program_paths.keep(self, None if isinstance(paths, Path) else paths)

    def get_transforms(self):
        drawn = self._drawn_transforms()
        if drawn is not None:
            return drawn
        dpi, draws = self.__dict__.get("_sizes_scaled", (72.0, 0))
        figure = self.get_figure(root=True)
        if figure is not None and _artist.draw_count(self._native) > draws:
            dpi = float(figure.dpi)
        return _collections.get_transforms(self._native, dpi)

    def _drawn_transforms(self):
        """The transforms the class' draws compute, which `get_transforms`
        gives once it is drawn (an `EllipseCollection`'s), else None."""
        return None

    def get_offsets(self):
        """Return the offsets for the collection."""
        if not self._has_offsets():
            return np.zeros((1, 2))
        offsets, masked = _collections.get_offsets(self._native)
        # A collection made with its offsets (barbs) keeps their dtype.
        dtype = self.__dict__.get("_offsets_dtype")
        if dtype is not None and not masked:
            offsets = offsets.astype(dtype, copy=False)
        return np.ma.masked_invalid(offsets) if masked else offsets

    def _has_offsets(self):
        """Whether the items are placed at offsets (matplotlib's ``_offsets
        is not None``)."""
        return _collections.get_offset_transform(self._native) is not None

    def get_window_extent(self, renderer=None):
        return self.get_datalim(mtransforms.IdentityTransform())

    def _add_to_axes(self, axes, kind, autolim=True):
        # The data limits come from the paths as they are now.
        _program_paths.sync(self)
        super()._add_to_axes(axes, kind, autolim)
        if autolim and any(takes_from_program(type(self), name)
                           for name in ("get_paths", "get_datalim")):
            # matplotlib measures the collection through its `get_datalim`,
            # which reads the paths a class of the program's gives
            # (cartopy's ``FeatureArtist`` projects its geometries there):
            # pltrs (Rust) counts the paths it holds, the class' measure
            # adds to them, here and in `relim`.
            axes._measure_in_relim(self)
            axes._update_collection_limits(self)
        return self

    def get_datalim(self, transData):
        # This operation depends on the transforms for the data in the
        # collection and whether the collection has offsets:
        #
        # 1. offsets = None, transform child of transData: use the paths for
        # the automatic limits (i.e. for LineCollection in streamline).
        # 2. offsets != None: offset_transform is child of transData:
        #
        #    a. transform is child of transData: use the path + offset for
        #       limits (i.e for bar).
        #    b. transform is not a child of transData: just use the offsets
        #       for the limits (i.e. for scatter)
        #
        # 3. otherwise return a null Bbox.

        transform = self.get_transform()
        offset_trf = self.get_offset_transform()
        if not (isinstance(offset_trf, mtransforms.IdentityTransform)
                or offset_trf.contains_branch(transData)):
            # if the offsets are in some coords other than data,
            # then don't use them for autoscaling.
            return mtransforms.Bbox.null()

        paths = self.get_paths()
        if not len(paths):
            # No paths to transform
            return mtransforms.Bbox.null()

        if not transform.is_affine:
            paths = [transform.transform_path_non_affine(p) for p in paths]
            # Don't convert transform to transform.get_affine() here because
            # we may have transform.contains_branch(transData) but not
            # transforms.get_affine().contains_branch(transData).  But later,
            # be careful to only apply the affine part that remains.

        offsets = self.get_offsets()

        if any(transform.contains_branch_separately(transData)):
            # collections that are just in data units (like quiver)
            # can properly have the axes limits set by their shape +
            # offset.  LineCollections that have no offsets can
            # also use this algorithm (like streamplot).
            if isinstance(offsets, np.ma.MaskedArray):
                offsets = offsets.filled(np.nan)
                # get_path_collection_extents handles nan but not masked arrays
            data_trf = transform.get_affine() - transData
            if not data_trf.is_affine:
                paths = [data_trf.transform_path_non_affine(p) for p in paths]
            return mpath.get_path_collection_extents(
                data_trf.get_affine(), paths,
                self.get_transforms(),
                offset_trf.transform_non_affine(offsets),
                offset_trf.get_affine().frozen())

        if self._has_offsets():
            # this is for collections that have their paths (shapes)
            # in physical, axes-relative, or figure-relative units
            # (i.e. like scatter). We can't uniquely set limits based on
            # those shapes, so we just set the limits based on their
            # location.
            offsets = (offset_trf - transData).transform(offsets)
            # note A-B means A B^{-1}
            offsets = np.ma.masked_invalid(offsets)
            if not offsets.mask.all():
                bbox = mtransforms.Bbox.null()
                bbox.update_from_data_xy(offsets.compressed().reshape(-1, 2))
                return bbox
        return mtransforms.Bbox.null()

    def set_offsets(self, offsets):
        """
        Set the offsets for the collection.

        Parameters
        ----------
        offsets : (N, 2) or (2,) array-like
        """
        offsets = np.asanyarray(offsets)
        if offsets.shape == (2,):  # Broadcast (2,) -> (1, 2) but nothing else.
            offsets = offsets[None, :]
        masked = isinstance(offsets, np.ma.MaskedArray)
        cstack = np.ma.column_stack if masked else np.column_stack
        offsets = cstack(
            (np.asanyarray(self.convert_xunits(offsets[:, 0]), float),
             np.asanyarray(self.convert_yunits(offsets[:, 1]), float)))
        _collections.set_offsets(
            self._native, np.ma.filled(offsets, np.nan).reshape(-1, 2), masked)
        self.__dict__.pop("_offsets_dtype", None)

    def get_pickradius(self):
        return _values.given(self, "pickradius",
                             _collections.get_pickradius(self._native))

    def set_pickradius(self, pickradius):
        """
        Set the pick radius used for containment tests.

        Parameters
        ----------
        pickradius : float
            Pick radius, in points.
        """
        if not isinstance(pickradius, Real):
            raise ValueError(
                f"pickradius must be a real-valued number, not {pickradius!r}")
        _collections.set_pickradius(self._native, float(pickradius))
        _values.remember(self, "pickradius", pickradius)

    pickradius = property(get_pickradius, set_pickradius)

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    def _draws_snapped(self):
        # docstring inherited: matplotlib draws the paths of a collection
        # with the collection's snapping.
        return True

    def contains(self, mouseevent):
        """
        Test whether the mouse event occurred in the collection.

        Returns ``bool, dict(ind=itemlist)``, where every item in itemlist
        contains the event.
        """
        return super().contains(mouseevent)

    def _contains_details(self, ind):
        """The indices of the items hit, C ``int``s as matplotlib's hit test
        gives them; none for a hidden collection, which matplotlib does not
        test."""
        return dict(ind=ind.astype(np.int32)) if self.get_visible() else {}

    def get_urls(self):
        """
        Return a list of URLs, one for each element of the collection.

        The list contains *None* for elements without a URL. See
        :doc:`/gallery/misc/hyperlinks_sgskip` for an example.
        """
        if _values.is_kept(self, "urls"):
            return _values.given(self, "urls", None)
        urls = _collections.get_urls(self._native)
        # A collection starts with `[None]` (matplotlib's `set_urls(None)`).
        if not urls and "urls" not in self.__dict__.get("_given_values", {}):
            return [None]
        return urls

    def set_urls(self, urls):
        """
        Parameters
        ----------
        urls : list of str or None

        Notes
        -----
        URLs are currently only implemented by the SVG backend. They are
        ignored by all other backends.
        """
        urls = urls if urls is not None else [None]
        # Kept as given (matplotlib writes any object as its text).
        _collections.set_urls(self._native, [None if url is None else str(url) for url in urls])
        _values.keep(self, "urls", urls)

    def get_path_effects(self):
        """Return the path effects (None until set, as matplotlib's
        collections start)."""
        return self.__dict__.get("_path_effects")

    def get_hatch(self):
        """Return the current hatching pattern."""
        return self._get("hatch")

    def set_hatch(self, hatch):
        r"""
        Set the hatching pattern

        *hatch* can be one of::

          /   - diagonal hatching
          \   - back diagonal
          |   - vertical
          -   - horizontal
          +   - crossed
          x   - crossed diagonal
          o   - small circle
          O   - large circle
          .   - dots
          *   - stars

        Letters can be combined, in which case all the specified
        hatchings are done.  If same letter repeats, it increases the
        density of hatching of that pattern.

        Unlike other properties such as linewidth and colors, hatching
        can only be specified for the collection as a whole, not separately
        for each member.

        Parameters
        ----------
        hatch : {'/', '\\', '|', '-', '+', 'x', 'o', 'O', '.', '*'}
        """
        from . import hatch as mhatch
        mhatch._validate_hatch_pattern(hatch)
        _collections.set_hatch(self._native, hatch)

    def get_hatch_linewidth(self):
        """Return the hatch linewidth."""
        return _values.given(self, "hatch_linewidth", self._get("hatch_linewidth"))

    def set_hatch_linewidth(self, lw):
        """Set the hatch linewidth."""
        if not isinstance(lw, Real):
            # Kept unchecked, as matplotlib keeps it until it draws.
            _values.keep(self, "hatch_linewidth", lw)
            _unusable.keep(self, "hatch_linewidth", lw, functools.partial(
                _collection_draw_checks.hatch_linewidth, self), measured=False)
            return
        native = float(lw)
        _collections.set_hatch_linewidth(self._native, native)
        _values.remember(self, "hatch_linewidth", lw)
        _unusable.drop(self, "hatch_linewidth")

    def get_hatchcolor(self):
        return self._get("hatchcolor")

    def set_hatchcolor(self, c):
        """
        Set the hatchcolor(s) of the collection.

        Parameters
        ----------
        c : :mpltype:`color` or list of :mpltype:`color` or 'edge'
            The collection hatchcolor(s).  If a sequence, the patches cycle
            through it.
        """
        if c is None:
            c = _rcParams()['hatch.color']
        _collections.set_hatchcolors(
            self._native, c if cbook._str_equal(c, 'edge') else _rgba(c))

    @_docstring.interpd
    def get_capstyle(self):
        """
        Return the cap style for the collection (for all its elements).

        Returns
        -------
        %(CapStyle)s or None
        """
        return self._get("capstyle")

    @_docstring.interpd
    def set_capstyle(self, cs):
        """
        Set the `.CapStyle` for the collection (for all its elements).

        Parameters
        ----------
        cs : `.CapStyle` or %(CapStyle)s
        """
        _collections.set_capstyle(self._native, CapStyle(cs).name)

    @_docstring.interpd
    def get_joinstyle(self):
        """
        Return the join style for the collection (for all its elements).

        Returns
        -------
        %(JoinStyle)s or None
        """
        return self._get("joinstyle")

    @_docstring.interpd
    def set_joinstyle(self, js):
        """
        Set the `.JoinStyle` for the collection (for all its elements).

        Parameters
        ----------
        js : `.JoinStyle` or %(JoinStyle)s
        """
        _collections.set_joinstyle(self._native, JoinStyle(js).name)

    def get_linewidth(self):
        return self._lwls()[0]

    def set_linewidth(self, lw):
        """
        Set the linewidth(s) for the collection.  *lw* can be a scalar
        or a sequence; if it is a sequence the patches will cycle
        through the sequence

        Parameters
        ----------
        lw : float or list of floats
        """
        if lw is None:
            lw = self._get_default_linewidth()
        lw = np.atleast_1d(lw)
        # matplotlib scales the dash patterns by the widths, failing on
        # widths that are no numbers.
        bcast, _ = self._bcast_lwls(lw, [mlines._get_dash_pattern(style)
                                         for style in self._given_linestyles()])
        if any(isinstance(w, str) for w in bcast):
            # matplotlib keeps widths given as text, which its renderers
            # take or fail on (`._collection_draw_checks`); pltrs (Rust)
            # draws with those a float reads.
            _values.keep(self, "linewidth", bcast)
            _unusable.keep(self, "linewidth", bcast, functools.partial(
                _collection_draw_checks.linewidths, self), measured=False)
            with contextlib.suppress(ValueError):
                _collections.set_linewidths(self._native, [float(w) for w in lw])
            return
        # Masked widths draw nothing, as NaN ones (and read back masked).
        widths = lw.astype(float).filled(np.nan) if np.ma.isMaskedArray(lw) else lw
        try:
            widths = [float(w) for w in widths]
        except (TypeError, ValueError):
            # matplotlib keeps them, and its renderers fail to take them.
            _values.keep(self, "linewidth", lw)
            _unusable.keep(self, "linewidth", lw, _drawn_linewidths, measured=False)
            return
        _collections.set_linewidths(self._native, widths)
        _values.remember(self, "linewidth", lw)
        _unusable.drop(self, "linewidth")

    def get_linestyle(self):
        return self._lwls()[1]

    def set_linestyle(self, ls):
        """
        Set the linestyle(s) for the collection.

        Parameters
        ----------
        ls : {'-', '--', '-.', ':', '', ...} or (offset, on-off-seq) or list thereof
            If a list, the individual elements are assigned to the elements of the
            collection.

            Possible values:

            - A string:

              =======================================================  ================
              linestyle                                                description
              =======================================================  ================
              ``'-'`` or ``'solid'``                                   solid line
              ``'--'`` or ``'dashed'``                                 dashed line
              ``'-.'`` or ``'dashdot'``                                dash-dotted line
              ``':'`` or ``'dotted'``                                  dotted line
              ``''`` or ``'none'`` (discouraged: ``'None'``, ``' '``)  draw nothing
              =======================================================  ================

            - A tuple describing the start position and lengths of dashes and spaces:

                  (offset, onoffseq)

              where

              - *offset* is a float specifying the offset (in points); i.e. how much
                is the dash pattern shifted.
              - *onoffseq* is a sequence of on and off ink in points. There can be
                arbitrary many pairs of on and off values.

              Example: The tuple ``(0, (10, 5, 1, 5))`` means that the pattern starts
              at the beginning of the line. It draws a 10 point long dash,
              then a 5 point long space, then a 1 point long dash, followed by a 5 point
              long space, and then the pattern repeats.

            For examples see :doc:`/gallery/lines_bars_and_markers/linestyles`.
        """
        styles = _checked_linestyles(ls)
        _collections.set_linestyles(self._native, [_native_style(s) for s in styles])
        # matplotlib scales the dash patterns as given (ints stay ints).
        _values.remember(self, "linestyle", styles)

    def _given_linestyles(self):
        """The line styles as given (matplotlib's ``_us_linestyles``),
        while pltrs (Rust) holds them, else those it holds."""
        return _values.given(self, "linestyle", self._get("linestyle"),
                             same=_same_linestyles)

    def _lwls(self):
        """The line widths and scaled dash patterns, broadcast as
        matplotlib's ``_bcast_lwls`` does."""
        linewidths = _values.given(
            self, "linewidth", np.asarray(self._get("linewidth"), dtype=float),
            same=_values.same_cycled)
        dashes = [mlines._get_dash_pattern(style) for style in self._given_linestyles()]
        return self._bcast_lwls(linewidths, dashes)

    @staticmethod
    def _bcast_lwls(linewidths, dashes):
        """
        Internal helper function to broadcast + scale ls/lw

        In the collection drawing code, the linewidth and linestyle are cycled
        through as circular buffers (via ``v[i % len(v)]``).  Thus, if we are
        going to scale the dash pattern at set time (not draw time) we need to
        do the broadcasting now and expand both lists to be the same length.

        Parameters
        ----------
        linewidths : list
            line widths of collection
        dashes : list
            dash specification (offset, (dash pattern tuple))

        Returns
        -------
        linewidths, dashes : list
            Will be the same length, dashes are scaled by paired linewidth
        """
        if _rcParams()['_internal.classic_mode']:
            return linewidths, dashes
        # make sure they are the same length so we can zip them
        if len(dashes) != len(linewidths):
            l_dashes = len(dashes)
            l_lw = len(linewidths)
            gcd = math.gcd(l_dashes, l_lw)
            dashes = list(dashes) * (l_lw // gcd)
            linewidths = list(linewidths) * (l_dashes // gcd)

        # scale the dash patterns
        dashes = [mlines._scale_dashes(o, d, lw)
                  for (o, d), lw in zip(dashes, linewidths)]

        return linewidths, dashes

    def _get_default_linewidth(self):
        return _rcParams()['patch.linewidth']

    def _get_default_facecolor(self):
        """The face colour of the collection when none is set."""
        return _rcParams()['patch.facecolor']

    def _get_default_edgecolor(self):
        """The edge colour of the collection when none is set."""
        return _rcParams()['patch.edgecolor']

    def get_antialiased(self):
        """
        Get the antialiasing state for rendering.

        Returns
        -------
        array of bools
        """
        antialiased = self._get("antialiased")
        if antialiased is None:
            antialiased = [_rcParams()['patch.antialiased']]
        return np.array(antialiased, dtype=bool)

    def set_antialiased(self, aa):
        """
        Set the antialiasing state for rendering.

        Parameters
        ----------
        aa : bool or list of bools
        """
        if aa is None:
            aa = _rcParams()['patch.antialiased']
        _collections.set_antialiaseds(
            self._native, [bool(a) for a in np.atleast_1d(aa)])

    def get_fill(self):
        """Return whether face is colored."""
        # matplotlib answers from the face colour given last: colours as an
        # array, transparent ones too, count as a face.
        fill = self.__dict__.get("_face_fill")
        return _collections.get_fill(self._native) if fill is None else fill

    def update_from(self, other):
        """Copy properties from other to self."""
        _artist_copying.check_copyable(other)
        self._took_from(other, _collections.update_from(self._native, other._native))
        self._array_alpha = other.__dict__.get("_array_alpha")
        self._face_fill = other.__dict__.get("_face_fill")
        # update_from for scalarmappable
        self._A = other._A
        self.norm = other.norm
        self.cmap = other.cmap
        self.stale = True

    def get_facecolor(self):
        return self._get("facecolor")

    @property
    def _facecolors(self):
        """The RGBA face colours, as drawn (matplotlib's ``_facecolors``,
        which `get_facecolor` gives)."""
        return self.get_facecolor()

    def set_facecolor(self, c):
        """
        Set the facecolor(s) of the collection. *c* can be a color (all patches
        have same color), or a sequence of colors; if it is a sequence the
        patches will cycle through the sequence.

        If *c* is 'none', the patch will not be filled.

        Parameters
        ----------
        c : :mpltype:`color` or list of :mpltype:`color`
        """
        if isinstance(c, str) and c.lower() in ("none", "face"):
            c = c.lower()
        self._original_facecolor = c
        _refused_colors.take(self, "facecolor", c, lambda: _collections.set_facecolors(
            self._native, None if c is None else _rgba(c)))
        self._face_fill = not cbook._str_lower_equal(c, "none")

    def get_edgecolor(self):
        return self._get("edgecolor")

    @property
    def _offsets(self):
        """The offsets of the items, None without (matplotlib's attribute,
        which `get_offsets` gives as a ``(1, 2)`` array of zeros then)."""
        return self.get_offsets() if self._has_offsets() else None

    @property
    def _edgecolors(self):
        """The RGBA edge colours, as drawn, or ``'face'`` while the edges
        take the face colours (matplotlib's ``_edgecolors``, which
        `get_edgecolor` gives in RGBA)."""
        if cbook._str_equal(self._get("edgecolor_setting"), "face"):
            return "face"
        return self.get_edgecolor()

    def set_edgecolor(self, c):
        """
        Set the edgecolor(s) of the collection.

        Parameters
        ----------
        c : :mpltype:`color` or list of :mpltype:`color` or 'face'
            The collection edgecolor(s).  If a sequence, the patches cycle
            through it.  If 'face', match the facecolor.
        """
        if isinstance(c, str) and c.lower() in ("none", "face"):
            c = c.lower()
        _refused_colors.take(self, "edgecolor", c, lambda: _collections.set_edgecolors(
            self._native, _edge_spec(c)))
        if c is None and "_original_facecolor" in self.__dict__:
            # matplotlib decides from the face colour given last, which a
            # colour it refused still is.
            _collections.decide_default_edges_as(
                self._native, cbook._str_equal(self._original_facecolor, "none"))

    def set_color(self, c):
        """
        Set the edgecolor, facecolor and hatchcolor.

        .. versionchanged:: 3.11
            Now sets the hatchcolor as well.

        Parameters
        ----------
        c : :mpltype:`color` or list of RGBA tuples

        See Also
        --------
        Collection.set_facecolor, Collection.set_edgecolor, Collection.set_hatchcolor
            For setting the facecolor, edgecolor, and hatchcolor individually.
        """
        self.set_facecolor(c)
        self.set_edgecolor(c)
        self.set_hatchcolor(c)

    def set_alpha(self, alpha):
        _check_array_alpha(alpha)
        if np.iterable(alpha):
            alpha = np.asarray(alpha, dtype=float)
            # The array itself comes back, as matplotlib keeps it, even
            # when it does not fit the colours.
            self._array_alpha = alpha
            _collections.set_alphas(self._native, alpha.ravel().tolist(),
                                    list(alpha.shape))
        else:
            array = self.__dict__.get("_array_alpha")
            if array is not None:
                # matplotlib's `Artist.set_alpha` compares the alpha with
                # the array it had (`alpha != self._alpha`), whose truth
                # numpy refuses unless it holds exactly one value.
                bool(alpha != array)
            self._array_alpha = None
            super().set_alpha(alpha)
            # The colours read back as set again, until the next draw.
            _collections.set_alpha(self._native, alpha)
        if self._A is not None and self._maps_variates():
            self._map_variates()

    set_alpha.__doc__ = Artist._set_alpha_for_array.__doc__

    @property
    def _alpha(self):
        # An array of alphas as given, else the alpha.
        array = self.__dict__.get("_array_alpha")
        return super()._alpha if array is None else array

    def _array_set(self):
        # docstring inherited
        if self._A is not None and self._A.ndim > 1 and not isinstance(
                self, (QuadMesh, PolyQuadMesh)):
            # matplotlib keeps it, and refuses it when it draws
            # (`update_scalarmappable`).
            _unusable.keep(self, "array", self._A, _refuse_rank, measured=False)
        else:
            _unusable.drop(self, "array")
        if self._A is None or self._maps_variates():
            _collections.set_array(self._native, None)
            if self._maps_variates():
                self._map_variates()
            return
        _collections.set_array(
            self._native, np.ma.filled(self._A.astype(float), np.nan).ravel())
        _collections.set_mapping(self._native, self.cmap, self._colorizer.norm,
                                 self._handing_norm())


class _CollectionWithSizes(Collection):
    """Base class for collections that have an array of sizes."""

    _factor = 1.0

    def get_sizes(self):
        """
        Return the sizes ('areas') of the elements in the collection.

        Returns
        -------
        array
            The 'area' of each element.
        """
        sizes = _collections.get_sizes(self._native)
        return _unusable.kept(self, "sizes", _values.given(
            self, "sizes", np.zeros(0) if sizes is None else sizes))

    def set_sizes(self, sizes, dpi=72.0):
        """
        Set the sizes of each member of the collection.

        Parameters
        ----------
        sizes : `numpy.ndarray` or None
            The size to set for each element of the collection.  The
            value is the 'area' of the element.
        dpi : float, default: 72
            The dpi of the canvas.
        """
        if sizes is None:
            sizes = np.array([])
        sizes = np.asarray(sizes)
        try:
            # matplotlib's scales of the items, which fail and warn on
            # sizes that are no numbers or negative, the sizes kept.
            _collection_draw_checks.size_scales(sizes, dpi)
        except (TypeError, ValueError):
            _values.keep(self, "sizes", sizes)
            _unusable.keep(self, "sizes", sizes, _collection_draw_checks.size_scales,
                           measured=False)
            raise
        _collections.set_sizes(self._native, np.asarray(sizes, float))
        _values.remember(self, "sizes", sizes)
        if len(sizes) or not isinstance(self, RegularPolyCollection):
            _unusable.drop(self, "sizes")
        else:
            # Regular polygons of no rotated transforms, which matplotlib's
            # renderers fail to draw.
            _unusable.keep(self, "sizes", sizes, functools.partial(
                _collection_draw_checks.no_sizes, self), measured=False)
        # The resolution the items are scaled at until the next draw.
        self._sizes_scaled = (float(dpi), _artist.draw_count(self._native))

    @allow_rasterization
    def draw(self, renderer):
        # matplotlib's draw sets the sizes anew at the figure's resolution
        # first, which makes the collection stale, and its Axes and figure
        # with it: drawn alone (``draw_artist``, blitting), it leaves them
        # stale.
        self.stale = True
        super().draw(renderer)


class PathCollection(_CollectionWithSizes):
    r"""
    A collection of `~.path.Path`\s, as created by e.g. `~.Axes.scatter`.
    """

    def __init__(self, paths, sizes=None, **kwargs):
        """
        Parameters
        ----------
        paths : list of `.path.Path`
            The paths that will make up the `.Collection`.
        sizes : array-like
            The factor by which to scale each drawn `~.path.Path`. One unit
            squared in the Path's data space is scaled to be ``sizes**2``
            points when rendered.
        **kwargs
            Forwarded to `.Collection`.
        """
        held = _collection_arguments.held_paths(paths)
        self._native = _collections.new_path_collection([] if held is None else held)
        self._paths_container = _paths_container(paths)
        super().__init__(**kwargs)
        if held is None:
            # matplotlib keeps them as given.
            self.set_paths(paths)
        self.set_sizes(sizes)
        _program_paths.keep(self, paths)

    def get_paths(self):
        return super().get_paths()

    def legend_elements(self, prop="colors", num="auto",
                        fmt=None, func=lambda x: x, **kwargs):
        """
        Create legend handles and labels for a PathCollection.

        Each legend handle is a `.Line2D` representing the Path that was drawn,
        and each label is a string that represents the Path.

        This is useful for obtaining a legend for a `~.Axes.scatter` plot;
        e.g.::

            scatter = plt.scatter([1, 2, 3],  [4, 5, 6],  c=[7, 2, 3], num=None)
            plt.legend(*scatter.legend_elements())

        creates three legend elements, one for each color with the numerical
        values passed to *c* as the labels.

        Also see the :ref:`automatedlegendcreation` example.

        Parameters
        ----------
        prop : {"colors", "sizes"}, default: "colors"
            If "colors", the legend handles will show the different colors of
            the collection. If "sizes", the legend will show the different
            sizes. To set both, use *kwargs* to directly edit the `.Line2D`
            properties.
        num : int, None, "auto" (default), array-like, or `~.ticker.Locator`
            Target number of elements to create.
            If None, use all unique elements of the mappable array. If an
            integer, target to use *num* elements in the normed range.
            If *"auto"*, try to determine which option better suits the nature
            of the data.
            The number of created elements may slightly deviate from *num* due
            to a `~.ticker.Locator` being used to find useful locations.
            If a list or array, use exactly those elements for the legend.
            Finally, a `~.ticker.Locator` can be provided.
        fmt : str, `~matplotlib.ticker.Formatter`, or None (default)
            The format or formatter to use for the labels. If a string must be
            a valid input for a `.StrMethodFormatter`. If None (the default),
            use a `.ScalarFormatter`.
        func : function, default: ``lambda x: x``
            Function to calculate the labels.  Often the size (or color)
            argument to `~.Axes.scatter` will have been pre-processed by the
            user using a function ``s = f(x)`` to make the markers visible;
            e.g. ``size = np.log10(x)``.  Providing the inverse of this
            function here allows that pre-processing to be inverted, so that
            the legend labels have the correct values; e.g. ``func = lambda
            x: 10**x``.
        **kwargs
            Allowed keyword arguments are *color* and *size*. E.g. it may be
            useful to set the color of the markers if *prop="sizes"* is used;
            similarly to set the size of the markers if *prop="colors"* is
            used. Any further parameters are passed onto the `.Line2D`
            instance. This may be useful to e.g. specify a different
            *markeredgecolor* or *alpha* for the legend handles.

        Returns
        -------
        handles : list of `.Line2D`
            Visual representation of each element of the legend.
        labels : list of str
            The string labels for elements of the legend.
        """
        from . import rcParams, ticker as mticker
        handles = []
        labels = []
        hasarray = self.get_array() is not None
        if fmt is None:
            fmt = mticker.ScalarFormatter(useOffset=False, useMathText=True)
        elif isinstance(fmt, str):
            fmt = mticker.StrMethodFormatter(fmt)
        fmt.create_dummy_axis()

        if prop == "colors":
            if not hasarray:
                warnings.warn("Collection without array used. Make sure to "
                              "specify the values to be colormapped via the "
                              "`c` argument.")
                return handles, labels
            u = np.unique(self.get_array())
            size = kwargs.pop("size", rcParams["lines.markersize"])
        elif prop == "sizes":
            u = np.unique(self.get_sizes())
            color = kwargs.pop("color", "k")
        else:
            raise ValueError("Valid values for `prop` are 'colors' or "
                             f"'sizes'. You supplied '{prop}' instead.")

        fu = func(u)
        fmt.axis.set_view_interval(fu.min(), fu.max())
        fmt.axis.set_data_interval(fu.min(), fu.max())
        if num == "auto":
            num = 9
            if len(u) <= num:
                num = None
        if num is None:
            values = u
            label_values = func(values)
        else:
            if prop == "colors":
                arr = self.get_array()
            elif prop == "sizes":
                arr = self.get_sizes()
            if isinstance(num, mticker.Locator):
                loc = num
            elif np.iterable(num):
                loc = mticker.FixedLocator(num)
            else:
                num = int(num)
                loc = mticker.MaxNLocator(nbins=num, min_n_ticks=num-1,
                                          steps=[1, 2, 2.5, 3, 5, 6, 8, 10])
            label_values = loc.tick_values(func(arr).min(), func(arr).max())
            cond = ((label_values >= func(arr).min()) &
                    (label_values <= func(arr).max()))
            label_values = label_values[cond]
            yarr = np.linspace(arr.min(), arr.max(), 256)
            xarr = func(yarr)
            ix = np.argsort(xarr)
            values = np.interp(label_values, xarr[ix], yarr[ix])

        kw = {"markeredgewidth": self.get_linewidths()[0],
              "alpha": self.get_alpha(),
              **kwargs}

        # Pre-format all labels, passing each formatter call the value alone
        # (as matplotlib does).
        fmt.set_locs(label_values)
        formatted_labels = [fmt(lab) for lab in label_values]

        for val, formatted in zip(values, formatted_labels):
            if prop == "colors":
                color = self.cmap(self.norm(val))
            elif prop == "sizes":
                size = np.sqrt(val)
                if np.isclose(size, 0.0):
                    continue
            h = mlines.Line2D([0], [0], ls="", color=color, ms=size,
                              marker=self.get_paths()[0], **kw)
            handles.append(h)
            labels.append(formatted)

        return handles, labels


class PolyCollection(_CollectionWithSizes):
    def __init__(self, verts, sizes=None, *, closed=True, **kwargs):
        """
        Parameters
        ----------
        verts : list of array-like
            The sequence of polygons [*verts0*, *verts1*, ...] where each
            element *verts_i* defines the vertices of polygon *i* as a 2D
            array-like of shape (M, 2).
        sizes : array-like, default: None
            Squared scaling factors for the polygons. The coordinates of each
            polygon *verts_i* are multiplied by the square-root of the
            corresponding entry in *sizes* (i.e., *sizes* specify the scaling
            of areas). The scaling is applied before the Artist master
            transform.
        closed : bool, default: True
            Whether the polygon should be closed by adding a CLOSEPOLY
            connection at the end.
        **kwargs
            Forwarded to `.Collection`.
        """
        self._native = _collections.new_poly_collection(_polygon_vertices(verts, closed))
        super().__init__(**kwargs)
        if sizes is not None:
            self.set_sizes(sizes)
        if not closed:
            self.set_verts(verts, closed=False)

    def set_verts(self, verts, closed=True):
        """
        Set the vertices of the polygons.

        Parameters
        ----------
        verts : list of array-like
            The sequence of polygons [*verts0*, *verts1*, ...] where each
            element *verts_i* defines the vertices of polygon *i* as a 2D
            array-like of shape (M, 2).
        closed : bool, default: True
            Whether the polygon should be closed by adding a CLOSEPOLY
            connection at the end.
        """
        # New paths, as matplotlib makes them.
        self.__dict__.pop("_kept_paths", None)
        if not closed:
            _collections.set_segments(self._native, _polygon_vertices(verts, closed))
            return
        polygons = []
        try:
            for vertices in _each_polygon_vertices(verts, closed):
                polygons.append(vertices)
        finally:
            # matplotlib's collection keeps the polygons it made before it
            # failed (its list of closed paths is new from the start).
            _collections.set_verts(self._native, polygons)

    set_paths = set_verts

    def set_verts_and_codes(self, verts, codes):
        """Initialize vertices with path codes."""
        if len(verts) != len(codes):
            raise ValueError("'codes' must be a 1D list or array "
                             "with the same length of 'verts'")
        Collection.set_paths(self, [Path(xy, cds) if len(xy) else Path(xy)
                                    for xy, cds in zip(verts, codes)])
        self.stale = True


def _path_vertices(vertices):
    """*vertices* as the (N, 2) float array of matplotlib's `.Path` of
    them (masked points as NaN), refused as `.Path` refuses another
    shape."""
    vertices = cbook._to_unmasked_float_array(vertices)
    _api.check_shape((None, 2), vertices=vertices)
    return vertices


def _each_polygon_vertices(verts, closed):
    """The (N, 2) float arrays of the polygons *verts*, one by one, refused
    as matplotlib's `.PolyCollection.set_verts` refuses them making their
    paths: it counts the points of each polygon to close, and checks the
    shape of its points with the first repeated."""
    if isinstance(verts, np.ma.MaskedArray):
        verts = verts.astype(float).filled(np.nan)
    for xy in verts:
        if not closed or not len(xy):
            yield _path_vertices(xy)
            continue
        vertices = cbook._to_unmasked_float_array(xy)
        _api.check_shape((None, 2), vertices=np.concatenate([vertices, vertices[:1]]))
        yield vertices


def _polygon_vertices(verts, closed):
    """The (N, 2) float arrays of all the polygons *verts* (see
    `_each_polygon_vertices`)."""
    return list(_each_polygon_vertices(verts, closed))


def _check_step(step, curves, where):
    """Raise matplotlib's error for a *step* its fills cannot draw with:
    they look ``"steps-" + step`` up in `.cbook.STEP_LOOKUP_MAP` (a
    `TypeError` for no string, a `KeyError` for an unknown one) for each
    region they fill, the points of the *curves* (NaN where masked) that
    *where* (None for all) keeps."""
    if step is None:
        return
    filled = ~np.logical_or.reduce([np.isnan(curve) for curve in curves])
    if where is not None:
        filled &= np.asarray(where, dtype=bool)
    if filled.any():
        cbook.STEP_LOOKUP_MAP["steps-" + step]


class FillBetweenPolyCollection(PolyCollection):
    """
    `.PolyCollection` that fills the area between two x- or y-curves.
    """

    def get_datalim(self, transData):
        """Calculate the data limits and return them as a `.Bbox`."""
        # matplotlib's: the box of the points filled between, where they
        # are filled, and their smallest positive coordinates (its
        # ``_bbox``).
        (x0, y0, x1, y1), minpos = _collections.fill_between_limits(self._native)
        datalim = mtransforms.Bbox.null()
        datalim.update_from_data_xy((self.get_transform() - transData).transform(
            np.array([[x0, y0], [x1, y1], minpos])))
        return datalim

    def __init__(
            self, t_direction, t, f1, f2, *,
            where=None, interpolate=False, step=None, **kwargs):
        """
        Parameters
        ----------
        t_direction : {{'x', 'y'}}
            The axes on which the variable lies.

            - 'x': the curves are ``(t, f1)`` and ``(t, f2)``.
            - 'y': the curves are ``(f1, t)`` and ``(f2, t)``.

        t : array-like
            The ``t_direction`` coordinates of the nodes defining the curves.

        f1 : array-like or float
            The other coordinates of the nodes defining the first curve.

        f2 : array-like or float
            The other coordinates of the nodes defining the second curve.

        where : array-like of bool, optional
            Define *where* to exclude some {dir} regions from being filled.
            The filled regions are defined by the coordinates ``t[where]``.
            More precisely, fill between ``t[i]`` and ``t[i+1]`` if
            ``where[i] and where[i+1]``.  Note that this definition implies
            that an isolated *True* value between two *False* values in *where*
            will not result in filling.  Both sides of the *True* position
            remain unfilled due to the adjacent *False* values.

        interpolate : bool, default: False
            This option is only relevant if *where* is used and the two curves
            are crossing each other.

            Semantically, *where* is often used for *f1* > *f2* or
            similar.  By default, the nodes of the polygon defining the filled
            region will only be placed at the positions in the *t* array.
            Such a polygon cannot describe the above semantics close to the
            intersection.  The t-sections containing the intersection are
            simply clipped.

            Setting *interpolate* to *True* will calculate the actual
            intersection point and extend the filled region up to this point.

        step : {{'pre', 'post', 'mid'}}, optional
            Define *step* if the filling should be a step function,
            i.e. constant in between *t*.  The value determines where the
            step will occur:

            - 'pre': The f value is continued constantly to the left from
              every *t* position, i.e. the interval ``(t[i-1], t[i]]`` has the
              value ``f[i]``.
            - 'post': The y value is continued constantly to the right from
              every *x* position, i.e. the interval ``[t[i], t[i+1])`` has the
              value ``f[i]``.
            - 'mid': Steps occur half-way between the *t* positions.

        **kwargs
            Forwarded to `.PolyCollection`.

        See Also
        --------
        .Axes.fill_between, .Axes.fill_betweenx
        """
        self._f_dir_from_t(t_direction)
        self.t_direction = t_direction
        # matplotlib checks the arrays as given, which fails for anything
        # else.
        self._validate_shapes(t_direction, self._f_direction, t, f1, f2)
        curves, mask = self._checked_data(t_direction, t, f1, f2, where)
        _check_step(step, curves, mask)
        self._native = _collections.new_fill_between_collection(
            t_direction == "y", curves, mask, bool(interpolate), step)
        Collection.__init__(self, **kwargs)

    @staticmethod
    def _f_dir_from_t(t_direction):
        """The direction that is other than `t_direction`."""
        if t_direction == "x":
            return "y"
        elif t_direction == "y":
            return "x"
        else:
            msg = f"t_direction must be 'x' or 'y', got {t_direction!r}"
            raise ValueError(msg)

    @property
    def _f_direction(self):
        """The direction that is other than `self.t_direction`."""
        return self._f_dir_from_t(self.t_direction)

    @staticmethod
    def _validate_shapes(t_dir, f_dir, t, f1, f2):
        """Validate that t, f1 and f2 are 1-dimensional and have the same length."""
        names = (d + s for d, s in zip((t_dir, f_dir, f_dir), ("", "1", "2")))
        for name, array in zip(names, [t, f1, f2]):
            if array.ndim > 1:
                raise ValueError(f"{name!r} is not 1-dimensional")
            if t.size > 1 and array.size > 1 and t.size != array.size:
                msg = "{!r} has size {}, but {!r} has an unequal size of {}".format(
                    t_dir, t.size, name, array.size)
                raise ValueError(msg)

    @classmethod
    def _checked_data(cls, t_dir, t, f1, f2, where):
        """
        The curves along *t_dir* as float arrays broadcast together (masked
        points as NaN, which are not filled) and *where* as a list of flags,
        checked as matplotlib's ``_validate_shapes`` and ``_get_data_mask``
        do.
        """
        t, f1, f2 = (np.ma.asarray(v, dtype=float) for v in (t, f1, f2))
        cls._validate_shapes(t_dir, cls._f_dir_from_t(t_dir), t, f1, f2)
        if where is not None:
            where = np.asarray(where, dtype=bool)
            if where.size != t.size:
                msg = "where size ({}) does not match {!r} size ({})".format(
                    where.size, t_dir, t.size)
                raise ValueError(msg)
        # matplotlib's mask of the points masked in any curve, which fails
        # for curves that do not broadcast together.
        masked = functools.reduce(np.logical_or, map(np.ma.getmaskarray, [t, f1, f2]))
        curves = np.broadcast_arrays(*(np.atleast_1d(np.ma.filled(v, np.nan))
                                       for v in (t, f1, f2)))
        if where is None:
            return curves, None
        # matplotlib takes the points it fills, which fails for flags of
        # another shape.
        curves[0][where & ~masked]
        return curves, where.ravel().tolist()

    def set_data(self, t, f1, f2, *, where=None):
        """
        Set new values for the two bounding curves.

        Parameters
        ----------
        t : array-like
            The ``self.t_direction`` coordinates of the nodes defining the curves.

        f1 : array-like or float
            The other coordinates of the nodes defining the first curve.

        f2 : array-like or float
            The other coordinates of the nodes defining the second curve.

        where : array-like of bool, optional
            Define *where* to exclude some {dir} regions from being filled.
            The filled regions are defined by the coordinates ``t[where]``.
            More precisely, fill between ``t[i]`` and ``t[i+1]`` if
            ``where[i] and where[i+1]``.  Note that this definition implies
            that an isolated *True* value between two *False* values in *where*
            will not result in filling.  Both sides of the *True* position
            remain unfilled due to the adjacent *False* values.

        See Also
        --------
        .PolyCollection.set_verts, .Line2D.set_data
        """
        # matplotlib's Axes converts them with its units first
        # (``_fill_between_process_units``).
        t, f1, f2 = map(np.ma.masked_invalid, self.axes._process_unit_info(
            [(self.t_direction, t), (self._f_direction, f1), (self._f_direction, f2)], {}))
        curves, mask = self._checked_data(self.t_direction, t, f1, f2, where)
        _collections.set_fill_between_data(self._native, curves, mask)


class RegularPolyCollection(_CollectionWithSizes):
    """A collection of n-sided regular polygons."""

    _path_generator = mpath.Path.unit_regular_polygon

    def __init__(self, numsides, *, rotation=0, sizes=(1,), **kwargs):
        """
        Parameters
        ----------
        numsides : int
            The number of sides of the polygon.
        rotation : float
            The rotation of the polygon in radians.
        sizes : tuple of float
            The area of the circle circumscribing the polygon in points^2.
        **kwargs
            Forwarded to `.Collection`.

        Examples
        --------
        See :doc:`/gallery/event_handling/lasso_demo` for a complete example::

            offsets = np.random.rand(20, 2)
            facecolors = [cm.jet(x) for x in np.random.rand(20)]

            collection = RegularPolyCollection(
                numsides=5, # a pentagon
                rotation=0, sizes=(50,),
                facecolors=facecolors,
                edgecolors=("black",),
                linewidths=(1,),
                offsets=offsets,
                offset_transform=ax.transData,
                )
        """
        usable = isinstance(rotation, Real)
        # Made of triangles, which matplotlib's path of the items replaces
        # once its sizes are set (``_path_generator``).
        self._native = _collections.new_regular_poly_collection(
            3, float(rotation) if usable else 0., np.zeros(0))
        super().__init__(**kwargs)
        self.set_sizes(sizes)
        self._numsides = numsides
        _collections.set_paths(self._native, [self._path_generator(numsides)])
        _values.remember(self, "rotation", rotation)
        if not usable:
            # matplotlib keeps it, and fails to turn the polygons by it
            # when it draws them (``rotate(-self._rotation)``).
            _unusable.keep(self, "rotation", rotation, operator.neg, measured=False)

    def get_numsides(self):
        return self._numsides

    def get_rotation(self):
        return _unusable.kept(self, "rotation", _values.given(
            self, "rotation", _collections.get_rotation(self._native)))

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)


class StarPolygonCollection(RegularPolyCollection):
    """Draw a collection of regular stars with *numsides* points."""

    _path_generator = mpath.Path.unit_regular_star


class AsteriskPolygonCollection(RegularPolyCollection):
    """Draw a collection of regular asterisks with *numsides* points."""

    _path_generator = mpath.Path.unit_regular_asterisk


class LineCollection(Collection):
    r"""
    Represents a sequence of `.Line2D`\s that should be drawn together.

    This class extends `.Collection` to represent a sequence of
    `.Line2D`\s instead of just a sequence of `.Patch`\s.
    Just as in `.Collection`, each property of a *LineCollection* may be either
    a single value or a list of values. This list is then used cyclically for
    each element of the LineCollection, so the property of the ``i``\th element
    of the collection is::

      prop[i % len(prop)]

    The properties of each member of a *LineCollection* default to their values
    in :rc:`lines.*` instead of :rc:`patch.*`, and the property *colors* is
    added in place of *edgecolors*.
    """

    def set_alpha(self, alpha):
        # docstring inherited: matplotlib's override has none.
        super().set_alpha(alpha)

    _edge_default = True

    def __init__(self, segments,  # Can be None.
                 *,
                 zorder=2,        # Collection.zorder is 1
                 **kwargs
                 ):
        """
        Parameters
        ----------
        segments : list of (N, 2) array-like
            A sequence ``[line0, line1, ...]`` where each line is a (N, 2)-shape
            array-like containing points::

                line0 = [(x0, y0), (x1, y1), ...]

            Each line can contain a different number of points.
        linewidths : float or list of float, default: :rc:`lines.linewidth`
            The width of each line in points.
        colors : :mpltype:`color` or list of color, default: :rc:`lines.color`
            A sequence of RGBA tuples (e.g., arbitrary color strings, etc, not
            allowed).
        antialiaseds : bool or list of bool, default: :rc:`lines.antialiased`
            Whether to use antialiasing for each line.
        zorder : float, default: 2
            zorder of the lines once drawn.

        facecolors : :mpltype:`color` or list of :mpltype:`color`, default: 'none'
            When setting *facecolors*, each line is interpreted as a boundary
            for an area, implicitly closing the path from the last point to the
            first point. The enclosed area is filled with *facecolor*.
            In order to manually specify what should count as the "interior" of
            each line, please use `.PathCollection` instead, where the
            "interior" can be specified by appropriate usage of
            `~.path.Path.CLOSEPOLY`.

        **kwargs
            Forwarded to `.Collection`.
        """
        kwargs.setdefault('facecolors', 'none')
        self._native = _collections.new_line_collection(
            [] if segments is None else [_path_vertices(seg) for seg in segments])
        super().__init__(zorder=zorder, **kwargs)

    def set_segments(self, segments):
        if segments is None:
            return
        _collections.set_segments(self._native, [_path_vertices(seg) for seg in segments])
        # New paths, as matplotlib makes them.
        self.__dict__.pop("_kept_paths", None)

    set_verts = set_segments  # for compatibility with PolyCollection
    set_paths = set_segments

    def get_segments(self):
        """
        Returns
        -------
        list
            List of segments in the LineCollection. Each list item contains an
            array of vertices.
        """
        segments = []

        for path in self.get_paths():
            vertices = [
                vertex
                for vertex, _
                # Never simplify here, we want to get the data-space values
                # back and there in no way to know the "right" simplification
                # threshold so never try.
                in path.iter_segments(simplify=False)
            ]
            vertices = np.asarray(vertices)
            segments.append(vertices)

        return segments

    def _get_default_linewidth(self):
        return _rcParams()['lines.linewidth']

    def _get_default_edgecolor(self):
        return _rcParams()['lines.color']

    def _get_default_facecolor(self):
        return 'none'

    def get_antialiased(self):
        antialiased = self._get("antialiased")
        if antialiased is None:
            antialiased = [_rcParams()['lines.antialiased']]
        return np.array(antialiased, dtype=bool)

    def set_color(self, c):
        """
        Set the edgecolor(s) of the LineCollection.

        Parameters
        ----------
        c : :mpltype:`color` or list of :mpltype:`color`
            Single color (all lines have same color), or a
            sequence of RGBA tuples; if it is a sequence the lines will
            cycle through the sequence.
        """
        self.set_edgecolor(c)

    set_colors = set_color

    def set_gapcolor(self, gapcolor):
        """
        Set a color to fill the gaps in the dashed line style.

        .. note::

            Striped lines are created by drawing two interleaved dashed lines.
            There can be overlaps between those two, which may result in
            artifacts when using transparency.

            This functionality is experimental and may change.

        Parameters
        ----------
        gapcolor : :mpltype:`color` or list of :mpltype:`color` or None
            The color with which to fill the gaps. If None, the gaps are
            unfilled.
        """
        _collections.set_gapcolors(
            self._native, None if gapcolor is None else _rgba(gapcolor))
        self.stale = True

    def get_gapcolor(self):
        return _collections.get_gapcolors(self._native)

    def get_color(self):
        return self.get_edgecolor()

    get_colors = get_color  # for compatibility with old versions


class EventCollection(LineCollection):
    """
    A collection of locations along a single axis at which an "event" occurred.

    The events are given by a 1-dimensional array. They do not have an
    amplitude and are displayed as parallel lines.
    """

    _edge_default = True

    def __init__(self,
                 positions,  # Cannot be None.
                 orientation='horizontal',
                 *,
                 lineoffset=0,
                 linelength=1,
                 linewidth=None,
                 color=None,
                 linestyle='solid',
                 antialiased=None,
                 **kwargs
                 ):
        """
        Parameters
        ----------
        positions : 1D array-like
            Each value is an event.
        orientation : {'horizontal', 'vertical'}, default: 'horizontal'
            The sequence of events is plotted along this direction.
            The marker lines of the single events are along the orthogonal
            direction.
        lineoffset : float, default: 0
            The offset of the center of the markers from the origin, in the
            direction orthogonal to *orientation*.
        linelength : float, default: 1
            The total height of the marker (i.e. the marker stretches from
            ``lineoffset - linelength/2`` to ``lineoffset + linelength/2``).
        linewidth : float or list thereof, default: :rc:`lines.linewidth`
            The line width of the event lines, in points.
        color : :mpltype:`color` or list of :mpltype:`color`, default: :rc:`lines.color`
            The color of the event lines.
        linestyle : str or tuple or list thereof, default: 'solid'
            Valid strings are ['solid', 'dashed', 'dashdot', 'dotted',
            '-', '--', '-.', ':']. Dash tuples should be of the form::

                (offset, onoffseq),

            where *onoffseq* is an even length tuple of on and off ink
            in points.
        antialiased : bool or list thereof, default: :rc:`lines.antialiased`
            Whether to use antialiasing for drawing the lines.
        **kwargs
            Forwarded to `.LineCollection`.

        Examples
        --------
        .. plot:: gallery/lines_bars_and_markers/eventcollection_demo.py
        """
        # matplotlib makes the line collection with its properties first,
        # then sets the orientation and computes the segments of the
        # positions with the offset and length as given.
        _checked_linestyles(linestyle)
        if color is not None:
            mcolors.to_rgba_array(color)
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        positions = _event_positions(positions, lineoffset, linelength)
        self._native = _collections.new_event_collection(
            positions, orientation == 'vertical', float(lineoffset), float(linelength))
        _values.remember_all(self, {"lineoffset": lineoffset, "linelength": linelength},
                             ("lineoffset", "linelength"))
        Collection.__init__(self, linewidths=linewidth, linestyles=linestyle,
                            antialiaseds=antialiased, facecolors='none',
                            zorder=2, **kwargs)
        if color is not None:
            self.set_color(color)

    def get_positions(self):
        """
        Return an array containing the floating-point values of the positions.
        """
        pos = 0 if self.is_horizontal() else 1
        return [segment[0, pos] for segment in self.get_segments()]

    def set_positions(self, positions):
        """Set the positions of the events."""
        _collections.set_event_positions(self._native, _event_positions(
            positions, self.get_lineoffset(), self.get_linelength()))

    def add_positions(self, position):
        """Add one or more events at the specified positions."""
        if position is None or (hasattr(position, 'len') and
                                len(position) == 0):
            return
        positions = self.get_positions()
        positions = np.hstack([positions, np.asanyarray(position)])
        self.set_positions(positions)

    extend_positions = append_positions = add_positions

    def is_horizontal(self):
        """True if the eventcollection is horizontal, False if vertical."""
        return self.get_orientation() == 'horizontal'

    def get_orientation(self):
        """
        Return the orientation of the event line ('horizontal' or 'vertical').
        """
        return _collections.event_orientation(self._native)

    def switch_orientation(self):
        """
        Switch the orientation of the event line, either from vertical to
        horizontal or vice versus.
        """
        # matplotlib flips each segment, failing on one of no two vertices
        # (a position of NaN).
        for segment in self.get_segments():
            np.fliplr(segment)
        _collections.set_event_orientation(
            self._native, 'vertical' if self.is_horizontal() else 'horizontal')
        self.stale = True

    def set_orientation(self, orientation):
        """
        Set the orientation of the event line.

        Parameters
        ----------
        orientation : {'horizontal', 'vertical'}
        """
        is_horizontal = _api.getitem_checked(
            {"horizontal": True, "vertical": False}, orientation=orientation)
        if is_horizontal != self.is_horizontal():
            self.switch_orientation()

    def _place_lines(self, lineoffset, linelength):
        """Compute the ends of the event lines at *lineoffset* and of
        *linelength*, as matplotlib does on each segment: failing as it does
        on what is no number, or a segment of no two vertices."""
        pos = 1 if self.is_horizontal() else 0
        for segment in self.get_segments():
            segment[0, pos] = lineoffset + linelength / 2.
            segment[1, pos] = lineoffset - linelength / 2.

    def get_linelength(self):
        """Return the length of the lines used to mark each event."""
        return _values.given(self, "linelength",
                             _collections.event_lines(self._native)[1])

    def set_linelength(self, linelength):
        """Set the length of the lines used to mark each event."""
        if linelength == self.get_linelength():
            return
        self._place_lines(self.get_lineoffset(), linelength)
        _collections.set_event_lines(self._native, None, float(linelength))
        _values.remember(self, "linelength", linelength)

    def get_lineoffset(self):
        """Return the offset of the lines used to mark each event."""
        return _values.given(self, "lineoffset",
                             _collections.event_lines(self._native)[0])

    def set_lineoffset(self, lineoffset):
        """Set the offset of the lines used to mark each event."""
        if lineoffset == self.get_lineoffset():
            return
        self._place_lines(lineoffset, self.get_linelength())
        _collections.set_event_lines(self._native, float(lineoffset), None)
        _values.remember(self, "lineoffset", lineoffset)

    def get_linewidth(self):
        """Get the width of the lines used to mark each event."""
        return super().get_linewidth()[0]

    def get_linewidths(self):
        return super().get_linewidth()

    def get_color(self):
        """Return the color of the lines used to mark each event."""
        return self.get_colors()[0]


class CircleCollection(_CollectionWithSizes):
    """A collection of circles, drawn using splines."""

    def __init__(self, sizes, **kwargs):
        """
        Parameters
        ----------
        sizes : float or array-like
            The area of each circle in points^2.
        **kwargs
            Forwarded to `.Collection`.
        """
        self._native = _collections.new_circle_collection(np.zeros(0))
        super().__init__(**kwargs)
        self.set_sizes(sizes)


#: The units `EllipseCollection` sizes its ellipses in.
_ELLIPSE_UNITS = ('points', 'inches', 'dots', 'width', 'height', 'x', 'y', 'xy')


def _refuse_ellipse_units(units):
    """Raise what matplotlib's `EllipseCollection` raises for *units*,
    none of `_ELLIPSE_UNITS`, when it draws."""
    raise ValueError(f'Unrecognized units: {units!r}')


class EllipseCollection(Collection):
    """A collection of ellipses, drawn using splines."""

    def __init__(self, widths, heights, angles, *, units='points', **kwargs):
        """
        Parameters
        ----------
        widths : array-like
            The lengths of the first axes (e.g., major axis lengths).
        heights : array-like
            The lengths of second axes.
        angles : array-like
            The angles of the first axes, degrees CCW from the x-axis.
        units : {'points', 'inches', 'dots', 'width', 'height', 'x', 'y', 'xy'}
            The units in which majors and minors are given; 'width' and
            'height' refer to the dimensions of the axes, while 'x' and 'y'
            refer to the *offsets* data units. 'xy' differs from all others in
            that the angle as plotted varies with the aspect ratio, and equals
            the specified angle only when the aspect ratio is unity.  Hence
            it behaves the same as the `~.patches.Ellipse` with
            ``axes.transData`` as its transform.
        **kwargs
            Forwarded to `Collection`.
        """
        usable = units in _ELLIPSE_UNITS
        self._native = _collections.new_ellipse_collection(
            (np.zeros(0),) * 3, units if usable else 'points')
        super().__init__(**kwargs)
        self.set_widths(widths)
        self.set_heights(heights)
        self.set_angles(angles)
        self._units = units
        # The unit circle itself, as matplotlib's collection keeps it,
        # until other paths are set.
        self._unit_paths = [Path.unit_circle()]
        if not usable:
            # matplotlib keeps them, and refuses them when it draws.
            _unusable.keep(self, "units", units, _refuse_ellipse_units, measured=False)

    @property
    def _widths(self):
        """The half widths of the ellipses."""
        return 0.5 * self.get_widths()

    @property
    def _heights(self):
        """The half heights of the ellipses."""
        return 0.5 * self.get_heights()

    @property
    def _angles(self):
        """The angles of the ellipses, in radians."""
        return np.deg2rad(self.get_angles())

    def set_widths(self, widths):
        """Set the lengths of the first axes (e.g., major axis)."""
        # matplotlib halves them, failing on what is no number.
        0.5 * np.asarray(widths).ravel()
        _collections.set_ellipse_sizes(
            self._native, (np.ravel(widths).astype(float), None, None))

    def set_heights(self, heights):
        """Set the lengths of second axes (e.g., minor axes)."""
        0.5 * np.asarray(heights).ravel()
        _collections.set_ellipse_sizes(
            self._native, (None, np.ravel(heights).astype(float), None))

    def set_angles(self, angles):
        """Set the angles of the first axes, degrees CCW from the x-axis."""
        # matplotlib keeps them in radians, failing on what is no number.
        np.deg2rad(angles).ravel()
        _collections.set_ellipse_sizes(
            self._native, (None, None, np.ravel(angles).astype(float)))

    def get_widths(self):
        """Get the lengths of the first axes (e.g., major axis)."""
        return _collections.ellipse_sizes(self._native)[0]

    def get_heights(self):
        """Set the lengths of second axes (e.g., minor axes)."""
        return _collections.ellipse_sizes(self._native)[1]

    def get_angles(self):
        """Get the angles of the first axes, degrees CCW from the x-axis."""
        return _collections.ellipse_sizes(self._native)[2]

    def _drawn_transforms(self):
        # None before the first draw, then the ellipses' as the draws
        # work them out from the sizes in their units (matplotlib's
        # ``_set_transforms``).
        if self._figure is None or not _artist.draw_count(self._native):
            return None
        ax, fig = self.axes, self.get_figure(root=False)
        sc = _api.getitem_checked({
            'xy': lambda: 1,
            'x': lambda: ax.bbox.width / ax.viewLim.width,
            'y': lambda: ax.bbox.height / ax.viewLim.height,
            'inches': lambda: fig.dpi,
            'points': lambda: fig.dpi / 72.0,
            'width': lambda: ax.bbox.width,
            'height': lambda: ax.bbox.height,
            'dots': lambda: 1.0,
        }, units=self._units)()
        transforms = np.zeros((len(self._widths), 3, 3))
        try:
            widths, heights = self._widths * sc, self._heights * sc
            sin_angle, cos_angle = np.sin(self._angles), np.cos(self._angles)
            transforms[:, 0, 0] = widths * cos_angle
            transforms[:, 0, 1] = heights * -sin_angle
            transforms[:, 1, 0] = widths * sin_angle
            transforms[:, 1, 1] = heights * cos_angle
            transforms[:, 2, 2] = 1.0
        except ValueError:
            # Sizes of different lengths fail matplotlib's draws there,
            # which leave the transforms as far as they got.
            pass
        return transforms

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)


class PatchCollection(Collection):
    """
    A generic collection of patches.

    PatchCollection draws faster than a large number of equivalent individual
    Patches. It also makes it easier to assign a colormap to a heterogeneous
    collection of patches.
    """

    def __init__(self, patches, *, match_original=False, **kwargs):
        """
        Parameters
        ----------
        patches : list of `.Patch`
            A sequence of Patch objects.  This list may include
            a heterogeneous assortment of different patch types.

        match_original : bool, default: False
            If True, use the colors and linewidths of the original
            patches.  If False, new colors may be assigned by
            providing the standard collection arguments, facecolor,
            edgecolor, linewidths, norm or cmap.

        **kwargs
            All other parameters are forwarded to `.Collection`.

            If any of *edgecolors*, *facecolors*, *linewidths*, *antialiaseds*
            are None, they default to their `.rcParams` patch setting, in
            sequence form.

        Notes
        -----
        The use of `~matplotlib.cm.ScalarMappable` functionality is optional.
        If the `~matplotlib.cm.ScalarMappable` matrix ``_A`` has been set (via
        a call to `~.ScalarMappable.set_array`), at draw time a call to scalar
        mappable will be made to set the face colors.
        """
        patches = list(patches)
        held = _collection_arguments.held_patches(patches, match_original)
        self._native = _collections.new_patch_collection(
            [] if held is None else held, bool(match_original))
        if match_original:
            # The patches' own colours, kept; the colorizer is made as
            # matplotlib's collections make it, for an array set later.
            Artist.__init__(self)
            self._attach_colorizer(self._get_colorizer(
                kwargs.pop("cmap", None), kwargs.pop("norm", None),
                kwargs.pop("colorizer", None)))
            self._internal_update(kwargs)
        else:
            super().__init__(**kwargs)
        if held is None:
            # As matplotlib's collection sets its paths, failing on what
            # is no patch.
            self.set_paths(patches)

    def set_paths(self, patches):
        _collections.set_paths(self._native, [
            p.get_transform().transform_path(p.get_path()) for p in patches])
        # New paths, as matplotlib makes them.
        self.__dict__.pop("_kept_paths", None)


class TriMesh(Collection):
    """
    Class for the efficient drawing of a triangular mesh using Gouraud shading.

    A triangular mesh is a `~matplotlib.tri.Triangulation` object.
    """
    def __init__(self, triangulation, **kwargs):
        # matplotlib reads the points of the triangulation to bound it.
        triangulation.x  # noqa: B018
        self._native = _collections.new_tri_mesh(triangulation._native)
        super().__init__(**kwargs)
        self._hold_triangulation(triangulation)

    def _hold_triangulation(self, triangulation):
        """Keep *triangulation* and its bounds, as matplotlib's constructor
        does (also for the mesh `tripcolor` makes)."""
        self._triangulation = triangulation
        self._shading = 'gouraud'

        self._bbox = mtransforms.Bbox.unit()

        # Unfortunately this requires a copy, unless Triangulation
        # was rewritten.
        xy = np.hstack((triangulation.x.reshape(-1, 1),
                        triangulation.y.reshape(-1, 1)))
        self._bbox.update_from_data_xy(xy)

    def get_paths(self):
        return super().get_paths()

    def set_paths(self):
        super().set_paths(self.convert_mesh_to_paths(self._triangulation))

    @staticmethod
    def convert_mesh_to_paths(tri):
        """
        Convert a given mesh into a sequence of `.Path` objects.

        This function is primarily of use to implementers of backends that do
        not directly support meshes.
        """
        native = getattr(tri, "_native", None)
        if native is None:
            # Not one of the package's triangulations: matplotlib's
            # conversion, through what the object has.
            triangles = tri.get_masked_triangles()
            verts = np.stack((tri.x[triangles], tri.y[triangles]), axis=-1)
            return [Path(x) for x in verts]
        return [Path(vertices, codes)
                for vertices, codes in _tri.triangle_paths(native)]

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)


class _MeshData:
    """The vertex grid of a quadrilateral mesh (`QuadMesh`, `PolyQuadMesh`)."""

    def __init__(self, coordinates):
        _api.check_shape((None, None, 2), coordinates=coordinates)
        # The vertices read back as matplotlib keeps them (`get_coordinates`).
        self._coordinates_dtype = np.asarray(coordinates).dtype

    @staticmethod
    def _native_coordinates(coordinates):
        """*coordinates* as pltrs (Rust) takes them: floats, NaN where
        masked."""
        return np.ma.filled(np.ma.asarray(coordinates, dtype=float), np.nan)

    def get_edgecolor(self):
        # docstring inherited
        # Note that we want to return an array of shape (N*M, 4)
        # a flattened RGBA collection
        return super().get_edgecolor().reshape(-1, 4)

    def get_facecolor(self):
        # docstring inherited
        # Note that we want to return an array of shape (N*M, 4)
        # a flattened RGBA collection
        return super().get_facecolor().reshape(-1, 4)
    @property
    def _coordinates(self):
        """The vertices of the mesh: matplotlib's attribute behind
        `get_coordinates`, which programs read (cartopy's
        ``pcolormesh``)."""
        return self.get_coordinates()

    def get_coordinates(self):
        """
        Return the vertices of the mesh as an (M+1, N+1, 2) array.

        M, N are the number of quadrilaterals in the rows / columns of the
        mesh, corresponding to (M+1, N+1) vertices.
        The last dimension specifies the components (x, y).
        """
        coordinates = _collections.get_coordinates(self._native)
        # As matplotlib keeps them: integers for a mesh of the default grid
        # (masked vertices, NaN here, stay floats), masked where the mesh
        # was made of the coordinates given (``pcolormesh(X, Y, C)``, whose
        # ``_pcolorargs`` masks them).
        dtype = self.__dict__.get("_coordinates_dtype")
        if dtype is not None and np.isfinite(coordinates).all():
            coordinates = coordinates.astype(dtype, copy=False)
        if self.__dict__.get("_coordinates_masked"):
            return np.ma.masked_invalid(coordinates)
        return coordinates

    @staticmethod
    def _convert_mesh_to_paths(coordinates):
        """
        Convert a given mesh into a sequence of `.Path` objects.

        This function is primarily of use to implementers of backends that do
        not directly support quadmeshes (matplotlib's, as it is).
        """
        if isinstance(coordinates, np.ma.MaskedArray):
            c = coordinates.data
        else:
            c = coordinates
        points = np.concatenate([
            c[:-1, :-1],
            c[:-1, 1:],
            c[1:, 1:],
            c[1:, :-1],
            c[:-1, :-1]
        ], axis=2).reshape((-1, 5, 2))
        return [Path(x) for x in points]

    def _convert_mesh_to_triangles(self, coordinates):
        """
        Convert a given mesh into a sequence of triangles, each point
        with its own color.  The result can be used to construct a call to
        `~.RendererBase.draw_gouraud_triangles` (matplotlib's, as it is).
        """
        if isinstance(coordinates, np.ma.MaskedArray):
            p = coordinates.data
        else:
            p = coordinates

        p_a = p[:-1, :-1]
        p_b = p[:-1, 1:]
        p_c = p[1:, 1:]
        p_d = p[1:, :-1]
        p_center = (p_a + p_b + p_c + p_d) / 4.0
        triangles = np.concatenate([
            p_a, p_b, p_center,
            p_b, p_c, p_center,
            p_c, p_d, p_center,
            p_d, p_a, p_center,
        ], axis=2).reshape((-1, 3, 2))

        c = self.get_facecolor().reshape((*coordinates.shape[:2], 4))
        z = self.get_array()
        mask = z.mask if np.ma.is_masked(z) else None
        if mask is not None:
            c[mask, 3] = np.nan
        c_a = c[:-1, :-1]
        c_b = c[:-1, 1:]
        c_c = c[1:, 1:]
        c_d = c[1:, :-1]
        c_center = (c_a + c_b + c_c + c_d) / 4.0
        colors = np.concatenate([
            c_a, c_b, c_center,
            c_b, c_c, c_center,
            c_c, c_d, c_center,
            c_d, c_a, c_center,
        ], axis=2).reshape((-1, 3, 4))
        tmask = np.isnan(colors[..., 2, 3])
        return triangles[~tmask], colors[~tmask]

    @property
    def _shading(self):
        """How the cells are coloured: ``'flat'`` or ``'gouraud'``."""
        return _collections.get_mesh_shading(self._native)

    def _cells(self):
        """The ``(rows, columns)`` of values the mesh maps."""
        height, width = _collections.get_mesh_shape(self._native)
        if self._shading == 'flat':
            return height - 1, width - 1
        return height, width

    def _native_array(self):
        # One value per cell, as the grid of cells (the plotting functions
        # hand matplotlib's meshes their values as 2D arrays).
        array = super()._native_array()
        if array is None or array.ndim != 1:
            return array
        rows, columns = self._cells()
        return array.reshape(rows, columns) if array.size == rows * columns else array

    def set_array(self, A):
        """
        Set the data values.

        Parameters
        ----------
        A : array-like
            The mesh data. Supported array shapes are:

            - (M, N) or (M*N,): a mesh with scalar data. The values are mapped
              to colors using normalization and a colormap. See parameters
              *norm*, *cmap*, *vmin*, *vmax*.
            - (M, N, 3): an image with RGB values (0-1 float or 0-255 int).
            - (M, N, 4): an image with RGBA values (0-1 float or 0-255 int),
              i.e. including transparency.

            If the values are provided as a 2D grid, the shape must match the
            coordinates grid. If the values are 1D, they are reshaped to 2D.
            M, N follow from the coordinates grid, where the coordinates grid
            shape is (M, N) for 'gouraud' *shading* and (M+1, N+1) for 'flat'
            shading.
        """
        height, width = self.get_coordinates().shape[0:-1]
        h, w = self._cells()
        ok_shapes = [(h, w, 3), (h, w, 4), (h, w), (h * w,)]
        if A is not None:
            shape = np.shape(A)
            if shape not in ok_shapes:
                raise ValueError(
                    f"For X ({width}) and Y ({height}) with {self._shading} "
                    f"shading, A should have shape "
                    f"{' or '.join(map(str, ok_shapes))}, not {A.shape}")
            if len(shape) == 3:
                # RGB(A) values are the colours of the cells, which
                # ``to_rgba`` passes through.
                mcolorizer.ColorizingArtist.set_array(self, A)
                colors = self.to_rgba(self._A).reshape(-1, 4)
                if isinstance(self, PolyCollection):
                    colors = colors[self._draw_cells_of(self._A)]
                _collections.set_array(self._native, None)
                _collections.set_facecolors(self._native, colors)
                return
        super().set_array(A)

    def _array_set(self):
        # docstring inherited
        # RGB(A) values colour the cells themselves (`set_array`).
        if self._A is None or self._A.ndim != 3:
            super()._array_set()

    def _draw_cells_of(self, colors):
        """
        Have the polygons of the mesh be those of its cells whose RGB(A)
        *colors* (an ``(M, N, 3 or 4)`` array) and corners are all given,
        as matplotlib's `PolyQuadMesh` draws them; return which cells
        those are, in order.
        """
        masked = np.ma.getmaskarray(colors).any(axis=-1).ravel()
        # The polygons are those of the cells with a value (see
        # `CollectionArtist::mask_polygon_cells`).
        _collections.set_array(self._native, np.where(masked, np.nan, 0.0))
        corners = np.isfinite(self.get_coordinates()).all(axis=-1)
        given = (corners[:-1, :-1] & corners[1:, 1:]
                 & corners[:-1, 1:] & corners[1:, :-1])
        return given.ravel() & ~masked


class QuadMesh(_MeshData, Collection):
    r"""
    Class for the efficient drawing of a quadrilateral mesh.

    A quadrilateral mesh is a grid of M by N adjacent quadrilaterals that are
    defined via a (M+1, N+1) grid of vertices. The quadrilateral (m, n) is
    defined by the vertices ::

               (m+1, n) ----------- (m+1, n+1)
                  /                   /
                 /                 /
                /               /
            (m, n) -------- (m, n+1)

    The mesh need not be regular and the polygons need not be convex.

    Parameters
    ----------
    coordinates : (M+1, N+1, 2) array-like
        The vertices. ``coordinates[m, n]`` specifies the (x, y) coordinates
        of vertex (m, n).

    antialiased : bool, default: True

    shading : {'flat', 'gouraud'}, default: 'flat'

    Notes
    -----
    Unlike other `.Collection`\s, the default *pickradius* of `.QuadMesh` is 0,
    i.e. `~.Artist.contains` checks whether the test point is within any of the
    mesh quadrilaterals.

    """

    def __init__(self, coordinates, *, antialiased=True, shading='flat',
                 **kwargs):
        kwargs.setdefault("pickradius", 0)
        super().__init__(coordinates=coordinates)
        self._native = _collections.new_quad_mesh(
            self._native_coordinates(coordinates), shading == 'gouraud',
            bool(antialiased))
        Collection.__init__(self, **kwargs)
        self.set_mouseover(False)

    def _keep_given_array(self, given):
        """
        Keep the values *given* to the plotting function as the mesh's
        array, as matplotlib's `set_array` keeps them: masked where invalid,
        with the values a mask hides and their dtype (pltrs (Rust) maps
        floats, NaN where masked). A plain float array is already that.
        """
        if type(given) is np.ndarray and given.dtype == np.float64:
            return
        kept = cbook.safe_masked_invalid(given, copy=True)
        if self._A is not None and kept.shape == self._A.shape:
            self._A = kept

    def get_paths(self):
        return super().get_paths()

    def set_paths(self):
        # Make the paths of the mesh's quadrilaterals from its coordinates
        # anew (pltrs (Rust) makes them from the coordinates whenever they are
        # read): `get_paths` hands out new ones.
        self.__dict__.pop("_kept_paths", None)
        self.stale = True

    def get_datalim(self, transData):
        bbox = mtransforms.Bbox.unit()
        bbox.update_from_data_xy(self.get_coordinates().reshape(-1, 2))
        return (self.get_transform() - transData).transform_bbox(bbox)

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    def get_cursor_data(self, event):
        contained, info = self.contains(event)
        if contained and self.get_array() is not None:
            return self.get_array().ravel()[info["ind"]]
        return None

    def get_pickradius(self):
        # docstring inherited
        # matplotlib's meshes are picked at their cells (``pickradius=0``).
        return _values.given(self, "pickradius",
                             _collections.get_pickradius(self._native), default=0)

    @property
    def _antialiased(self):
        """Whether the mesh is drawn antialiased (matplotlib's attribute,
        what the mesh was made with; see `get_antialiased`)."""
        antialiased = self._get("antialiased")
        return bool(antialiased[0]) if antialiased else _rcParams()['patch.antialiased']

    @_antialiased.setter
    def _antialiased(self, antialiased):
        _collections.set_antialiaseds(self._native, [bool(antialiased)])

    def get_antialiased(self):
        # The antialiasing of the collection: :rc:`patch.antialiased` until
        # set. matplotlib's mesh draws with the antialiasing it was made
        # with (``pcolormesh(antialiased=...)``, `_antialiased`), which this
        # does not read.
        return np.array(self.__dict__.get(
            "_antialiaseds", [_rcParams()['patch.antialiased']]), dtype=bool)

    get_antialiased.__doc__ = Collection.get_antialiased.__doc__

    def set_antialiased(self, aa):
        # Sets the antialiasing `get_antialiased` reads.
        if aa is None:
            aa = _rcParams()['patch.antialiased']
        self._antialiaseds = [bool(a) for a in np.atleast_1d(aa)]

    set_antialiased.__doc__ = Collection.set_antialiased.__doc__


class PolyQuadMesh(_MeshData, PolyCollection):
    """
    Class for drawing a quadrilateral mesh as individual Polygons.

    A quadrilateral mesh is a grid of M by N adjacent quadrilaterals that are
    defined via a (M+1, N+1) grid of vertices. The quadrilateral (m, n) is
    defined by the vertices ::

               (m+1, n) ----------- (m+1, n+1)
                  /                   /
                 /                 /
                /               /
            (m, n) -------- (m, n+1)

    The mesh need not be regular and the polygons need not be convex.

    Parameters
    ----------
    coordinates : (M+1, N+1, 2) array-like
        The vertices. ``coordinates[m, n]`` specifies the (x, y) coordinates
        of vertex (m, n).

    Notes
    -----
    Unlike `.QuadMesh`, this class will draw each cell as an individual Polygon.
    This is significantly slower, but allows for more flexibility when wanting
    to add additional properties to the cells, such as hatching.

    Another difference from `.QuadMesh` is that if any of the vertices or data
    of a cell are masked, that Polygon will **not** be drawn and it won't be in
    the list of paths returned.
    """

    def __init__(self, coordinates, **kwargs):
        super().__init__(coordinates=coordinates)
        self._native = _collections.new_poly_quad_mesh(
            self._native_coordinates(coordinates))
        Collection.__init__(self, **kwargs)

    def _keep_given_array(self, given):
        """
        Keep the values *given* to `~.Axes.pcolor` as the mesh's array, as
        matplotlib's mesh is made with them: one per cell, masked where
        invalid, in their dtype (pltrs (Rust) holds the values of the cells
        it draws, those not masked).
        """
        kept = cbook.safe_masked_invalid(given, copy=True)
        if kept.shape == self._cells():
            self._A = kept

    # pltrs (Rust) holds the polygons, colours and values of the cells it
    # draws, those not masked, which these give.

    def get_edgecolor(self):
        # docstring inherited
        return super().get_edgecolor()

    def get_facecolor(self):
        # docstring inherited
        return super().get_facecolor()

    def set_array(self, A):
        # docstring inherited
        super().set_array(A)


# What matplotlib's module also has from its imports, which programs
# import from it.
import functools  # noqa: E402, F401
import itertools  # noqa: E402, F401
from numbers import Number  # noqa: E402, F401
from . import artist, hatch as mhatch, transforms  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
