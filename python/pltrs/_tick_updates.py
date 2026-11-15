"""
matplotlib's ``Axis._update_ticks`` with the program's tick helpers, for
pltrs (Rust).

pltrs (Rust) computes the ticks of its own helpers itself. An axis whose
locator, formatter or scale is Python's (a class of the program's, a
`FuncFormatter`, a function scale, the date and category helpers set from
Python, a secondary axis, the colorbar of a norm of the program's or of a
`FuncNorm`) is ticked here instead, as matplotlib's `Axis` ticks it, wherever matplotlib
does: pltrs (Rust) models matplotlib's call sites (`Axis.draw`,
``get_tightbbox``, the label and title placement, the layout engines'
passes, the draw before a print), so that the program's code is called as
often, in the same order and with the same arguments. pltrs (Rust) lends
the figure meanwhile: the code may read it (limits, ticks, positions) and
change it, as matplotlib's may.

Each root figure with such an axis has an `_Updater` (`install`), which
pltrs (Rust) calls with the axes and the name of the axis to tick and its
description of the axis there (an ``AxisInfo``, which the axis and its
helpers answer from meanwhile, see ``Axis._drawn_info``).

An axis object the program gave an ``_update_ticks`` of its own
(`follow_program_update`: ultraplot's tick cache wraps the axis' own for
the length of a draw) is ticked here too, through it: pltrs' own
``Axis._update_ticks`` answers its calls of the original (`own_update`),
and the ticks it returns are those drawn.
"""

import collections
import contextlib
import warnings
import weakref

import numpy as np

from ._native import axis as _axis


#: The root figures whose updater is installed (pltrs (Rust) holds it).
_INSTALLED = weakref.WeakSet()

#: The last levels pltrs' own ``_update_ticks`` gave each axis ticked
#: through an ``_update_ticks`` of the program's (`own_update`), each with
#: the ids of the list of ticks it returned and of the ticks in it: a list
#: the program's update returns stands for the levels it was made for,
#: even one kept from an earlier update (ultraplot's tick cache). Ids
#: only, so that nothing here holds the axis' ticks.
_OWN_LEVELS = weakref.WeakKeyDictionary()

#: How many updates `_OWN_LEVELS` remembers per axis.
_OWN_LEVELS_KEPT = 8


def install(owner):
    """Have the axes of the root figure of *owner* (a figure, or Axes, also
    removed ones, which keep their figure) whose helpers or scale are
    Python's ticked as matplotlib ticks them (once per figure)."""
    from ._live_transforms import root_figure
    root = root_figure(owner)
    if root is None or root in _INSTALLED:
        return
    _INSTALLED.add(root)
    _axis.set_tick_updater(root._native, _Updater(root))


def follow_program_update(axis):
    """Have pltrs (Rust) tick *axis* through the ``_update_ticks`` the
    program gave the object (see `_program_update`), from now on."""
    from .axis import Axis
    if not isinstance(axis, Axis) or axis.axis_name not in ("x", "y"):
        return
    _axis.set_program_update(*axis._args(), True)
    install(axis.axes)


#: The tick labels of each axis given the position setter of the
#: program's that matplotlib's tick update calls with the tick's location
#: (`follow_label_setter`), held weakly.
_LABEL_SETTERS = weakref.WeakKeyDictionary()


def follow_label_setter(axis, label):
    """Have each update of *axis* at matplotlib's call sites call the
    position setter of the program's that its tick label *label* was given
    (``set_x`` of an x tick's label, ``set_y`` of a y tick's: pygtc shifts
    its tick labels so), as matplotlib's ``Tick.update_position`` does,
    and draw the label where it puts it."""
    from .axis import Axis
    if not isinstance(axis, Axis) or axis.axis_name not in ("x", "y"):
        return
    _LABEL_SETTERS.setdefault(axis, weakref.WeakSet()).add(label)
    _axis.set_program_update(*axis._args(), True)
    install(axis.axes.figure)


