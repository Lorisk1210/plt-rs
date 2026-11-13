"""
The base class of everything drawn in a figure (matplotlib's ``artist``).

An `Artist` wraps the native handle of a pltrs (Rust) artist (``self._native``):
its getters and setters call the native functions of ``_native.artist``
and of its own module. Property handling that is inherently Python stays
here, as in matplotlib: ``set(**kwargs)`` resolves aliases and calls the
``set_<name>`` methods, which subclasses may override.

An artist made by its constructor (``Line2D(x, y)``, ``Artist()``) is
loose: its native handle reaches a pltrs (Rust) artist outside any figure,
and its ``figure`` is None. An axes or a figure adopting it
(``ax.add_line(line)``, ``fig.add_artist(artist)``) moves the artist into
its figure and gives the object the new handle (`._handles.adopt`).
"""

import contextlib
from functools import cache, reduce, wraps  # noqa: F401  (matplotlib exports reduce here)
from numbers import Real
from operator import attrgetter

import numpy as np

from . import _api, _artist_effects, _handles, _pickling, cbook  # noqa: F401  (_pickling: how natives pickle)
from . import _live_transforms, _mirrors, _program_classes, _pybind, _python_drawing, _unusable, _values
from . import _transform_trees
from ._artist_callbacks import CallbacksMixin
from ._artist_children import ChildrenSearchMixin
from ._artist_clipping import ClippingMixin
from ._artist_copying import CopyingMixin, _XYPair, detach_shallow_copy
from ._artist_effects import EffectsMixin
from ._artist_extent import ExtentMixin
from ._artist_picking import PickingMixin
from ._artist_private import PrivateAttributesMixin
from ._artist_staleness import StalenessMixin, _stale_axes_callback  # noqa: F401
from ._artist_subclassing import SubclassingMixin, _LazySet
from ._artist_units import UnitsMixin
from ._native import artist as _artist
from ._native import transforms as _transforms


def allow_rasterization(draw):
    """
    Decorator for Artist.draw method. Provides routines
    that run before and after the draw call. The before and after functions
    are useful for changing artist-dependent renderer attributes or making
    other setup function calls, such as starting and flushing a mixed-mode
    renderer.
    """

    @wraps(draw)
    def draw_wrapper(artist, renderer):
        try:
            if artist.get_rasterized():
                if renderer._raster_depth == 0 and not renderer._rasterizing:
                    renderer.start_rasterizing()
                    renderer._rasterizing = True
                renderer._raster_depth += 1
            else:
                if renderer._raster_depth == 0 and renderer._rasterizing:
                    # Only stop when we are not in a rasterized parent
                    # and something has be rasterized since last stop
                    renderer.stop_rasterizing()
                    renderer._rasterizing = False

            if artist.get_agg_filter() is not None:
                renderer.start_filter()
                with _artist_effects.applying_filter(artist):
                    return draw(artist, renderer)

            return draw(artist, renderer)
        finally:
            if artist.get_agg_filter() is not None:
                renderer.stop_filter(artist.get_agg_filter())
            if artist.get_rasterized():
                renderer._raster_depth -= 1
            if (renderer._rasterizing and (fig := artist.get_figure(root=True)) and
                    fig.suppressComposite):
                # restart rasterizing to prevent merging
                renderer.stop_rasterizing()
                renderer.start_rasterizing()

    draw_wrapper._supports_rasterization = True
    return draw_wrapper


def _check_alpha(alpha):
    """Raise matplotlib's errors for an *alpha* `Artist.set_alpha` refuses:
    one that is no number, or outside the 0-1 range (NaN included)."""
    if alpha is not None and not isinstance(alpha, Real):
        raise TypeError(
            f'alpha must be numeric or None, not {type(alpha)}')
    if alpha is not None and not (0 <= alpha <= 1):
        raise ValueError(f'alpha ({alpha}) is outside 0-1 range')


def _check_array_alpha(alpha):
    """Raise matplotlib's errors for an *alpha* `.Artist._set_alpha_for_array`
    refuses: a string, a number `Artist.set_alpha` refuses, or an array
    with a value outside the 0-1 range."""
    if isinstance(alpha, str):
        raise TypeError("alpha must be numeric or None, not a string")
    if not np.iterable(alpha):
        _check_alpha(alpha)
        return
    alpha = np.asarray(alpha)
    if not (0 <= alpha.min() and alpha.max() <= 1):
        raise ValueError('alpha must be between 0 and 1, inclusive, '
                         f'but min is {alpha.min()}, max is {alpha.max()}')


@cache
def _axes_class():
    """`~.axes.Axes`, imported on first use (the axes import this module)."""
    from .axes import Axes
    return Axes


