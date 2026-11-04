"""
What an Axes holds, clearing it, and the callbacks of its limits
(matplotlib's ``_children``, ``get_children``, ``clear``, ``cla`` and
``callbacks``).

pltrs (Rust) clears the axes and keeps the limit callbacks: it tells the
Axes' `.CallbackRegistry` when its view limits change, through a function
that holds the Axes weakly, so that no reference cycle runs through Rust.
The ``clear``/``cla`` dispatch and ``get_children`` are matplotlib
3.11.2's, under the Matplotlib License.
"""

import functools
import weakref

from .. import _api, _handles, cbook
from .._native import artist as _artist, axes as _axes
from ._artist_list import ArtistList


def _notify(axes_ref, signal):
    """Process *signal* of the Axes *axes_ref* refers to, if it is alive."""
    ax = axes_ref()
    if ax is not None:
        ax.callbacks.process(signal, ax)


class ClearMixin:
    """``clear``, ``cla``, the children and the limit callbacks of
    `~.axes.Axes`."""

    #: Whether a subclass overrides ``cla`` rather than ``clear``.
    _subclass_uses_cla = False

    def __init_subclass__(cls, **kwargs):
        parent_uses_cla = super(cls, cls)._subclass_uses_cla
        if 'cla' in cls.__dict__:
            _api.warn_deprecated(
                '3.6',
                pending=True,
                message=f'Overriding `Axes.cla` in {cls.__qualname__} is '
                'pending deprecation in %(since)s and will be fully '
                'deprecated in favor of `Axes.clear` in the future. '
                'Please report '
                f'this to the {cls.__module__!r} author.')
        cls._subclass_uses_cla = 'cla' in cls.__dict__ or parent_uses_cla
        super().__init_subclass__(**kwargs)

    def __getstate__(self):
        # The registry's native connection does not survive pickling; a
        # new one connects on first use.
        state = super().__getstate__()
        state.pop("_limit_callbacks", None)
        return state

    def __clear(self):
        """Clear the Axes."""
        figure = self.figure
        # matplotlib's clear forgets the legend without taking it out of
        # the figure (pandas reads the title of the one it replots).
        legend = self.legend_
        for native in _axes.clear(self._native):
            artist = figure._artists.pop(_artist.id(native), None)
            if artist is not None and artist is not legend:
                # Out of the figure, as matplotlib's clear leaves them.
                artist._figure = None
        self.__dict__.pop("_containers", None)
        self.__dict__.pop("_limit_callbacks", None)
        # A new background patch in the Axes' face colour, and new titles,
        # as matplotlib's clear makes them.
        self.__dict__.pop("_patch", None)
        for loc in ("center", "left", "right"):
            figure._artists.pop(_artist.id(_axes.title_text(self._native, loc)), None)
        from ..projections import _program
        _program.cleared(self)
        if "_facecolor" in self.__dict__:
            self.patch.set_facecolor(self._facecolor)

    def clear(self):
        """Clear the Axes."""
        # Act as an alias, or as the superclass implementation depending on the
        # subclass implementation.
        if self._subclass_uses_cla:
            self.cla()
        else:
            self.__clear()

    def cla(self):
        """Clear the Axes."""
        # Act as an alias, or as the superclass implementation depending on the
        # subclass implementation.
        if self._subclass_uses_cla:
            self.__clear()
        else:
            self.clear()

    @property
    def callbacks(self):
        """
        The `.CallbackRegistry` of the Axes: its ``'xlim_changed'`` and
        ``'ylim_changed'`` callbacks are called with the Axes whenever its
        view limits change.
        """
        registry = self.__dict__.get("_limit_callbacks")
        if registry is None:
            registry = self.__dict__["_limit_callbacks"] = cbook.CallbackRegistry(
                signals=["xlim_changed", "ylim_changed", "zlim_changed"])
            axes_ref = weakref.ref(self)
            for name in "xy":
                _axes.on_limits_changed(
                    self._native, name,
                    functools.partial(_notify, axes_ref, f"{name}lim_changed"))
        return registry

    @property
    def _children(self):
        """The artists added to the Axes, in the order they were added; an
        artist pltrs (Rust) makes of several (a contour set) once."""
        children = {id(child): child for child in (
            _handles.wrap_any(native, self.figure)
            for native in _axes.children(self._native))}
        return list(children.values())

    @property
    def artists(self):
        """The artists of the Axes that are no line, patch, collection,
        text, image or table, in the order they were added."""
        return ArtistList("artists", lambda: [
            _handles.wrap_any(native, self.figure)
            for native in _axes.artists(self._native)])

    def get_children(self):
        """Return a list of the child `.Artist`\\s of this `.Artist`."""
        return [
            *self._children,
            *self.spines.values(),
            *[getattr(self, f"{name}axis") for name in self._axis_names],
            self.title, self._left_title, self._right_title,
            *self.child_axes,
            *([self.legend_] if self.legend_ is not None else []),
            self.patch,
        ]