def _set_labels(axis, levels):
    """Call the position setters the program gave the labels of the ticks
    of *axis* with their ticks' locations in *levels* (an update's), as
    matplotlib's ``update_position`` does, and draw each label where its
    setter put it along the axis. A label whose tick is no longer one of
    the axis' (the axis cleared, its ticks made anew) is no longer
    followed, as matplotlib's update no longer reaches it."""
    from . import _mirrors
    labels = _LABEL_SETTERS.get(axis, ())
    along = 1 if axis.axis_name == "y" else 0
    setter = "set_y" if along else "set_x"
    for label in list(labels):
        tick = getattr(_mirrors.mirroring(label), "owner", None)
        if (tick is None or getattr(tick, "_tick_axis", None) is not axis
                or label not in (tick.label1, tick.label2)):
            labels.discard(label)
            continue
        level = levels[0 if tick._major else 1]
        if level is None or tick._index >= len(level[0]):
            continue
        loc = float(level[0][tick._index])
        getattr(label, setter)(loc)
        placed = float(label.get_position()[along])
        _axis.set_tick_label_along(*axis._args(), not tick._major, tick._index,
                                   label is tick.label2, None if placed == loc else placed)
    if not labels and "_update_ticks" not in vars(axis):
        _LABEL_SETTERS.pop(axis, None)
        _axis.set_program_update(*axis._args(), False)


class _Updater:
    """What pltrs (Rust) calls to tick an axis of its figure, which it
    holds weakly."""

    def __init__(self, figure):
        self._figure = weakref.ref(figure)

    def _axes(self, axes):
        """The Axes of the handle *axes*, or None once the figure is
        gone."""
        figure = self._figure()
        if figure is None:
            return None
        from . import _handles
        from .axes import Axes
        return _handles.wrap(axes, Axes, figure)

    def _axis(self, axes, name):
        """The `.Axis` named *name* of the Axes of the handle *axes*."""
        ax = self._axes(axes)
        return None if ax is None else ax._unit_axis_map.get(name)

    def spines(self, axes):
        """The spines of the Axes as matplotlib's code measures them: name,
        whether it is shown and laid out, and the name of the axis
        registered with it (None for none)."""
        ax = self._axes(axes)
        if ax is None:
            return []
        return [(str(name), bool(spine.get_visible() and spine.get_in_layout()),
                 getattr(spine.axis, "axis_name", None))
                for name, spine in ax.spines.items()]

    def update(self, axes, name, info, major, minor, quiet, called):
        """One ``_update_ticks`` of the axis; *major* and *minor* say which
        levels it shows. The locations, labels and drawn flags of each
        level computed, None for the others. A *quiet* update stands for
        none of matplotlib's (a hidden axis, which matplotlib never ticks),
        so numpy's warnings are not said, nor is a failure of the axis'
        helpers raised (no levels then) but for running out of stack or
        memory, which ends what recursed into it; one not *called* is
        pltrs' own (an ``_update_ticks`` the program gave the axis is not
        called)."""
        axis = self._axis(axes, name)
        if axis is None:
            return None, None
        with _ticking(axis, info), _quietly(quiet):
            update = (_program_update
                      if (called and "_update_ticks" in vars(axis)
                          and "_own_update" not in vars(axis))
                      else _update_ticks)
            if not quiet:
                levels = update(axis, info, major, minor)
                if called and axis in _LABEL_SETTERS:
                    _set_labels(axis, levels)
                return levels
            try:
                return update(axis, info, major, minor)
            except (RecursionError, MemoryError):
                raise
            except Exception:
                return None, None

    def locate(self, axes, name, info):
        """The locations matplotlib's ``Axis.get_children`` asks the axis'
        locators for, which a pick reaching the axis' children calls
        (``get_major_ticks``, then ``get_minor_ticks``): no formatter is
        called."""
        axis = self._axis(axes, name)
        if axis is None:
            return
        from ._tick_helpers import drawing
        with _ticking(axis, info):
            with drawing(axis.major.locator, info) as locator:
                locator()
            _minor_locs(axis, info)

    def inverted(self, axes, name):
        """Whether the axis is inverted, as its class answers (an axis of
        the program's class answering ``get_view_interval`` itself, which
        matplotlib's ``autoscale_view`` asks through ``set_xbound``)."""
        axis = self._axis(axes, name)
        return None if axis is None else bool(axis.get_inverted())

    def offset(self, axes, name, info):
        """The offset text of the axis' major formatter."""
        axis = self._axis(axes, name)
        if axis is None:
            return ""
        from ._tick_helpers import drawing
        with _ticking(axis, info), drawing(axis.major.formatter, info) as formatter:
            return str(formatter.get_offset())