class Artist(SubclassingMixin, CallbacksMixin, ClippingMixin, PickingMixin,
             EffectsMixin, ExtentMixin, UnitsMixin, ChildrenSearchMixin,
             CopyingMixin, StalenessMixin, PrivateAttributesMixin):
    """
    Abstract base class for objects that render into a FigureCanvas.

    Typically, all visible elements in a figure are subclasses of Artist.
    """

    def __init__(self):
        # Subclasses make their own native artist before calling this.
        if not hasattr(self, "_native"):
            self._native = _artist.new()
        self._figure = None
        # A subclass of the program's draws at its class' z-order, and
        # through its own `draw` if it has one.
        _program_classes.take_class_defaults(self)
        _python_drawing.attach(self)

    def __setattr__(self, name, value):
        super().__setattr__(name, value)
        if name == "draw" and "_native" in vars(self):  # As in matplotlib.
            _python_drawing.attach(self, again=True)

    def __delattr__(self, name):
        super().__delattr__(name)
        if name == "draw" and "_native" in vars(self):
            _python_drawing.attach(self, again=True)

    @classmethod
    def _from_native(cls, native, figure):
        """The object of class *cls* around the native handle *native* of
        an artist of *figure* (see `._handles.wrap`)."""
        artist = cls.__new__(cls)
        artist._native = native
        artist._figure = figure
        return artist

    def __getstate__(self):
        # The native handle pickles as its figure's state and the artist's
        # id (`._pickling`), a transform of the trees as a reference.
        return _pickling.referring_to_trees(self, self.__dict__.copy())

    def __setstate__(self, state):
        # Its figure, read back first when the pickle reached it from this
        # artist, may wait for the artist's state (`._pickling.when_built`).
        waiting = self.__dict__.pop(_pickling.WAITING, ())
        self.__dict__.update(state)
        _pickling.resolve_trees(self)
        detach_shallow_copy(self)
        for callback in waiting:
            callback()

    def set_figure(self, fig):
        """
        Set the `.Figure` or `.SubFigure` instance the artist belongs to.

        .. warning::

            This function should typically not be called by users.

            This is a low-level API and only modifies the internal state of the
            Artist. Only modifying this is not enough and will typically lead
            to an inconsistent state, because the state of the figure has to
            be changed as well.

            Users should typically instead call one of the high-level APIs, i.e.
            `.Figure.add_artist` or one of the Axes methods
            :ref:`axes-api-adding-artists`.

        Parameters
        ----------
        fig : `~matplotlib.figure.Figure` or `~matplotlib.figure.SubFigure`
        """
        if fig is self.get_figure(root=False):
            return
        # A part of another artist answers with that artist's figure.
        if self.get_figure(root=False) is not None or (
                self._figure is not None and not self.__dict__.get("_decoupled")):
            raise RuntimeError("Can not put single artist in "
                               "more than one figure")
        _handles.adopt(self, _artist.take_into_figure(fig._native, self._native),
                       fig.get_figure(root=True))
        # In no list of the figure's: matplotlib's has no removal method.
        self._only_given_figure = True
        self.pchanged()
        self.stale = True

    def get_figure(self, root=False):
        """
        Return the `.Figure` or `.SubFigure` instance the artist belongs to.

        Parameters
        ----------
        root : bool, default=False
            If False, return the (Sub)Figure this artist is on.  If True,
            return the root Figure for a nested tree of SubFigures.
        """
        return self._get_figure(root)

    def _get_figure(self, root):
        """What `get_figure` returns. Artists that find their figure
        elsewhere override this rather than `get_figure`, so that the
        `figure` property, made of `get_figure` itself, reads it too."""
        if self.__dict__.get("_no_figure") or self.__dict__.get("_decoupled"):
            # In its figure without belonging to it, as matplotlib's
            # artists without a figure that an aux-transform box holds, and
            # those removed or cleared from it (`_decouple`), unless an Axes
            # of another figure took it since (`.axes._foreign_children`).
            listed_in = self.__dict__.get("_listed_in")
            return None if listed_in is None else listed_in.get_figure(root=root)
        return self._home_figure(root)

    # As matplotlib's, made of `Artist.get_figure` and `Artist.set_figure`
    # themselves: a subclass' `set_figure` (an Axes' making its transforms
    # anew) is not called.
    figure = property(get_figure, set_figure,
                      doc=("The (Sub)Figure that the artist is on.  For more "
                           "control, use the `get_figure` method."))

    def _home_figure(self, root=False):
        """The (sub)figure pltrs (Rust) keeps the artist in, the root with
        *root*: its figure, also once it was decoupled from it."""
        if (root or self._figure is None
                or not self._figure.__dict__.get("_subfigures_made")):
            return self._figure
        from .figure import Figure
        return _handles.wrap(_artist.parent_figure(self._native), Figure,
                             self._figure)

    @property
    def axes(self):
        """The `~.axes.Axes` instance the artist resides in, or *None*."""
        if self._figure is None or self.__dict__.get("_decoupled"):
            return self._loose_axes()
        if self.__dict__.get("_axes_unset"):
            # Still drawn in its Axes, the artist was told it has none.
            return None
        if self.__dict__.get("_axes_as_given"):
            # Drawn in an Axes that never gave it to the artist (one inside
            # an annotation box's tree).
            return self.__dict__.get("_axes")
        axes = _handles.wrap(_artist.axes(self._native), _axes_class(), self._figure)
        # An artist the program gave a figure and an Axes without adding
        # it (cartopy's gridliner labels) keeps the Axes it was given.
        return self.__dict__.get("_axes") if axes is None else axes

    @axes.setter
    def axes(self, new_axes):
        if (new_axes is not None and self.axes is not None
                and new_axes is not self.axes):
            raise ValueError("Can not reset the Axes. You are probably trying to reuse "
                             "an artist in more than one Axes which is not supported")
        # Kept for an artist outside any figure; one in a figure is in the
        # axes pltrs (Rust) has it in, unless it was told it has none.
        if new_axes is None and self._figure is not None and self.axes is not None:
            self._axes_unset = True
        elif new_axes is not None:
            self.__dict__.pop("_axes_unset", None)
        self._axes = new_axes
        self._axes_given(new_axes)

    def _axes_given(self, axes):
        """Take the Axes *axes* (or None) the program set as the artist's
        (`axes`): subclasses whose coordinates read it hand it on."""

    def _loose_axes(self):
        """The Axes of the artist on no figure: the one it was given."""
        return self.__dict__.get("_axes")

    def get_transform(self):
        """Return the `.Transform` instance used by this artist."""
        given = self.__dict__.get("_given_transform")
        if _live_transforms.gives_transform(given):
            # The transform an object of the program's gives for the
            # artist's Axes, asked now as matplotlib asks it.
            self.set_transform(given._as_mpl_transform(self.axes))
            given = self._given_transform
        if given is not None:
            return given
        # matplotlib's makes the identity its ``_transform`` here.
        self.__dict__["_transform_made"] = True
        return _live_transforms.of_artist(
            self, _transforms.get_transform(self._native))

    @property
    def _transform(self):
        """The transform the artist has: the one it was given, or the one
        an axes or a figure gave it when it was added, or the identity once
        `get_transform` made it; None before (matplotlib's ``_transform``,
        which a patch, say, composes with a transform of its own). Read as
        `Artist.get_transform` gives it (a tick's line its mirror's), not
        a patch's composed one."""
        if not (self.is_transform_set() or "_transform_made" in self.__dict__):
            return None
        mirroring = _mirrors.mirroring(self)
        if mirroring is not None:
            transform = mirroring.transform(self)
            if transform is not _mirrors.OWN:
                return transform
        return Artist.get_transform(self)

    def set_transform(self, t):
        """
        Set the artist transform.

        Parameters
        ----------
        t : `~matplotlib.transforms.Transform`
        """
        if _live_transforms.gives_transform(t):
            # An object of the program's giving its transform for an Axes
            # (a cartopy CRS): that of the artist's Axes, asked once the
            # artist is in one, as matplotlib's ``get_transform`` asks it.
            axes = self.axes
            if axes is None:
                self._given_transform = t
                self.__dict__.setdefault("_on_adopt", []).append(
                    lambda: self.set_transform(t))
                self.pchanged()
                return
            t = t._as_mpl_transform(axes)
        if _live_transforms.holds_natively(t):
            _transforms.set_transform(self._native, _live_transforms.to_native(t))
        self._given_transform = t
        self.pchanged()

    def is_transform_set(self):
        """
        Return whether the Artist has an explicitly set transform.

        This is *True* after `.set_transform` has been called.
        """
        return ("_given_transform" in self.__dict__
                or _transforms.get_transform(self._native) is not None)

    @property
    def stale(self):
        """
        Whether the artist is 'stale' and needs to be re-drawn for the output
        to match the internal state of the artist.
        """
        # A value other than a bool reads back as given (matplotlib keeps
        # it) while pltrs (Rust) holds its truth.
        return _values.given_flag(self, "stale", _artist.get_stale(self._native))

    @stale.setter
    def stale(self, val):
        # pltrs (Rust) marks what contains the artist stale too, unless
        # the artist is animated.
        _artist.set_stale(self._native, bool(val))
        if type(val) is not bool:
            _values.remember(self, "stale", val)
        elif "_given_values" in self.__dict__:
            self.__dict__["_given_values"].pop("stale", None)
        self._went_stale(val)

    def get_visible(self):
        """Return the visibility."""
        return _values.given_flag(self, "visible", _artist.get_visible(self._native))

    def set_visible(self, b):
        """
        Set the artist's visibility.

        Parameters
        ----------
        b : bool
        """
        if b != self.get_visible():
            _artist.set_visible(self._native, bool(b))
            _values.remember(self, "visible", b)
            self.pchanged()
            self.stale = True

    def get_zorder(self):
        """Return the artist's zorder."""
        # matplotlib's own z-orders are int literals but a few (2.5, 1.5):
        # one pltrs (Rust) gave and the program did not is an int when whole.
        native = _artist.get_zorder(self._native)
        return _values.given(self, "zorder", native,
                             int(native) if float(native).is_integer() else native)

    def set_zorder(self, level):
        """
        Set the zorder for the artist.  Artists with lower zorder
        values are drawn first.

        Parameters
        ----------
        level : float
        """
        if level is None:
            level = type(self).zorder
        if level != self.get_zorder():
            if isinstance(level, Real):
                _artist.set_zorder(self._native, level)
                _values.remember(self, "zorder", level)
                _unusable.drop(self, "zorder")
            else:
                # Kept unchecked, as matplotlib keeps it until it sorts.
                # The parent's draw fails sorting before it draws any of
                # its children: pltrs (Rust) draws the artist, which fails
                # as that sort does, first among them.
                _artist.set_zorder(self._native, float("-inf"))
                _values.keep(self, "zorder", level)
                _unusable.keep(self, "zorder", level, self._sort_with_siblings,
                               measured=False)
            self.pchanged()
            self.stale = True

    def _sort_with_siblings(self, _):
        """Fail as matplotlib's draw of the Axes (or figure) holding the
        artist fails to sort what it draws by z-order, a z-order that is
        no number among them."""
        parent = self._sorted_among()
        if parent is None:
            return
        drawn = [artist for artist in parent.get_children()
                 if artist is not parent.patch and not artist.get_animated()]
        sorted(drawn, key=attrgetter("zorder"))

    def _sorted_among(self):
        """The artist whose draw sorts this one by z-order among what it
        draws: its Axes, else its (sub)figure."""
        axes = self.axes
        return axes if axes is not None and axes is not self else self.get_figure()

    #: The drawing order (see `.ClassZOrder`).
    zorder = 0

    def get_alpha(self):
        """
        Return the alpha value used for blending - not supported on all
        backends.
        """
        return self._alpha

    def set_alpha(self, alpha):
        """
        Set the alpha value used for blending - not supported on all backends.

        Parameters
        ----------
        alpha : float or None
            *alpha* must be within the 0-1 range, inclusive.
        """
        _check_alpha(alpha)
        # matplotlib stores only an alpha unequal to the one it holds
        # (``np.float32(0.5)`` after ``0.5`` keeps the float).
        if not alpha != self._alpha:
            return
        if self._kept_only:
            self._kept_alpha = alpha
            self.stale = True
        else:
            _artist.set_alpha(self._native, alpha)
            _values.remember(self, "alpha", alpha)
        self.pchanged()

    #: Whether the alpha, transform, clip regions and path effects the
    #: artist is given are only kept, matplotlib drawing nothing with them
    #: (a figure's: its background has an alpha of its own), else pltrs
    #: (Rust) draws the artist with them.
    _kept_only = False

    @property
    def _alpha(self):
        """The alpha set on the artist, None for none, as given (matplotlib's
        ``_alpha``, which `set_alpha` sets)."""
        if self._kept_only:
            return self.__dict__.get("_kept_alpha")
        return _values.given(self, "alpha", _artist.get_alpha(self._native))

    @_alpha.setter
    def _alpha(self, alpha):
        # Assigned as matplotlib's field is (legendkit): without the checks
        # and the callbacks of `set_alpha`, drawn from the next draw.
        if self._kept_only:
            self._kept_alpha = alpha
        else:
            _artist.set_alpha(self._native, None if alpha is None else float(alpha))
            _values.remember(self, "alpha", alpha)

    def _set_alpha_for_array(self, alpha):
        """
        Set the alpha value used for blending - not supported on all backends.

        Parameters
        ----------
        alpha : array-like or float or None
            All values must be within the 0-1 range, inclusive.
            Masked values and nans are not supported.
        """
        _check_array_alpha(alpha)
        if not np.iterable(alpha):
            Artist.set_alpha(self, alpha)
            return
        self.set_alpha(np.asarray(alpha))

    def get_clip_on(self):
        """Return whether the artist uses clipping."""
        return _values.given_flag(self, "clip_on", _artist.get_clip_on(self._native))

    def set_clip_on(self, b):
        """
        Set whether the artist uses clipping.

        When False, artists will be visible outside the Axes which
        can lead to unexpected results.

        Parameters
        ----------
        b : bool
        """
        _artist.set_clip_on(self._native, bool(b))
        _values.remember(self, "clip_on", b)

    def get_rasterized(self):
        """Return whether the artist is to be rasterized."""
        return _values.given_flag(self, "rasterized",
                                  _artist.get_rasterized(self._native))

    def _assign_rasterized(self, rasterized):
        """``self._rasterized = rasterized``, matplotlib's attribute,
        which a program sets without the checks of `set_rasterized`
        (legendkit's ``ColorArt``)."""
        _artist.set_rasterized(self._native, bool(rasterized))
        _values.remember(self, "rasterized", rasterized)

    _rasterized = property(get_rasterized, _assign_rasterized,
                           doc="Whether the artist is to be rasterized.")

    def set_rasterized(self, rasterized):
        """
        Force rasterized (bitmap) drawing for vector graphics output.

        Rasterized drawing is not supported by all artists. If you try to
        enable this on an artist that does not support it, the command has no
        effect and a warning will be issued.

        This setting is ignored for pixel-based output.

        See also :doc:`/gallery/misc/rasterization_demo`.

        Parameters
        ----------
        rasterized : bool
        """
        if rasterized and self._rasterization_ignored():
            _api.warn_external(f"Rasterization of '{self}' will be ignored")
        _artist.set_rasterized(self._native, bool(rasterized))
        _values.remember(self, "rasterized", rasterized)

    def get_animated(self):
        """Return whether the artist is animated."""
        return _values.given_flag(self, "animated", _artist.get_animated(self._native))

    def set_animated(self, b):
        """
        Set whether the artist is intended to be used in an animation.

        If True, the artist is excluded from regular drawing of the figure.
        You have to call `.Figure.draw_artist` / `.Axes.draw_artist`
        explicitly on the artist. This approach is used to speed up animations
        using blitting.

        See also `matplotlib.animation` and
        :ref:`blitting`.

        Parameters
        ----------
        b : bool
        """
        if self.get_animated() != b:
            _artist.set_animated(self._native, bool(b))
            _values.remember(self, "animated", b)
            self.pchanged()

    def get_in_layout(self):
        """
        Return boolean flag, ``True`` if artist is included in layout
        calculations.

        E.g. :ref:`constrainedlayout_guide`,
        `.Figure.tight_layout()`, and
        ``fig.savefig(fname, bbox_inches='tight')``.
        """
        return _values.given_flag(self, "in_layout", _artist.get_in_layout(self._native))

    def set_in_layout(self, in_layout):
        """
        Set if artist is to be included in layout calculations,
        E.g. :ref:`constrainedlayout_guide`,
        `.Figure.tight_layout()`, and
        ``fig.savefig(fname, bbox_inches='tight')``.

        Parameters
        ----------
        in_layout : bool
        """
        _artist.set_in_layout(self._native, bool(in_layout))
        _values.remember(self, "in_layout", in_layout)

    def get_mouseover(self):
        """
        Return whether this artist is queried for custom context information
        when the mouse cursor moves over it.
        """
        return _artist.get_mouseover(self._native)

    def set_mouseover(self, mouseover):
        """
        Set whether this artist is queried for custom context information when
        the mouse cursor moves over it.

        Parameters
        ----------
        mouseover : bool

        See Also
        --------
        get_cursor_data
        .ToolCursorPosition
        .NavigationToolbar2
        """
        _artist.set_mouseover(self._native, bool(mouseover))
        axes = self.axes
        if axes:
            # matplotlib adds the artist to its Axes' ``_mouseover_set``,
            # which an Axes of pltrs's computes when read: an ``axes`` that
            # is no Axes (a figure's list of them) fails as there.
            axes._mouseover_set  # noqa: B018

    mouseover = property(get_mouseover, set_mouseover)

    def get_snap(self):
        """
        Return the snap setting.

        See `.set_snap` for details.
        """
        from . import rcParams
        if not rcParams['path.snap']:
            return False
        return _values.given_flag(self, "snap", _artist.get_snap(self._native))

    def set_snap(self, snap):
        """
        Set the snapping behavior.

        Snapping aligns positions with the pixel grid, which results in
        clearer images. For example, if a black line of 1px width was
        defined at a position in between two pixels, the resulting image
        would contain the interpolated value of that line in the pixel grid,
        which would be a grey value on both adjacent pixel positions. In
        contrast, snapping will move the line to the nearest integer pixel
        value, so that the resulting image will really contain a 1px wide
        black line.

        Snapping is currently only supported by the Agg and MacOSX backends.

        Parameters
        ----------
        snap : bool or None
            Possible values:

            - *True*: Snap vertices to the nearest pixel center.
            - *False*: Do not modify vertex positions.
            - *None*: (auto) If the path contains only rectilinear line
              segments, round to the nearest pixel center.
        """
        if _pybind.boolean(snap):
            _artist.set_snap(self._native, None if snap is None else bool(snap))
            _unusable.drop(self, "snap")
            _values.remember(self, "snap", snap)
        else:
            # matplotlib keeps what no ``e_snap_mode`` casts from (text, a
            # list), failing where its draw hands a renderer a graphics
            # context snapping so (`_draws_snapped`).
            _unusable.keep(self, "snap", snap, self._draw_snap, measured=False)
            _values.keep(self, "snap", snap)

    def _draw_snap(self, snap):
        """Fail as matplotlib's renderer fails to cast the *snap* of the
        graphics context the artist draws with, where its draw draws with
        its own snapping."""
        if self._draws_snapped():
            raise _pybind.cast_error(snap)

    def _draws_snapped(self):
        """Whether matplotlib's draw of the artist hands a renderer a
        graphics context with the artist's snapping (``gc.set_snap(
        self.get_snap())``) as it is now: not a bare artist's."""
        return False

    def get_label(self):
        """Return the label used for this artist in the legend."""
        return self._label

    def set_label(self, s):
        """
        Set a label that will be displayed in the legend.

        Parameters
        ----------
        s : object
            *s* will be converted to a string by calling `str`.
        """
        label = str(s) if s is not None else None
        if label != self._label:
            self._label = label
            self.pchanged()
            self.stale = True

    @property
    def _label(self):
        # matplotlib's attribute, which its containers keep themselves: the
        # label pltrs (Rust) has, None for one set to None.
        label = _artist.get_label(self._native)
        return None if label == "" and self.__dict__.get("_label_none") else label

    @_label.setter
    def _label(self, label):
        _artist.set_label(self._native, "" if label is None else label)
        self._label_none = label is None

    def get_gid(self):
        """Return the group id."""
        return _values.given_text(self, "gid", _artist.get_gid(self._native))

    def set_gid(self, gid):
        """
        Set the (group) id for the artist.

        Parameters
        ----------
        gid : str
        """
        _artist.set_gid(self._native, None if gid is None else str(gid))
        _values.remember(self, "gid", gid)

    def get_url(self):
        """Return the url."""
        return _values.given_text(self, "url", _artist.get_url(self._native))

    def set_url(self, url):
        """
        Set the url for the artist.

        Parameters
        ----------
        url : str
        """
        _artist.set_url(self._native, None if url is None else str(url))
        _values.remember(self, "url", url)

    def get_sketch_params(self):
        """
        Return the sketch parameters for the artist.

        Returns
        -------
        tuple or None

            A 3-tuple with the following elements:

            - *scale*: The amplitude of the wiggle perpendicular to the
              source line.
            - *length*: The length of the wiggle along the line.
            - *randomness*: The scale factor by which the length is
              shrunken or expanded.

            Returns *None* if no sketch parameters were set.
        """
        return _values.given(self, "sketch_params",
                             _artist.get_sketch_params(self._native))

    def set_sketch_params(self, scale=None, length=None, randomness=None):
        """
        Set the sketch parameters.

        Parameters
        ----------
        scale : float, optional
            The amplitude of the wiggle perpendicular to the source
            line, in pixels.  If scale is `None`, or not provided, no
            sketch filter will be provided.
        length : float, optional
             The length of the wiggle along the line, in pixels
             (default 128.0)
        randomness : float, optional
            The scale factor by which the length is shrunken or
            expanded (default 16.0)

            The PGF backend uses this argument as an RNG seed and not as
            described above. Using the same seed yields the same random shape.

            .. ACCEPTS: (scale: float, length: float, randomness: float)
        """
        sketch = (None if scale is None
                  else (scale, length or 128.0, randomness or 16.0))
        try:
            native = None if sketch is None else tuple(map(float, sketch))
        except (TypeError, ValueError):
            # Kept unchecked, as matplotlib keeps it until it draws.
            _values.keep(self, "sketch_params", sketch)
        else:
            _artist.set_sketch_params(self._native, native)
            _values.remember(self, "sketch_params", sketch)
        self.stale = True

    def _add_to_axes(self, axes, kind, autolim=True):
        """
        Join *axes* as its ``add_<kind>`` method adds an artist (``kind`` is
        ``"artist"``, ``"line"``, ``"patch"``, ``"image"`` or
        ``"collection"``, the last with *autolim*, or ``"text"`` for
        ``_add_text``); return self.
        """
        if kind == "artist":
            # matplotlib's `add_artist` sets the Axes before the figure.
            self.axes = axes
        self._convert_units_for(axes)
        had_transform = self.is_transform_set()
        native = _artist.adopt_into_axes(axes._native, self._native, kind,
                                         bool(autolim))
        _handles.adopt(self, native, axes.figure)
        _program_classes.set_artist_props(self, had_transform, axes)
        if kind == "patch":
            _transform_trees.patch_added(axes, self)
        return self

    def _add_to_figure(self, figure, clip):
        """Join *figure* as `.Figure.add_artist` adds an artist; return
        self."""
        had_transform = self.is_transform_set()
        native = _artist.adopt_into_figure(figure._native, self._native,
                                           bool(clip))
        _handles.adopt(self, native, figure.figure)
        _program_classes.set_artist_props(self, had_transform)
        return self

    def remove(self):
        """
        Remove the artist from the figure if possible.

        The effect will not be visible until the figure is redrawn, e.g.,
        with `.FigureCanvasBase.draw_idle`.  Call `~.axes.Axes.relim` to
        update the Axes limits if desired.

        Note: there is no support for removing the artist's legend entry.
        """
        if self.__dict__.get("_out_of_lists"):
            # Of its figure still, out of the list of a figure cleared.
            self._remove_again()
            return
        decoupled = self._figure is None or self.__dict__.get("_decoupled")
        if decoupled and "_listing" not in self.__dict__:
            if self.__dict__.get("_removed"):
                self._remove_again()
                return
            raise NotImplementedError('cannot remove artist')
        # An owner removing the artist its own way (a colorbar its Axes), or
        # an Axes listing what pltrs (Rust) holds elsewhere (a part of its
        # own, an artist of another figure: ``_listing``, see
        # `.axes._foreign_children`).
        remove_method = self.__dict__.get("_remove_method")
        if remove_method is not None:
            remove_method(self)
            self._decouple(removed=True)
            return
        if _artist.is_part(self._native) or self.__dict__.get("_only_given_figure"):
            # Made by its owner as a part of itself (an Axes' title), or only
            # given its figure (`set_figure`), it has no removal method.
            raise NotImplementedError('cannot remove artist')
        self._take_out_of_figure()

    def _take_out_of_figure(self):
        """Take the artist out of its figure, as pltrs (Rust) holds it,
        whatever removal method it has."""
        # Decoupled first: the end of the native removal brings the
        # artist's trees up to date, which reads it.
        self._decouple(removed=True)
        self._remove_natively()
        _handles.forget(self)

    def _remove_again(self):
        """Remove the artist once more, as matplotlib's removal method does:
        the list the artist was taken out of no longer finds it."""
        raise ValueError("list.remove(x): x not in list")

    def _remove_natively(self):
        """Take the artist out of its figure in pltrs (Rust) (`remove`; a
        class made of several native artists takes them all out)."""
        _artist.remove(self._native)

    def _decouple(self, removed):
        """
        Leave the artist decoupled from its figure and Axes, as matplotlib's
        `remove` and ``clear`` leave it: its ``figure`` and ``axes`` are
        None, while pltrs (Rust) keeps it apart in its old figure, where it
        keeps measuring with the transform and clip box it had there, as
        matplotlib's keeps its ``_transform``. *removed* tells whether it
        was removed (removing it again fails as matplotlib's removal method
        does) rather than cleared (which leaves matplotlib's none).
        """
        self._keep_drawn_state()
        self._decoupled = True
        self.__dict__.pop("_axes", None)
        self.__dict__.pop("_listed_in", None)
        if removed:
            self._removed = True

    def _recouple(self):
        """Undo `_decouple`: the artist is in its figure (and Axes) again,
        as one of their add methods leaves it."""
        for name in ("_removed", "_decoupled", "_left_host", "_listed_in",
                     "_out_of_lists", "_axes_unset", "_only_given_figure"):
            self.__dict__.pop(name, None)

    def _keep_drawn_state(self):
        """Compute now, in its figure and Axes, what matplotlib's last draw
        of the artist computed and the artist keeps once decoupled from
        them (see `_decouple`): nothing for most artists."""

    def properties(self):
        """Return a dictionary of all the properties of the artist."""
        return ArtistInspector(self).properties()

    def set(self, **kwargs):
        """Set multiple properties at once."""
        return self._internal_update(cbook.normalize_kwargs(kwargs, self))

    def update(self, props):
        """
        [*Discouraged*] Update this artist's properties from the dictionary *props*.

        .. admonition:: Discouraged

            This method exists for historic reasons. Please use `.Artist.set` instead.
            ``artist.update(props)`` is nowadays almost identical to
            ``artist.set(**props)`` with the only difference that ``set`` will
            additionally check that a property is not specified multiple times
            through aliases.

        Parameters
        ----------
        props : dict
            Dictionary of properties (keys) and their new values.

        Returns
        -------
        list
            A list of return values from the configured setters.

        """
        return self._update_props(
            props, "{cls.__name__!r} object has no property {prop_name!r}")

    def _internal_update(self, kwargs):
        """
        Update artist properties without prenormalizing them, but generating
        errors as if calling `set`.
        """
        return self._update_props(
            kwargs, "{cls.__name__}.set() got an unexpected keyword argument "
            "{prop_name!r}")

    def _update_props(self, props, errfmt):
        """
        Helper for `.Artist.set` and `.Artist.update`.

        *errfmt* is used to generate error messages for invalid property
        names; it gets formatted with ``type(self)`` for "{cls}" and the
        property name for "{prop_name}".
        """
        ret = []
        for k, v in props.items():
            # matplotlib lets art.update, art.set and setp set the axes.
            if k == "axes":
                ret.append(setattr(self, k, v))
                continue
            func = getattr(self, f"set_{k}", None)
            if not callable(func):
                raise AttributeError(
                    errfmt.format(cls=type(self), prop_name=k), name=k)
            ret.append(func(v))
        return ret

    @contextlib.contextmanager
    def _cm_set(self, **kwargs):
        """
        A context manager to temporarily set artist properties.

        In contrast to `.Artist.set` and for performance, this skips the
        `normalize_kwargs` check.
        """
        orig_vals = {k: getattr(self, f"get_{k}")() for k in kwargs}
        try:
            self._internal_update({k: kwargs[k] for k in orig_vals})
            yield
        finally:
            self._internal_update(orig_vals)

    def draw(self, renderer):
        """
        Draw the Artist (and its children) using the given renderer.

        This has no effect if the artist is not visible (`.Artist.get_visible`
        returns False).

        Parameters
        ----------
        renderer : `~matplotlib.backend_bases.RendererBase` subclass.

        Notes
        -----
        This method is overridden in the Artist subclasses.
        """
        # This has no effect if the artist is not visible. pltrs (Rust) draws
        # the artist as it draws it in a figure, through *renderer*: where
        # pltrs is drawing with a renderer it handed Python, onto the
        # renderer's canvas, or through the drawing methods of a renderer of
        # the program's (see `._python_drawing`). A bare `Artist` draws
        # nothing, as matplotlib's does; subclasses override it.
        if not self.get_visible():
            return
        draw_artist = getattr(renderer, "_draw_artist", None)
        if draw_artist is None and not hasattr(renderer, "draw_path"):
            # No renderer (None, a number) reaches here only through
            # `Artist.draw` itself (the wrappers of the subclasses' draws
            # refuse it), which draws nothing in matplotlib.
            return
        _unusable.check_ahead(self)
        # What matplotlib's draw reads, computed by a class of the program's.
        push_computed = getattr(self, "_push_computed", None)
        if push_computed is not None and _python_drawing.figure_changeable():
            push_computed()
        if _python_drawing.figure_changeable():
            _python_drawing.evaluate_given_transform(self)
        if draw_artist is not None:
            draw_artist(self)
        else:
            _python_drawing.draw_through(renderer, self)
        self.stale = False


# The introspection of artist properties, which needs `Artist` defined.
from ._artist_inspector import ArtistInspector, get, getp, kwdoc, setp  # noqa: E402, F401

# The signature and docstring of `Artist.set`, made when it is first looked
# up, as those of its subclasses (`._artist_subclassing`).
Artist.set = _LazySet(Artist, Artist.set)


cbook._set_module(__name__, _XYPair, ArtistInspector, get, getp, setp, kwdoc)
cbook._own_mixin_methods(Artist, object)


# What matplotlib's module also has from its imports, which programs
# import from it.
import inspect  # noqa: E402, F401
import logging  # noqa: E402, F401
import operator  # noqa: E402, F401
import re  # noqa: E402, F401
import warnings  # noqa: E402, F401
from collections import namedtuple  # noqa: E402, F401
from inspect import Parameter, Signature  # noqa: E402, F401
from numbers import Number  # noqa: E402, F401
from .path import Path  # noqa: E402, F401
from .transforms import (  # noqa: E402, F401
    Bbox, BboxBase, IdentityTransform, Transform, TransformedBbox, TransformedPatchPath,
    TransformedPath)
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
