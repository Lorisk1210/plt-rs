"""
What changes the drawing of every artist (matplotlib's
``Artist.set_path_effects`` and ``Artist.set_agg_filter``): pltrs (Rust)
draws the path effects of `.patheffects`, which describe themselves to it
(``AbstractPathEffect._native_spec``), and calls an agg filter function
with what the artist drew.

matplotlib keeps whatever the setters are given and uses it when it
draws: so does pltrs. Any other object in a list of path effects is
drawn as an effect of the program's, whose ``draw_path`` its draw calls
(and fails on, as matplotlib's does, when the object has none); a value
that is no list fails as matplotlib's iteration over it does, once a path
is drawn. Any filter other than None is called when the artist is drawn
into pixels, by the artists whose matplotlib ``draw`` applies filters
(``allow_rasterization`` wraps it, or the ``draw`` of a class it calls):
not offset boxes and annotation boxes, fancy arrows and subfigures
(`_UNFILTERED`).
"""

import collections
import contextlib
import functools
import weakref

from . import _program_classes
from ._native import artist as _artist
from ._native import patheffects as _patheffects


class EffectsMixin:
    """The path effect and image filter methods of `.Artist`."""

    def set_path_effects(self, path_effects):
        """
        Set the path effects.

        Parameters
        ----------
        path_effects : list of `.AbstractPathEffect`
        """
        self._path_effects = path_effects
        if self._kept_only:
            return
        # None draws as no effect, as an empty list does in matplotlib.
        _patheffects.set_path_effects(self._native,
                                      _effect_specs(path_effects, self._figure or self))

    def get_path_effects(self):
        if "_path_effects" in self.__dict__:
            return self._path_effects
        if self._kept_only:
            return rc_effects()
        # matplotlib's artist keeps the list of the rcParam it was made
        # under, which pltrs (Rust) draws with.
        made_under = next((effects for effects in _rc_effects_known()
                           if _draws(self, effects)), None)
        if made_under is not None:
            return made_under
        return [] if _draws(self, []) else rc_effects()

    def set_agg_filter(self, filter_func):
        """
        Set the agg filter.

        Parameters
        ----------
        filter_func : callable
            A filter function, which takes a (m, n, depth) float array
            and a dpi value, and returns a (m, n, depth) array and two
            offsets from the bottom left corner of the image

            .. ACCEPTS: a filter function, which takes a (m, n, 3) float array
                and a dpi value, and returns a (m, n, 3) array and two offsets
                from the bottom left corner of the image
        """
        self._agg_filter = filter_func
        _artist.set_agg_filter(
            self._native, None if filter_func is None or not _draws_filtered(self)
            else held_filter(self, notes_failure=True))

    def get_agg_filter(self):
        """Return filter function to be used for agg filter."""
        return self.__dict__.get("_agg_filter")


#: The lists the ``path.effects`` rcParam was set to, the last last: an
#: artist made under one of them gives it as its effects
#: (`EffectsMixin.get_path_effects`).
_rc_effects_set = collections.deque(maxlen=16)


def rc_effects_set(effects):
    """Note that the ``path.effects`` rcParam is the list *effects* now."""
    _rc_effects_set.append(effects)


def _rc_effects_known():
    """The lists of path effects the ``path.effects`` rcParam was: the one
    rcParams handed the program (`.RcParams._lists`), then those it was
    set to, the last first."""
    from . import rcParams
    kept = rcParams._lists.get('path.effects')
    if kept is not None:
        yield kept[0]
    yield from reversed(_rc_effects_set)


def rc_effects():
    """The ``path.effects`` rcParam: the very list the program holds as
    it, or set it to, as matplotlib's holds one list, while that is its
    value."""
    from . import rcParams
    current = rcParams._get('path.effects')
    specs = _specs(current)
    return next((effects for effects in _rc_effects_known()
                 if _specs(effects) == specs), current)


def _specs(effects):
    """The descriptions of the path effects *effects*, None for objects
    that cannot describe themselves."""
    try:
        return [effect._native_spec() for effect in effects]
    except (AttributeError, TypeError):
        return None