@contextlib.contextmanager
def _quietly(quiet):
    """While numpy's runtime warnings are not said, if *quiet*."""
    if not quiet:
        yield
        return
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", RuntimeWarning)
        yield


@contextlib.contextmanager
def _ticking(axis, info):
    """While *axis* is ticked from the description *info*: it answers from
    it, and reads its major ticks from its locator, as matplotlib's does."""
    before = axis._drawn_info, vars(axis).get("_ticking", False)
    axis._drawn_info = info
    axis._ticking = True
    try:
        yield
    finally:
        axis._drawn_info, axis._ticking = before


def _program_update(axis, info, major_shown, minor_shown):
    """The ticks of *axis* through the ``_update_ticks`` the program gave
    it: those it returns are drawn, of the levels pltrs' own update gave
    last (which answers the program's calls of it, `own_update`)."""
    vars(axis)["_own_update"] = (info, major_shown, minor_shown)
    try:
        returned = axis._update_ticks()
    finally:
        vars(axis).pop("_own_update", None)
    chosen = {id(tick) for tick in returned or ()}
    levels = _levels_of(axis, returned, chosen)
    if levels is None:
        # Ticks pltrs never made: matplotlib draws what its own update
        # placed.
        return _update_ticks(axis, info, major_shown, minor_shown)
    made = axis.__dict__.get("_ticks", {})
    result = []
    for minor, level in zip((False, True), levels):
        if level is None:
            result.append(None)
            continue
        locs, labels, _ = level
        ticks = made.get(minor, [])
        result.append((locs, labels, [i < len(ticks) and id(ticks[i]) in chosen
                                      for i in range(len(locs))]))
    return tuple(result)


def _levels_of(axis, returned, chosen):
    """The levels the ticks *returned* (whose ids are *chosen*) were placed
    for: those of the update that returned that very list, else of the
    last one whose ticks include them all; None for none."""
    made = _OWN_LEVELS.get(axis, ())
    for listed, ticks, levels in reversed(made):
        if listed == id(returned) and ticks == tuple(map(id, returned)):
            return levels
    for _, ticks, levels in reversed(made):
        if chosen <= set(ticks):
            return levels
    return None


def own_update(axis, info, major_shown, minor_shown):
    """pltrs' ``Axis._update_ticks`` called by the program's while it
    ticks *axis* (`_program_update`): the ticks placed as matplotlib
    places them, the list of those drawn returned."""
    levels = _update_ticks(axis, info, major_shown, minor_shown)
    drawn = []
    for minor, level in zip((False, True), levels):
        if level is not None:
            locs, _, shown = level
            ticks = axis._tick_objects(minor, len(locs))[:len(locs)]
            drawn += [tick for tick, on in zip(ticks, shown) if on]
    made = _OWN_LEVELS.setdefault(axis, collections.deque(maxlen=_OWN_LEVELS_KEPT))
    made.append((id(drawn), tuple(map(id, drawn)), levels))
    return drawn


