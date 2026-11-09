"""
Pickling figures and everything in them, as matplotlib's objects pickle
with the standard `pickle` (and `copy`) modules.

pltrs (Rust) holds what a figure draws, behind the native objects
(``_native.Native``) pltrs's classes keep, so a figure pickles as an
archive of its native state (``_native.archive``) and the Python objects
around it, which pickle as they always do:

* Every native object pickles through `copyreg`: the handle of an artist
  as the `_FigureState` of its figure and the artist's id there, a
  native value (a transform, a grid, a colorbar, a container) as an
  archive of its own.
* A `_FigureState` stands for the native state of one figure (or of the
  holder of an artist made outside any figure) in a pickle. It carries
  the archive. Unpickled, it is first an empty native figure, which the
  handles of its artists reach, then filled in place with the archive:
  so the objects of a pickle can refer to each other in any order.
* The Python objects the archive refers to (formatters and norms of
  Python classes, callables given to pltrs, figures reached by the
  handles inside a value) pickle beside the bytes, as `(role, object)`
  pairs.

`.Figure` pickles as matplotlib's does (``Figure.__getstate__`` and
``__setstate__``): without its canvas, which unpickling makes again, and
back into pyplot's figures when it was in them.
"""

import copyreg
import weakref

from ._native import Native
from ._native import archive as _archive
from ._native import artist as _artist


class _FigureState:
    """The native state of one figure in a pickle (see the module
    docstring)."""

    def __init__(self, native=None):
        # A handle of the figure when pickling; when unpickling, the root
        # of a new figure, filled by __setstate__.
        self._native = _archive.new_figure() if native is None else native
        self._restored = native is not None
        self._pending = []

    def __reduce__(self):
        return (_FigureState, (), _archive.archive_figure(self._native))

    def __setstate__(self, state):
        data, externals = state
        _archive.restore_figure(self._native, data, externals)
        self._restored = True
        pending, self._pending = self._pending, []
        for callback in pending:
            callback()

    def when_restored(self, callback):
        """Call *callback* once the native state is restored: at once if
        it is, else when it is."""
        if self._restored:
            callback()
        else:
            self._pending.append(callback)


#: The `_FigureState` of each figure being pickled, by figure: one per
#: figure, however many handles of it a pickle holds.
_STATES = weakref.WeakValueDictionary()


def state_of(native):
    """The `_FigureState` of the figure the artist handle *native* is in."""
    key = _archive.figure_key(native)
    state = _STATES.get(key)
    if state is None:
        state = _STATES[key] = _FigureState(native)
    return state


def _handle(state, artist_id):
    """The handle of artist *artist_id* in the figure of *state*."""
    return _archive.handle(state._native, artist_id)


def _value(kind, data, externals):
    """The native value of *kind* archived in *data*, with *externals*."""
    return _archive.restore_value(kind, data, externals)


def _reduce_native(native):
    """Pickle a native object: an artist's handle as its figure's state and
    its id, a native value as its archive."""
    if _archive.is_handle(native):
        return (_handle, (state_of(native), _artist.id(native)))
    return (_value, _archive.archive_value(native))


copyreg.pickle(Native, _reduce_native)


def program_hooks(figure):
    """
    What the hooks of the root *figure* follow, as a pickle keeps it
    since the hooks do not pickle: what locators of the program's place
    (`.axes._program_locators`) and the artists whose transforms the
    program computes (`._computed_transforms`).
    """
    from . import _computed_transforms
    from .axes import _program_locators
    return (_program_locators.followed(figure), _computed_transforms.tracked(figure))


def restore_program_hooks(figure, hooks):
    """Give the root *figure*, read back from a pickle, its hooks again
    (*hooks* from `program_hooks`), and connect the Axes of projections of
    the program's to their limits again."""
    from . import _computed_transforms
    from .axes import _program_locators
    from .projections import _program
    followed, tracked = hooks
    for owner in followed:
        _program_locators.follow(owner, figure)
    for artist in tracked:
        _computed_transforms.track(artist, figure)
    for artist in list(figure._artists.values()):
        if "_program_projection" in artist.__dict__:
            _program.connect(artist)


#: The key under which an object read back from a pickle, its state not
#: set yet, keeps what waits for its state (`when_built`).
WAITING = "_waiting_for_state"


def _built(obj):
    """Whether *obj*, read back from a pickle, has its state."""
    return any(key != WAITING for key in vars(obj))


def when_built(objects, callback):
    """
    Call *callback* once each of *objects*, read back from a pickle, has
    its state: at once if they all do, else when the last of them gets it
    (`.Artist.__setstate__`). A pickle of an artist reaches its figure from
    inside the artist's state, so that the figure's state is set before the
    artist's.
    """
    unbuilt = [obj for obj in objects if not _built(obj)]
    if not unbuilt:
        callback()
        return
    remaining = [len(unbuilt)]

    def one_built():
        remaining[0] -= 1
        if not remaining[0]:
            callback()

    for obj in unbuilt:
        vars(obj).setdefault(WAITING, []).append(one_built)


def restore_figure(cls, state):
    """A figure of class *cls* being unpickled: its attributes come from
    ``Figure.__setstate__`` once *state*, its native state, is read."""
    figure = cls.__new__(cls)
    figure._unpickling_state = state
    return figure