def _draws(artist, effects):
    """Whether pltrs (Rust) draws *artist* through *effects*, a list of
    path effects."""
    specs = _specs(effects)
    return specs is not None and _patheffects.draws_path_effects(artist._native, specs)


#: The classes whose matplotlib ``draw`` applies no agg filter.
_UNFILTERED = frozenset({"FancyArrowPatch", "OffsetBox", "AnnotationBbox", "SubFigure",
                         "InsetIndicator"})


def _draws_filtered(artist):
    """Whether *artist*'s drawing goes through its agg filter, as
    matplotlib's ``allow_rasterization`` applies it."""
    return not any(cls.__name__ in _UNFILTERED and cls.__module__.startswith(__package__)
                   for cls in type(artist).__mro__)


class _Iterated:
    """Path effects given as what is no list of them: drawing a path with
    them iterates them, as matplotlib's ``PathEffectRenderer`` does, which
    raises what iterating the value raises."""

    def __init__(self, effects):
        self.effects = effects

    def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
        for effect in self.effects:
            effect.draw_path(renderer, gc, tpath, affine, rgbFace)


def _effect_specs(path_effects, figure=None):
    """What pltrs (Rust) draws *path_effects* with: each effect's own
    description, the effect itself (drawn through its ``draw_path``) for
    any other object, held for *figure* (`._program_classes.held`), and,
    for a value that is no list, one effect iterating it when drawing."""
    if not path_effects:
        return []
    try:
        effects = list(path_effects)
    except TypeError:
        effects = [_Iterated(path_effects)]
    specs = []
    for effect in effects:
        spec = effect._native_spec() if hasattr(effect, "_native_spec") else ("python", effect)
        if spec[0] == "python":
            spec = ("python", _program_classes.held(spec[1], figure))
        specs.append(spec)
    return specs


@contextlib.contextmanager
def applying_filter(artist):
    """While *artist*'s ``draw`` filters what it draws itself (matplotlib's
    `.allow_rasterization` around it), pltrs (Rust) draws it without its
    filter (see `filter_applied`)."""
    depth = artist.__dict__.get("_filter_applied", 0)
    artist.__dict__["_filter_applied"] = depth + 1
    try:
        yield
    finally:
        if depth:
            artist.__dict__["_filter_applied"] = depth
        else:
            del artist.__dict__["_filter_applied"]


def filter_applied(artist):
    """Whether a ``draw`` running now filters what *artist* draws, which
    pltrs (Rust) then draws without its own filter (`applying_filter`)."""
    return artist.__dict__.get("_filter_applied", 0) > 0


def held_filter(artist, notes_failure=False):
    """What pltrs (Rust) calls for the agg filter of *artist*: the filter
    *artist* holds (``_agg_filter``), reached through a weak reference, so
    that pltrs (Rust) keeps neither the filter nor what it refers to alive
    (a filter whose code reaches the figure would keep the figure, past
    what Python's collector sees). A failing filter is noted on *artist*
    if *notes_failure* (see `filtered`)."""
    return functools.partial(_filtered_by, weakref.ref(artist), notes_failure)


def _filtered_by(reference, notes_failure, image, dpi):
    artist = reference()
    if artist is None:
        return image, 0, 0
    # matplotlib's renderer hands the resolution as its figure holds it
    # (the int a program gave).
    figure = artist.get_figure(root=True)
    if figure is not None and figure.dpi == dpi:
        dpi = figure.dpi
    return filtered(artist.__dict__.get("_agg_filter"), image, dpi,
                    artist=reference if notes_failure else None)


def filtered(filter_func, image, dpi, *, artist=None):
    """What *filter_func*, an artist's agg filter, makes of *image*: called
    as matplotlib calls it, whatever it is. A filter that fails fails the
    draw of the artist *artist* refers to (if given), which the artist may
    note (``_filter_failed``: a host Axes keeps its parasites' children
    listed)."""
    try:
        return filter_func(image, dpi)
    except Exception:
        failed = None if artist is None else getattr(artist(), "_filter_failed", None)
        if failed is not None:
            failed()
        raise
