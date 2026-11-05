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

import math
import warnings

import numpy as np

from . import (_api, _live_transforms, _values, cbook, colorizer as mcolorizer,
               colors as mcolors, lines as mlines, path as mpath,
               transforms as mtransforms)
from ._enums import CapStyle, JoinStyle
from ._native import artist as _artist, collections as _collections
from ._native import tri as _tri
from .artist import Artist
from .path import Path


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


@_api.define_aliases({
    "antialiased": ["antialiaseds", "aa"],
    "edgecolor": ["edgecolors", "ec"],
    "facecolor": ["facecolors", "fc"],
    "linestyle": ["linestyles", "dashes", "ls"],
    "linewidth": ["linewidths", "lw"],
    "offset_transform": ["transOffset"],
})
class Collection(mcolorizer.ColorizingArtist):
    """
    Base class for Collections. Must be subclassed to be usable.

    A Collection represents a sequence of `.Patch`\\es that can be drawn
    more efficiently together than individually.

    pltrs: the colorizer is Python's, as matplotlib's is; pltrs (Rust)
    maps the values with a copy of its colormap and norm, handed over
    whenever it changes (`changed`).
    """

    # Whether to draw an edge by default.  Set on a
    # subclass-by-subclass basis.
    _edge_default = False

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
        Style the collection a subclass holds in ``self._native``: the
        collection is outside any figure until an axes or a figure adds it
        (`~.Axes.add_collection`, `.Figure.add_artist`).
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
            self.set_offsets(offsets)
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
        # The array is a fresh copy: masking it in place spares another.
        return None if array is None else np.ma.masked_invalid(array, copy=False)

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

    def changed(self):
        """
        Call this whenever the mappable is changed to notify all the
        callbackSM listeners to the 'changed' signal.
        """
        if self._maps_variates():
            self._map_variates()
        else:
            _collections.set_mapping(self._native, self.cmap, self._colorizer.norm,
                                     self._handing_norm())
        super().changed()

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
        colors that were directly set, as appropriate (drawing does it).
        """
        if self._A is None:
            _collections.update_scalarmappable(self._native)
            return
        if self._maps_variates():
            self._map_variates()
        if self._A.ndim > 1 and not isinstance(self, (QuadMesh, PolyQuadMesh)):
            raise ValueError('Collections can only map rank 1 arrays')
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
        _collections.set_offset_transform(
            self._native, _live_transforms.to_native(offset_transform))
        self._offset_transform = offset_transform

    def _get(self, name):
        """The native property *name*."""
        return _collections.get(self._native, name)

    #: The kind of sequence the paths were given in (matplotlib keeps them
    #: as given: ``scatter`` gives its marker as a tuple).
    _paths_container = list

    def get_paths(self):
        """Return the paths of the items: the same objects while they
        stay, as matplotlib's collections keep theirs."""
        native = _collections.get_paths(self._native)
        kept = self.__dict__.get("_kept_paths")
        if kept is not None and _same_paths(kept[0], native):
            return kept[1]
        paths = [Path(vertices, codes) for vertices, codes in native]
        if _collections.has_single_path(self._native):
            paths = paths[0]
        else:
            paths = self._paths_container(paths)
        self._kept_paths = (native, paths)
        return paths

    _paths = property(get_paths, doc="The paths of the items.")

    def set_paths(self, paths):
        """
        Set the paths of the items.

        A single `.Path` is kept as it is given, as matplotlib keeps it:
        drawing the collection then raises matplotlib's `TypeError`.
        """
        if isinstance(paths, Path):
            _collections.set_single_path(self._native, paths)
        else:
            self._paths_container = tuple if isinstance(paths, tuple) else list
            _collections.set_paths(self._native, list(paths))
        self.__dict__.pop("_kept_paths", None)

    def get_transforms(self):
        """
        Return the transforms of the items, as an ``(N, 3, 3)`` array
        suitable for the *all_transforms* argument of
        `~matplotlib.backend_bases.RendererBase.draw_path_collection`.

        The items are scaled to their areas at the resolution of the
        collection's figure once it was drawn since its sizes were set, as
        matplotlib's ``draw`` scales them, else at the resolution they were
        set at (72 dpi by default).
        """
        dpi, draws = self.__dict__.get("_sizes_scaled", (72.0, 0))
        figure = self.get_figure(root=True)
        if figure is not None and _artist.draw_count(self._native) > draws:
            dpi = float(figure.dpi)
        return _collections.get_transforms(self._native, dpi)

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
        """Return the collection's box in display space as matplotlib does:
        its data limits in display coordinates (`get_datalim` with the
        identity), null for items placed in other coordinates (a scatter's
        offsets in data)."""
        return self.get_datalim(mtransforms.IdentityTransform())

    def get_datalim(self, transData):
        """
        Return the data limits of the collection in the coordinates of
        *transData*: its shapes at their offsets where its transform
        contains *transData*, else its offsets where those are in
        coordinates *transData* reaches; a null box otherwise.
        """
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
            return mpath.get_path_collection_extents(
                transform.get_affine() - transData, paths,
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
        """Return the pick radius used for containment tests."""
        return _values.given(self, "pickradius",
                             _collections.get_pickradius(self._native))

    def set_pickradius(self, pickradius):
        """
        Set the pick radius used for containment tests, in points.

        Parameters
        ----------
        pickradius : float
        """
        if not isinstance(pickradius, (int, float)):
            raise ValueError(
                f"pickradius must be a real-valued number, not {pickradius!r}")
        _collections.set_pickradius(self._native, float(pickradius))
        _values.remember(self, "pickradius", pickradius)

    pickradius = property(get_pickradius, set_pickradius)

    def _contains_details(self, ind):
        """The indices of the items hit."""
        return dict(ind=ind)

    def get_urls(self):
        """Return a list of URLs, one for each element of the collection."""
        urls = _collections.get_urls(self._native)
        # A collection starts with `[None]` (matplotlib's `set_urls(None)`).
        if not urls and "urls" not in self.__dict__.get("_given_values", {}):
            return [None]
        return urls

    def set_urls(self, urls):
        """
        Set the URLs, one per element (None: no link).

        Parameters
        ----------
        urls : list of str or None
        """
        urls = list(urls) if urls is not None else [None]
        _collections.set_urls(self._native, urls)
        _values.remember(self, "urls", urls)

    def get_path_effects(self):
        """Return the path effects (None until set, as matplotlib's
        collections start)."""
        return self.__dict__.get("_path_effects")

    def get_hatch(self):
        """Return the current hatching pattern."""
        return self._get("hatch")

    def set_hatch(self, hatch):
        """
        Set the hatching pattern (see `.hatch`).

        Parameters
        ----------
        hatch : {'/', '\\\\', '|', '-', '+', 'x', 'o', 'O', '.', '*'}
        """
        from . import hatch as mhatch
        mhatch._validate_hatch_pattern(hatch)
        _collections.set_hatch(self._native, hatch)

    def get_hatch_linewidth(self):
        """Return the hatch linewidth."""
        return _values.given(self, "hatch_linewidth", self._get("hatch_linewidth"))

    def set_hatch_linewidth(self, lw):
        """Set the hatch linewidth."""
        _collections.set_hatch_linewidth(self._native, float(lw))
        _values.remember(self, "hatch_linewidth", lw)

    def get_hatchcolor(self):
        """Return the hatch colors."""
        return self._get("hatchcolor")

    def set_hatchcolor(self, c):
        """
        Set the hatchcolor(s): colors, 'edge' or None (:rc:`hatch.color`).

        Parameters
        ----------
        c : :mpltype:`color` or list of :mpltype:`color` or 'edge'
        """
        if c is None:
            c = _rcParams()['hatch.color']
        _collections.set_hatchcolors(
            self._native, c if cbook._str_equal(c, 'edge') else _rgba(c))

    def get_capstyle(self):
        """Return the cap style for the collection (for all its elements)."""
        return self._get("capstyle")

    def set_capstyle(self, cs):
        """
        Set the `.CapStyle` for the collection (for all its elements).

        Parameters
        ----------
        cs : `.CapStyle` or {'butt', 'projecting', 'round'}
        """
        _collections.set_capstyle(self._native, CapStyle(cs).name)

    def get_joinstyle(self):
        """Return the join style for the collection (for all its elements)."""
        return self._get("joinstyle")

    def set_joinstyle(self, js):
        """
        Set the `.JoinStyle` for the collection (for all its elements).

        Parameters
        ----------
        js : `.JoinStyle` or {'miter', 'round', 'bevel'}
        """
        _collections.set_joinstyle(self._native, JoinStyle(js).name)

    def get_linewidth(self):
        """Return the line widths, broadcast against the line styles."""
        return self._lwls()[0]

    def set_linewidth(self, lw):
        """
        Set the linewidth(s) for the collection (None: the default).

        Parameters
        ----------
        lw : float or list of floats
        """
        if lw is None:
            lw = self._get_default_linewidth()
        lw = np.atleast_1d(lw)
        _collections.set_linewidths(self._native, [float(w) for w in lw])
        _values.remember(self, "linewidth", lw)

    def get_linestyle(self):
        """Return the dash patterns, scaled by the line widths."""
        return self._lwls()[1]

    def set_linestyle(self, ls):
        """
        Set the linestyle(s) for the collection: a line style or a
        sequence of them.

        Parameters
        ----------
        ls : {'-', '--', '-.', ':', '', ...} or (offset, on-off-seq) or list thereof
        """
        try:
            styles = _linestyle_list(ls)
        except ValueError as err:
            raise ValueError(f'Do not know how to convert {ls!r} to dashes') from err
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
        if _rcParams()['_internal.classic_mode']:
            return linewidths, dashes
        if len(dashes) != len(linewidths):
            gcd = math.gcd(len(dashes), len(linewidths))
            dashes, linewidths = (list(dashes) * (len(linewidths) // gcd),
                                  list(linewidths) * (len(dashes) // gcd))
        return linewidths, [mlines._scale_dashes(o, d, lw)
                            for (o, d), lw in zip(dashes, linewidths)]

    def _get_default_linewidth(self):
        return _rcParams()['patch.linewidth']

    def _get_default_facecolor(self):
        """The face colour of the collection when none is set."""
        return _rcParams()['patch.facecolor']

    def _get_default_edgecolor(self):
        """The edge colour of the collection when none is set."""
        return _rcParams()['patch.edgecolor']

    def get_antialiased(self):
        """Get the antialiasing state for rendering."""
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
        self._took_from(other, _collections.update_from(self._native, other._native))
        self._array_alpha = other.__dict__.get("_array_alpha")
        self._face_fill = other.__dict__.get("_face_fill")
        # update_from for scalarmappable
        self._A = other._A
        self.norm = other.norm
        self.cmap = other.cmap
        self.stale = True

    def get_facecolor(self):
        """Return the face colors, as drawn."""
        return self._get("facecolor")

    @property
    def _facecolors(self):
        """The RGBA face colours, as drawn (matplotlib's ``_facecolors``,
        which `get_facecolor` gives)."""
        return self.get_facecolor()

    def set_facecolor(self, c):
        """
        Set the facecolor(s) of the collection. *c* can be a color (all
        patches have same color), or a sequence of colors; None restores
        the default.

        Parameters
        ----------
        c : :mpltype:`color` or list of :mpltype:`color`
        """
        if isinstance(c, str) and c.lower() in ("none", "face"):
            c = c.lower()
        _collections.set_facecolors(self._native, None if c is None else _rgba(c))
        self._face_fill = not cbook._str_lower_equal(c, "none")

    def get_edgecolor(self):
        """Return the edge colors, as drawn."""
        return self._get("edgecolor")

    def set_edgecolor(self, c):
        """
        Set the edgecolor(s) of the collection: a color, a sequence of
        colors, ``'face'`` (the face colors) or None (the default rule).

        Parameters
        ----------
        c : :mpltype:`color` or list of :mpltype:`color` or 'face'
        """
        if isinstance(c, str) and c.lower() in ("none", "face"):
            c = c.lower()
        _collections.set_edgecolors(self._native, _edge_spec(c))

    def set_color(self, c):
        """
        Set both the edgecolor and the facecolor.

        Parameters
        ----------
        c : :mpltype:`color` or list of RGBA tuples
        """
        self.set_facecolor(c)
        self.set_edgecolor(c)

    def set_alpha(self, alpha):
        """
        Set the transparency of the collection: a float, an array of one
        per element, or None.

        Parameters
        ----------
        alpha : array-like or float or None
        """
        if np.iterable(alpha):
            alpha = np.asarray(alpha, dtype=float)
            if ((alpha < 0) | (alpha > 1)).any():
                raise ValueError("alpha must be between 0 and 1, inclusive")
            # The array itself comes back, as matplotlib keeps it, even
            # when it does not fit the colours.
            self._array_alpha = alpha
            _collections.set_alphas(self._native, alpha.ravel().tolist(),
                                    list(alpha.shape))
        else:
            if alpha is not None and not isinstance(alpha, (int, float)):
                raise TypeError(
                    f'alpha must be numeric or None, not {type(alpha)}')
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

    def get_alpha(self):
        """Return the transparency: a float, the array of one per element
        it was set to, or None."""
        array = self.__dict__.get("_array_alpha")
        return super().get_alpha() if array is None else array

    def set_array(self, A):
        """
        Set the value array from array-like *A*.

        Parameters
        ----------
        A : array-like or None
            The values that are mapped to colors.
        """
        super().set_array(A)
        if A is None or self._maps_variates():
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
        return _values.given(self, "sizes", np.zeros(0) if sizes is None else sizes)

    def set_sizes(self, sizes, dpi=72.0):
        """
        Set the sizes of each member of the collection.

        Parameters
        ----------
        sizes : `numpy.ndarray` or None
        """
        if sizes is None:
            sizes = np.array([])
        sizes = np.atleast_1d(sizes)
        _collections.set_sizes(self._native, np.asarray(sizes, float))
        _values.remember(self, "sizes", sizes)
        # The resolution the items are scaled at until the next draw.
        self._sizes_scaled = (float(dpi), _artist.draw_count(self._native))


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
            The factor by which to scale each drawn `~.path.Path`.
        """
        self._native = _collections.new_path_collection(list(paths))
        self._paths_container = tuple if isinstance(paths, tuple) else list
        super().__init__(**kwargs)
        self.set_sizes(sizes)

    def legend_elements(self, prop="colors", num="auto",
                        fmt=None, func=lambda x: x, **kwargs):
        """
        Create legend handles and labels for a PathCollection.

        Each legend handle is a `.Line2D` representing the Path that was drawn,
        and each label is a string that represents the Path (from matplotlib
        3.11.2, under the Matplotlib License).

        Parameters
        ----------
        prop : {"colors", "sizes"}, default: "colors"
            If "colors", the legend handles will show the different colors of
            the collection. If "sizes", the legend will show the different
            sizes.
        num : int, None, "auto" (default), array-like, or `~.ticker.Locator`
            Target number of elements to create.
        fmt : str, `~matplotlib.ticker.Formatter`, or None (default)
            The format or formatter to use for the labels.
        func : function, default: ``lambda x: x``
            Function to calculate the labels.
        **kwargs
            Allowed keyword arguments are *color* and *size*; any further
            parameters are passed onto the `.Line2D` instance.

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
    """A collection of polygons (as `~.Axes.fill_between` and `~.Axes.hexbin`
    make)."""

    def __init__(self, verts, sizes=None, *, closed=True, **kwargs):
        """
        Parameters
        ----------
        verts : list of array-like
            The sequence of polygons [*verts0*, *verts1*, ...] where each
            element *verts_i* defines the vertices of polygon *i* as a 2D
            array-like of shape (M, 2).
        sizes : array-like, default: None
            Squared scaling factors for the polygons.
        closed : bool, default: True
            Whether the polygon should be closed by adding a CLOSEPOLY
            connection at the end.
        """
        self._native = _collections.new_poly_collection(
            [np.asarray(v, float).reshape(-1, 2) for v in verts])
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
        if not closed:
            _collections.set_segments(
                self._native, [np.asarray(v, float).reshape(-1, 2) for v in verts])
            return
        _collections.set_verts(
            self._native, [np.asarray(v, float).reshape(-1, 2) for v in verts])

    set_paths = set_verts

    def set_verts_and_codes(self, verts, codes):
        """Initialize vertices with path codes."""
        if len(verts) != len(codes):
            raise ValueError("'codes' must be a 1D list or array "
                             "with the same length of 'verts'")
        Collection.set_paths(self, [Path(xy, cds) if len(xy) else Path(xy)
                                    for xy, cds in zip(verts, codes)])
        self.stale = True


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

        interpolate : bool, default: False
            Whether to extend the filled regions to where the curves cross,
            when *where* is used.

        step : {{'pre', 'post', 'mid'}}, optional
            Define *step* if the filling should be a step function,
            i.e. constant in between *t*.

        **kwargs
            Forwarded to `.PolyCollection`.
        """
        self._f_dir_from_t(t_direction)
        self.t_direction = t_direction
        # matplotlib reads the dimensions of the arrays as given, which
        # fails for anything else.
        for array in (t, f1, f2):
            getattr(array, "ndim")
        curves, mask = self._checked_data(t, f1, f2, where)
        _check_step(step, np.broadcast_arrays(*curves), mask)
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

    def _checked_data(self, t, f1, f2, where):
        """
        The curves as float arrays (masked points as NaN, which are not
        filled) and *where* as a list of flags, checked as matplotlib's
        ``_validate_shapes`` and ``_get_data_mask`` do.
        """
        t, f1, f2 = (np.ma.asarray(v, dtype=float) for v in (t, f1, f2))
        t_dir, f_dir = self.t_direction, self._f_direction
        names = (d + s for d, s in zip((t_dir, f_dir, f_dir), ("", "1", "2")))
        for name, array in zip(names, [t, f1, f2]):
            if array.ndim > 1:
                raise ValueError(f"{name!r} is not 1-dimensional")
            if t.size > 1 and array.size > 1 and t.size != array.size:
                msg = "{!r} has size {}, but {!r} has an unequal size of {}".format(
                    t_dir, t.size, name, array.size)
                raise ValueError(msg)
        if where is not None:
            where = np.asarray(where, dtype=bool)
            if where.size != t.size:
                msg = "where size ({}) does not match {!r} size ({})".format(
                    where.size, t_dir, t.size)
                raise ValueError(msg)
            where = where.ravel().tolist()
        curves = tuple(np.atleast_1d(np.ma.filled(v, np.nan)) for v in (t, f1, f2))
        return curves, where

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

        See Also
        --------
        .PolyCollection.set_verts, .Line2D.set_data
        """
        curves, mask = self._checked_data(t, f1, f2, where)
        _collections.set_fill_between_data(self._native, curves, mask)


class RegularPolyCollection(_CollectionWithSizes):
    """A collection of n-sided regular polygons."""

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
        """
        self._native = _collections.new_regular_poly_collection(
            int(numsides), float(rotation), np.atleast_1d(np.asarray(sizes, float)))
        super().__init__(**kwargs)

    def get_numsides(self):
        """Return the number of sides of the polygons."""
        return len(self.get_paths()[0].vertices) - 1

    def get_rotation(self):
        """Return the rotation of the polygons in radians."""
        return _collections.get_rotation(self._native)


class StarPolygonCollection(RegularPolyCollection):
    """Draw a collection of regular stars with *numsides* points."""


class AsteriskPolygonCollection(RegularPolyCollection):
    """Draw a collection of regular asterisks with *numsides* points."""


class LineCollection(Collection):
    r"""
    Represents a sequence of `.Line2D`\s that should be drawn together.
    """

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
        **kwargs
            Forwarded to `.Collection`.
        """
        kwargs.setdefault('facecolors', 'none')
        self._native = _collections.new_line_collection(
            [] if segments is None else
            [np.asarray(seg, float).reshape(-1, 2) for seg in segments])
        super().__init__(zorder=zorder, **kwargs)

    def set_segments(self, segments):
        """Set the segments: a sequence of (N, 2) arrays, one per line."""
        if segments is None:
            return
        _collections.set_segments(
            self._native, [np.asarray(seg, float).reshape(-1, 2) for seg in segments])

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
        return [path.vertices for path in self.get_paths()]

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
        """Return the gap colors, the alpha of the collection applied."""
        return _collections.get_gapcolors(self._native)

    def get_color(self):
        """Return the edge colors of the lines."""
        return self.get_edgecolor()

    get_colors = get_color  # for compatibility with old versions


class EventCollection(LineCollection):
    """
    A collection of locations along a single axis at which an "event"
    occurred.
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
        lineoffset : float, default: 0
            The offset of the center of the markers from the origin.
        linelength : float, default: 1
            The total height of the marker.
        """
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        positions = np.asarray(positions, dtype=float)
        if positions.ndim > 1:
            raise ValueError('positions must be one-dimensional')
        self._native = _collections.new_event_collection(
            np.atleast_1d(positions), orientation == 'vertical',
            float(lineoffset), float(linelength))
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
        return list(_collections.event_positions(self._native))

    def set_positions(self, positions):
        """Set the positions of the events."""
        if positions is None:
            positions = []
        if np.ndim(positions) != 1:
            raise ValueError('positions must be one-dimensional')
        _collections.set_event_positions(
            self._native, np.sort(np.asarray(positions, dtype=float)))

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
        self.set_orientation('vertical' if self.is_horizontal() else 'horizontal')

    def set_orientation(self, orientation):
        """
        Set the orientation of the event line.

        Parameters
        ----------
        orientation : {'horizontal', 'vertical'}
        """
        _api.getitem_checked({"horizontal": True, "vertical": False},
                             orientation=orientation)
        _collections.set_event_orientation(self._native, orientation)

    def get_linelength(self):
        """Return the length of the lines used to mark each event."""
        return _values.given(self, "linelength",
                             _collections.event_lines(self._native)[1])

    def set_linelength(self, linelength):
        """Set the length of the lines used to mark each event."""
        _collections.set_event_lines(self._native, None, float(linelength))
        _values.remember(self, "linelength", linelength)

    def get_lineoffset(self):
        """Return the offset of the lines used to mark each event."""
        return _values.given(self, "lineoffset",
                             _collections.event_lines(self._native)[0])

    def set_lineoffset(self, lineoffset):
        """Set the offset of the lines used to mark each event."""
        _collections.set_event_lines(self._native, float(lineoffset), None)
        _values.remember(self, "lineoffset", lineoffset)

    def get_linewidth(self):
        """Get the width of the lines used to mark each event."""
        return super().get_linewidth()[0]

    def get_linewidths(self):
        """Get the widths of the lines used to mark each event."""
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
        """
        self._native = _collections.new_circle_collection(
            np.atleast_1d(np.asarray(sizes, float)))
        super().__init__(**kwargs)


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
            The units in which majors and minors are given.
        """
        _api.check_in_list(['points', 'inches', 'dots', 'width', 'height',
                            'x', 'y', 'xy'], units=units)
        widths, heights, angles = np.broadcast_arrays(
            np.ravel(widths).astype(float), np.ravel(heights).astype(float),
            np.ravel(angles).astype(float))
        self._native = _collections.new_ellipse_collection((widths, heights, angles), units)
        super().__init__(**kwargs)

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
        _collections.set_ellipse_sizes(
            self._native, (np.ravel(widths).astype(float), None, None))

    def set_heights(self, heights):
        """Set the lengths of second axes (e.g., minor axes)."""
        _collections.set_ellipse_sizes(
            self._native, (None, np.ravel(heights).astype(float), None))

    def set_angles(self, angles):
        """Set the angles of the first axes, degrees CCW from the x-axis."""
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
        """
        patches = list(patches)
        self._native = _collections.new_patch_collection([p._native for p in patches],
                                              bool(match_original))
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

    def set_paths(self, patches):
        """Set the paths from the outlines of *patches*."""
        _collections.set_paths(self._native, [
            p.get_path().transformed(p.get_patch_transform()) for p in patches])


class TriMesh(Collection):
    """
    Class for the efficient drawing of a triangular mesh using Gouraud shading.

    A triangular mesh is a `~matplotlib.tri.Triangulation` object.
    """
    def __init__(self, triangulation, **kwargs):
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

    def set_paths(self):
        super().set_paths(self.convert_mesh_to_paths(self._triangulation))

    @staticmethod
    def convert_mesh_to_paths(tri):
        """
        Convert a given mesh into a sequence of `.Path` objects.

        This function is primarily of use to implementers of backends that do
        not directly support meshes.
        """
        return [Path(vertices, codes)
                for vertices, codes in _tri.triangle_paths(tri._native)]


class _MeshData:
    """The vertex grid of a quadrilateral mesh (`QuadMesh`, `PolyQuadMesh`)."""

    def get_coordinates(self):
        """
        Return the vertices of the mesh as an (M+1, N+1, 2) array.

        M, N are the number of quadrilaterals in the rows / columns of the
        mesh, corresponding to (M+1, N+1) vertices.
        The last dimension specifies the components (x, y).
        """
        coordinates = _collections.get_coordinates(self._native)
        # As matplotlib keeps them: integers for a mesh of the default grid
        # (masked vertices, NaN here, stay floats).
        dtype = self.__dict__.get("_coordinates_dtype")
        if dtype is None or not np.isfinite(coordinates).all():
            return coordinates
        return coordinates.astype(dtype, copy=False)

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

            - (M, N) or (M*N,): a mesh with scalar data. The values are mapped to
              colors using normalization and a colormap. See parameters *norm*,
              *cmap*, *vmin*, *vmax*.
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
                    f"{' or '.join(map(str, ok_shapes))}, not {shape}")
            if len(shape) == 3:
                # RGB(A) values are the colours of the cells, which
                # ``to_rgba`` passes through.
                mcolorizer.ColorizingArtist.set_array(self, A)
                _collections.set_array(self._native, None)
                _collections.set_facecolors(
                    self._native, self.to_rgba(self._A).reshape(-1, 4))
                return
        super().set_array(A)


class QuadMesh(_MeshData, Collection):
    """
    Class for the efficient drawing of a quadrilateral mesh.

    A quadrilateral mesh is a grid of M by N adjacent quadrilaterals that are
    defined via a (M+1, N+1) grid of vertices. The quadrilateral (m, n) is
    defined by the vertices ``(m, n)``, ``(m, n+1)``, ``(m+1, n+1)`` and
    ``(m+1, n)``. The mesh need not be regular and the polygons need not be
    convex.
    """

    def __init__(self, coordinates, *, antialiased=True, shading='flat',
                 **kwargs):
        """
        Parameters
        ----------
        coordinates : (M+1, N+1, 2) array-like
            The vertices. ``coordinates[m, n]`` specifies the (x, y)
            coordinates of vertex (m, n).
        antialiased : bool, default: True
            Whether the mesh is drawn antialiased.
        shading : {'flat', 'gouraud'}, default: 'flat'
            One colour per cell, or colours interpolated from the vertices.
        **kwargs
            Forwarded to `.Collection`.
        """
        kwargs.setdefault("pickradius", 0)
        _api.check_shape((None, None, 2), coordinates=coordinates)
        self._native = _collections.new_quad_mesh(
            np.ma.filled(np.ma.asarray(coordinates, dtype=float), np.nan),
            shading == 'gouraud', bool(antialiased))
        # The vertices read back as matplotlib keeps them (`get_coordinates`).
        self._coordinates_dtype = np.asarray(coordinates).dtype
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

    def get_datalim(self, transData):
        """The box of the mesh's coordinates in the coordinates of
        *transData* (matplotlib's, from its ``_bbox``)."""
        bbox = mtransforms.Bbox.unit()
        bbox.update_from_data_xy(self.get_coordinates().reshape(-1, 2))
        return (self.get_transform() - transData).transform_bbox(bbox)

    def get_cursor_data(self, event):
        """The values of the cells under the event position, None outside
        the mesh or when it maps no values."""
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
        """
        Return the antialiasing of the collection: :rc:`patch.antialiased`
        until set. matplotlib's mesh draws with the antialiasing it was made
        with (``pcolormesh(antialiased=...)``, `_antialiased`), which this
        does not read.
        """
        return np.array(self.__dict__.get(
            "_antialiaseds", [_rcParams()['patch.antialiased']]), dtype=bool)

    def set_antialiased(self, aa):
        """Set the antialiasing of the collection (see `get_antialiased`)."""
        if aa is None:
            aa = _rcParams()['patch.antialiased']
        self._antialiaseds = [bool(a) for a in np.atleast_1d(aa)]


class PolyQuadMesh(_MeshData, PolyCollection):
    """
    Class for drawing a quadrilateral mesh as individual Polygons (as
    `~.Axes.pcolor` makes).
    """

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
