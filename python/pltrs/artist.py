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

from numbers import Real

import numpy as np

from . import _handles, _pickling, cbook  # noqa: F401  (_pickling: how natives pickle)
from . import _live_transforms
from ._artist_callbacks import CallbacksMixin
from ._artist_children import ChildrenSearchMixin
from ._artist_clipping import ClippingMixin
from ._artist_effects import EffectsMixin
from ._artist_extent import ExtentMixin
from ._artist_picking import PickingMixin
from ._artist_subclassing import SubclassingMixin
from ._artist_units import UnitsMixin
from ._native import artist as _artist
from ._native import transforms as _transforms


class Artist(SubclassingMixin, CallbacksMixin, ClippingMixin, PickingMixin,
             EffectsMixin, ExtentMixin, UnitsMixin, ChildrenSearchMixin):
    """
    Abstract base class for objects that render into a FigureCanvas.

    Typically, all visible elements in a figure are subclasses of Artist.
    """

    def __init__(self):
        # Subclasses make their own native artist before calling this.
        if not hasattr(self, "_native"):
            self._native = _artist.new()
        self._figure = None

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
        # id (`._pickling`).
        return self.__dict__.copy()

    @property
    def figure(self):
        """The (root) `.Figure` the artist belongs to."""
        return self._figure

    def get_figure(self, root=False):
        """
        Return the `.Figure` or `.SubFigure` instance the artist belongs to.

        Parameters
        ----------
        root : bool, default=False
            If False, return the (Sub)Figure this artist is on.  If True,
            return the root Figure for a nested tree of SubFigures.
        """
        if root or self._figure is None:
            return self._figure
        from .figure import Figure
        return _handles.wrap(_artist.parent_figure(self._native), Figure,
                             self._figure)

    @property
    def axes(self):
        """The `~.axes.Axes` instance the artist resides in, or *None*."""
        if self._figure is None:
            return self.__dict__.get("_axes")
        from .axes import Axes
        return _handles.wrap(_artist.axes(self._native), Axes, self._figure)

    @axes.setter
    def axes(self, new_axes):
        if (new_axes is not None and self.axes is not None
                and new_axes is not self.axes):
            raise ValueError("Can not reset the Axes. You are probably trying to reuse "
                             "an artist in more than one Axes which is not supported")
        # Kept for an artist outside any figure; one in a figure is in the
        # axes pltrs (Rust) has it in.
        self._axes = new_axes

    def get_transform(self):
        """Return the `.Transform` instance used by this artist."""
        given = self.__dict__.get("_given_transform")
        if given is not None:
            return given
        return _live_transforms.of_artist(
            self, _transforms.get_transform(self._native))

    @property
    def _transform(self):
        """The transform the artist has: the one it was given, or the one
        an axes or a figure gave it when it was added; None before it has
        one (matplotlib's ``_transform``, which a patch, say, composes
        with a transform of its own)."""
        return Artist.get_transform(self) if self.is_transform_set() else None

    def set_transform(self, t):
        """
        Set the artist transform.

        Parameters
        ----------
        t : `~matplotlib.transforms.Transform`
        """
        _transforms.set_transform(self._native, _live_transforms.to_native(t))
        self._given_transform = t
        self.pchanged()

    def is_transform_set(self):
        """
        Return whether the Artist has an explicitly set transform.

        This is *True* after `.set_transform` has been called.
        """
        return _transforms.get_transform(self._native) is not None

    @property
    def stale(self):
        """Whether the artist needs drawing: pltrs (Rust) draws from scratch
        every time, so never."""
        return False

    @stale.setter
    def stale(self, value):
        pass

    def get_visible(self):
        """Return the visibility."""
        return _artist.get_visible(self._native)

    def set_visible(self, b):
        """Set the artist's visibility."""
        _artist.set_visible(self._native, bool(b))
        self.pchanged()

    def get_zorder(self):
        """Return the artist's zorder."""
        return _artist.get_zorder(self._native)

    def set_zorder(self, level):
        """
        Set the zorder for the artist.  Artists with lower zorder
        values are drawn first.

        Parameters
        ----------
        level : float
        """
        if level is None:
            # The class' default; `Artist.zorder` itself reads the artist's.
            default = type(self).zorder
            level = default if isinstance(default, Real) else 0
        _artist.set_zorder(self._native, level)
        self.pchanged()

    @property
    def zorder(self):
        """The drawing order."""
        return self.get_zorder()

    def get_alpha(self):
        """Return the alpha value used for blending - not supported on all
        backends."""
        return _artist.get_alpha(self._native)

    def set_alpha(self, alpha):
        """
        Set the alpha value used for blending - not supported on all backends.

        Parameters
        ----------
        alpha : float or None
            *alpha* must be within the 0-1 range, inclusive.
        """
        if alpha is not None and not isinstance(alpha, Real):
            raise TypeError(
                f'alpha must be numeric or None, not {type(alpha)}')
        if alpha is not None and not (0 <= alpha <= 1):
            raise ValueError(f'alpha ({alpha}) is outside 0-1 range')
        _artist.set_alpha(self._native, alpha)
        self.pchanged()

    def _set_alpha_for_array(self, alpha):
        """
        Set the alpha value used for blending - not supported on all backends.

        Parameters
        ----------
        alpha : array-like or float or None
            All values must be within the 0-1 range, inclusive.
            Masked values and nans are not supported. Only artists of many
            items (collections, images) take an array.
        """
        if isinstance(alpha, str):
            raise TypeError("alpha must be numeric or None, not a string")
        if not np.iterable(alpha):
            Artist.set_alpha(self, alpha)
            return
        alpha = np.asarray(alpha)
        if not (0 <= alpha.min() and alpha.max() <= 1):
            raise ValueError('alpha must be between 0 and 1, inclusive, '
                             f'but min is {alpha.min()}, max is {alpha.max()}')
        self.set_alpha(alpha)

    def get_clip_on(self):
        """Return whether the artist uses clipping."""
        return _artist.get_clip_on(self._native)

    def set_clip_on(self, b):
        """
        Set whether the artist uses clipping.

        When False, artists will be visible outside the Axes which
        can lead to unexpected results.
        """
        _artist.set_clip_on(self._native, bool(b))

    def get_rasterized(self):
        """Return whether the artist is to be rasterized."""
        return _artist.get_rasterized(self._native)

    def set_rasterized(self, rasterized):
        """
        Force rasterized (bitmap) drawing for vector graphics output.

        Rasterized drawing is not supported by all artists. If you try to
        enable this on an artist that does not support it, the command has no
        effect and a warning will be issued.
        """
        _artist.set_rasterized(self._native, bool(rasterized))

    def get_animated(self):
        """Return whether the artist is animated."""
        return _artist.get_animated(self._native)

    def set_animated(self, b):
        """
        Set whether the artist is intended to be used in an animation.

        If True, the artist is excluded from regular drawing of the figure.
        You have to call `.Figure.draw_artist` / `.Axes.draw_artist`
        explicitly on the artist.
        """
        _artist.set_animated(self._native, bool(b))

    def get_in_layout(self):
        """
        Return boolean flag, ``True`` if artist is included in layout
        calculations.
        """
        return _artist.get_in_layout(self._native)

    def set_in_layout(self, in_layout):
        """
        Set if artist is to be included in layout calculations,
        E.g. :ref:`constrainedlayout_guide`,
        `.Figure.tight_layout()`, and
        ``fig.savefig(fname, bbox_inches='tight')``.
        """
        _artist.set_in_layout(self._native, bool(in_layout))

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
        """
        _artist.set_mouseover(self._native, bool(mouseover))

    mouseover = property(get_mouseover, set_mouseover)

    def get_snap(self):
        """Return the snap setting."""
        return _artist.get_snap(self._native)

    def set_snap(self, snap):
        """
        Set the snapping behavior.

        Snapping aligns positions with the pixel grid, which results in
        clearer images.

        Parameters
        ----------
        snap : bool or None
            - *True*: Snap vertices to the nearest pixel center.
            - *False*: Do not modify vertex positions.
            - *None*: (auto) If the path contains only rectilinear line
              segments, round to the nearest pixel center.
        """
        _artist.set_snap(self._native, None if snap is None else bool(snap))

    def get_label(self):
        """Return the label used for this artist in the legend."""
        return _artist.get_label(self._native)

    def set_label(self, s):
        """Set a label that will be displayed in the legend."""
        _artist.set_label(self._native, str(s) if s is not None else "")

    def get_gid(self):
        """Return the group id."""
        return _artist.get_gid(self._native)

    def set_gid(self, gid):
        """Set the (group) id for the artist."""
        _artist.set_gid(self._native, None if gid is None else str(gid))

    def get_url(self):
        """Return the url."""
        return _artist.get_url(self._native)

    def set_url(self, url):
        """Set the url for the artist."""
        _artist.set_url(self._native, None if url is None else str(url))

    def get_sketch_params(self):
        """
        Return the sketch parameters for the artist: ``(scale, length,
        randomness)``, or None if no sketch parameters were set.
        """
        return _artist.get_sketch_params(self._native)

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
        """
        _artist.set_sketch_params(
            self._native, None if scale is None
            else (float(scale), float(length or 128.0),
                  float(randomness or 16.0)))

    def _add_to_axes(self, axes, kind, autolim=True):
        """
        Join *axes* as its ``add_<kind>`` method adds an artist (``kind`` is
        ``"artist"``, ``"line"``, ``"patch"``, ``"image"`` or
        ``"collection"``, the last with *autolim*); return self.
        """
        self._convert_units_for(axes)
        native = _artist.adopt_into_axes(axes._native, self._native, kind,
                                         bool(autolim))
        return _handles.adopt(self, native, axes.figure)

    def _add_to_figure(self, figure, clip):
        """Join *figure* as `.Figure.add_artist` adds an artist; return
        self."""
        native = _artist.adopt_into_figure(figure._native, self._native,
                                           bool(clip))
        return _handles.adopt(self, native, figure.figure)

    def remove(self):
        """Remove the artist from the figure if possible."""
        if self._figure is None:
            raise NotImplementedError('cannot remove artist')
        _artist.remove(self._native)
        _handles.forget(self)

    def properties(self):
        """Return a dictionary of all the properties of the artist."""
        return ArtistInspector(self).properties()

    def set(self, **kwargs):
        """Set multiple properties at once."""
        return self._internal_update(cbook.normalize_kwargs(kwargs, self))

    def update(self, props):
        """
        Update this artist's properties from the dict *props*.
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
            func = getattr(self, f"set_{k}", None)
            if not callable(func):
                raise AttributeError(
                    errfmt.format(cls=type(self), prop_name=k), name=k)
            ret.append(func(v))
        return ret

    def draw(self, renderer):
        """
        Draw the Artist (and its children) using the given renderer.

        This has no effect if the artist is not visible. pltrs (Rust) draws
        figures itself: the renderer it hands to the program's replacement
        of `.Text.draw` draws the artist where pltrs is drawing the text.
        An artist with no pltrs (Rust) counterpart draws nothing, as
        matplotlib's base `Artist.draw` does; subclasses override it.
        """
        if not self.get_visible():
            return
        draw_artist = getattr(renderer, "_draw_artist", None)
        if draw_artist is not None:
            draw_artist(self)
        elif getattr(self, "_native", None) is not None:
            raise NotImplementedError(
                "pltrs: Artist.draw(renderer) with a renderer pltrs (Rust) is "
                "not drawing with")
        self.stale = False


# The introspection of artist properties, which needs `Artist` defined.
from ._artist_inspector import ArtistInspector, get, getp, kwdoc, setp  # noqa: E402, F401

# The signature and docstring of `Artist.set`, which subclasses get theirs
# from as they are defined (`._artist_subclassing`).
Artist._update_set_signature_and_docstring()