def _update_ticks(axis, info, major_shown, minor_shown):
    """matplotlib's ``Axis._update_ticks`` (3.11), the ticks placed by
    pltrs (Rust): each level computed, then which ticks are drawn."""
    from ._tick_helpers import drawing
    from .ticker import NullLocator
    levels = [None, None]
    if major_shown and not isinstance(axis.get_major_locator(), NullLocator):
        with drawing(axis.major.locator, info) as locator:
            locs = locator()
        levels[0] = (locs, _labels(axis.major.formatter, locs, info))
    if minor_shown and not isinstance(axis.get_minor_locator(), NullLocator):
        locs = _minor_locs(axis, info)
        levels[1] = (locs, _labels(axis.minor.formatter, locs, info))
    locs = [loc for level in levels if level is not None for loc in level[0]]
    drawn = _drawn(axis, locs) if locs else []
    result = []
    for level in levels:
        if level is None:
            result.append(None)
            continue
        values = _floats(level[0])
        result.append((values, level[1], drawn[:len(values)]))
        drawn = drawn[len(values):]
    return tuple(result)


def _labels(formatter, locs, info):
    """The labels *formatter* gives the ticks at *locs* (any object, as
    `.Text.set_text` takes)."""
    from ._tick_helpers import drawing
    with drawing(formatter, info):
        labels = formatter.format_ticks(locs)
    return ['' if text is None else str(text) for text in labels]


def _minor_locs(axis, info):
    """matplotlib's ``Axis.get_minorticklocs``: the minor locator's ticks
    without those on major ticks, unless the axis keeps them."""
    from ._tick_helpers import drawing
    locator = axis.minor.locator
    with drawing(locator, info):
        minor_locs = np.asarray(locator())
    if axis.remove_overlapping_locs:
        major_locs = axis.major.locator()
        transform = _transform(axis)
        tr_minor_locs = transform.transform(minor_locs)
        tr_major_locs = transform.transform(major_locs)
        lo, hi = sorted(transform.transform(axis.get_view_interval()))
        # Use the transformed view limits as scale.  1e-5 is the default
        # rtol for np.isclose.
        tol = (hi - lo) * 1e-5
        mask = np.isclose(tr_minor_locs[:, None], tr_major_locs[None, :],
                          atol=tol, rtol=0).any(axis=1)
        minor_locs = minor_locs[~mask]
    return minor_locs


def _drawn(axis, locs):
    """Whether each tick at *locs* is drawn: inside the view, through the
    axis' scale (the end of matplotlib's ``_update_ticks``)."""
    from .transforms import _interval_contains_close
    view_low, view_high = sorted(axis.get_view_interval())
    interval_t = _transform(axis).transform([view_low, view_high])
    drawn = []
    for loc in locs:
        try:
            loc_t = _transform(axis).transform(loc)
        except AssertionError:
            # transforms.transform doesn't allow masked values but
            # some scales might make them, so we need this try/except.
            drawn.append(False)
        else:
            drawn.append(bool(_interval_contains_close(interval_t, loc_t)))
    return drawn


def _transform(axis):
    """The transform of the scale of *axis*, where matplotlib's code asks
    the axis for it (``get_transform()``): that of the scale object it has
    (``axis._scale``), whose calls the program may observe, else pltrs
    (Rust)'s map of the scale of functions pltrs (Rust) gave it, which no
    object stands for (a colorbar's)."""
    from ._transform_trees import AxesTrees
    name = axis.get_scale()
    given = axis._given_scale
    if (name in ("function", "functionlog")
            and (given is None or given.name != name)
            and type(axis.axes)._default_axis_scale
            is AxesTrees._default_axis_scale):
        return _NativeScale(axis)
    return axis._scale.get_transform()


class _NativeScale:
    """The map of the scale pltrs (Rust) gave an axis, as a transform."""

    def __init__(self, axis):
        self._axis = axis

    def transform(self, values):
        return _axis.scaled(*self._axis._args(), np.asarray(values, float))


def _floats(values):
    return [float(v) for v in np.asarray(values, dtype=float).ravel()]
