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

from .._program_classes import takes_from_program
from .. import _api, _handles, cbook
from .._native import artist as _artist, axes as _axes
from .. import _lazy_imports
from . import _sharing
from ._artist_list import ArtistList

_foreign_children = _lazy_imports.module(globals(), "._foreign_children")


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
        # The registry pickles as matplotlib's does (its signals, the
        # callbacks it may pickle); its native connection does not, and is
        # made again on first use.
        state = super().__getstate__()
        state.pop("_limit_callbacks_connected", None)
        return _sharing.with_peers(self, state)

    def __clear(self):
        """Clear the Axes."""
        figure = self._figure
        # matplotlib's clear forgets the legend without taking it out of
        # the figure (pandas reads the title of the one it replots).
        legend = self.legend_
        if legend is not None:
            # Which keeps the Axes as its own.
            legend._cleared_from = self
        for child in self.child_axes:
            child._leave_host()
        self._decouple_children()
        for axis in self._axis_map.values():
            axis._retire_ticks()
        for native in self._clear_natively():
            artist = figure._artists.get(_artist.id(native))
            if artist is not None and "_child_of" in artist.__dict__:
                # Still of the figure and of the Axes, as matplotlib's clear
                # leaves the child Axes it forgets.
                continue
            figure._artists.pop(_artist.id(native), None)
        _foreign_children.forget(self)
        self._forget_cycles()
        for spine in self.spines.values():
            spine._clear()
        # Stale, without units, ticks and callbacks (the Axes' handler of
        # its units too), as matplotlib's clear of each axis leaves it.
        for axis in self._axis_map.values():
            if axis._held:
                # matplotlib's clear of the axis, at the z-order the Axes
                # gives its axes again.
                axis.clear()
                axis.set_zorder(axis._AXISBELOW_ZORDER[self.get_axisbelow()])
                continue
            axis._forget_units()
            axis.reset_ticks()
            # matplotlib's clear of the axis made the first tick of each level.
            axis._ticks_seen = {False: 1, True: 1}
            axis.callbacks = cbook.CallbackRegistry(signals=["units"])
            axis.stale = True
        from .. import rcParams
        self._gridOn = rcParams['axes.grid']
        if takes_from_program(type(self), "grid"):
            # matplotlib's clear turns the grid off, then as the rcParams
            # say, through the class' own `grid`.
            self.grid(False)
            self.grid(self._gridOn, which=rcParams['axes.grid.which'],
                      axis=rcParams['axes.grid.axis'])
        self.__dict__.pop("_containers", None)
        self.__dict__.pop("_limit_callbacks", None)
        self.__dict__.pop("_limit_callbacks_connected", None)
        # A new background patch in the Axes' face colour, and new titles,
        # as matplotlib's clear makes them.
        self.__dict__.pop("_patch", None)
        for loc in ("center", "left", "right"):
            figure._artists.pop(_artist.id(_axes.title_text(self._native, loc)), None)
        from ..projections import _program
        _program.cleared(self)
        if "_facecolor" in self.__dict__:
            self.patch.set_facecolor(self._facecolor)
        if type(self).callbacks is not ClearMixin.callbacks:
            # A registry of the class' own hears of the limits at once.
            self._hear_limit_changes()
        # The patch matplotlib's first clear makes exists from here on
        # (see `.._held_axis.Axis.reset_ticks`).
        self._cleared_once = True
        # matplotlib's clear ends with the limits of each axis no Axes
        # shares (pltrs (Rust) set those of the axes it holds).
        for name, axis in self._axis_map.items():
            if axis is None or not axis._held:
                continue
            share = getattr(self, f"_share{name}")
            if share is not None:
                getattr(self, f"share{name}")(share)
            else:
                axis._set_lim(0, 1, auto=True)

    @property
    def _gridOn(self):
        # matplotlib's attribute, which Axes classes of libraries read in
        # their own ``grid`` (mplstereonet's): :rc:`axes.grid` as the last
        # clear of the Axes read it (as it is now for an Axes pltrs (Rust)
        # made without a clear of this class).
        from .. import rcParams
        return self.__dict__.get("_grid_on_when_cleared", rcParams['axes.grid'])

    @_gridOn.setter
    def _gridOn(self, on):
        self.__dict__["_grid_on_when_cleared"] = on

    def _clear_natively(self):
        """Have pltrs (Rust) clear the Axes; return the artists it took
        out (see `_axes.clear`)."""
        return _axes.clear(self._native)

    def _decouple_children(self):
        """Decouple the children from the figure and the Axes, as
        matplotlib's clear leaves them, before pltrs (Rust) takes them
        out."""
        for native in _axes.children(self._native):
            artist = self._figure._artists.get(_artist.id(native))
            if artist is not None:
                artist._decouple(removed=False)

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
        self._hear_limit_changes()
        return registry

    @callbacks.setter
    def callbacks(self, registry):
        # The registry told of limit changes is the one the attribute holds
        # then (`_notify`), as matplotlib's axes tell theirs.
        self.__dict__["_limit_callbacks"] = registry

    def _hear_limit_changes(self):
        """Have pltrs (Rust) tell the Axes' ``callbacks`` of the changes of
        its view limits (once, until the Axes is cleared). The registry is
        read where a change is told, so that a class of the program's
        keeping ``callbacks`` as a property of its own (mpltern's
        ``TernaryAxes``) is told through that."""
        if not self.__dict__.get("_limit_callbacks_connected"):
            self.__dict__["_limit_callbacks_connected"] = True
            axes_ref = weakref.ref(self)
            for name in self._axis_names:
                self._on_limits_changed(
                    name, functools.partial(_notify, axes_ref, f"{name}lim_changed"))

    def _on_limits_changed(self, name, notify):
        """Have pltrs (Rust) call *notify* whenever the view limits of the
        axis *name* change (the x and y axes: pltrs (Rust) keeps no limits
        of the other axes a program's projection class makes)."""
        if name in ("x", "y"):
            _axes.on_limits_changed(self._native, name, notify)

    @property
    def _children(self):
        """The artists added to the Axes, in the order they were added; an
        artist pltrs (Rust) makes of several (a contour set) once."""
        children = {id(child): child for child in (
            _foreign_children.listed(_handles.wrap_any(native, self._figure))
            for native in _axes.children(self._native))}
        return [*children.values(), *_foreign_children.of(self)]

    @property
    def artists(self):
        return ArtistList._of("artists", lambda: [
            *(artist for artist in (_handles.wrap_any(native, self._figure)
                                    for native in _axes.artists(self._native))
              if not _foreign_children.is_stand_in(artist)),
            *_foreign_children.of(self)])

    def get_children(self):
        return [
            *self._children,
            *self.spines.values(),
            *[getattr(self, f"{name}axis") for name in self._axis_names],
            self.title, self._left_title, self._right_title,
            *self.child_axes,
            *([self.legend_] if self.legend_ is not None else []),
            self.patch,
        ]
