"""
The base class of everything drawn in a figure (matplotlib's ``artist``).

An `Artist` wraps the native handle of a pltrs (Rust) artist (``self._native``):
its getters and setters call the native functions of ``_native.artist``
and of its own module. Property handling that is inherently Python stays
here, as in matplotlib: ``set(**kwargs)`` resolves aliases and calls the
``set_<name>`` methods, which subclasses may override.
"""

from . import _handles, cbook
from ._native import artist as _artist


class Artist:
    """
    Abstract base class for objects that render into a FigureCanvas.

    Typically, all visible elements in a figure are subclasses of Artist.
    """

    def __init__(self):
        raise NotImplementedError(
            "pltrs cannot make an artist outside a figure yet; make it with "
            "a plotting method instead")

    @classmethod
    def _from_native(cls, native, figure):
        """The object of class *cls* around the native handle *native* of
        an artist of *figure* (see `._handles.wrap`)."""
        artist = cls.__new__(cls)
        artist._native = native
        artist._figure = figure
        return artist

    @property
    def figure(self):
        """The (root) `.Figure` the artist belongs to."""
        return self._figure

    def get_figure(self, root=False):
        """Return the `.Figure` instance the artist belongs to."""
        return self._figure

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

    def get_zorder(self):
        """Return the artist's zorder."""
        return _artist.get_zorder(self._native)

    def set_zorder(self, level):
        """Set the zorder for the artist.  Artists with lower zorder values
        are drawn first."""
        _artist.set_zorder(self._native, level)

    @property
    def zorder(self):
        """The drawing order."""
        return self.get_zorder()

    def get_alpha(self):
        """Return the alpha value used for blending - not supported on all
        backends."""
        return _artist.get_alpha(self._native)

    def set_alpha(self, alpha):
        """Set the alpha value used for blending - not supported on all
        backends."""
        _artist.set_alpha(self._native, alpha)

    def get_label(self):
        """Return the label used for this artist in the legend."""
        return _artist.get_label(self._native)

    def set_label(self, s):
        """Set a label that will be displayed in the legend."""
        _artist.set_label(self._native, str(s) if s is not None else "")

    def remove(self):
        """Remove the artist from the figure if possible."""
        _artist.remove(self._native)
        _handles.forget(self)

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
