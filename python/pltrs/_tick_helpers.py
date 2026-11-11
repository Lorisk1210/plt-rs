"""
The locators and formatters of an axis as Python objects (matplotlib's
``axis.major.locator``, ``get_major_formatter``, ``set_minor_locator``).

pltrs (Rust) keeps the tick helpers of every axis, shared between the axes
sharing it, and draws with them. Python refers to one through a token of
``_native.axis`` whose identity stays unique while the token lives; each
root figure keeps its helpers by identity (``figure._tick_helpers``), so
that an axis gives back the same object every time and the axes sharing it
the same one.

* A locator set from Python (`install`) of a class of ``ticker`` is
  installed natively from its spec (``locator._spec()``): pltrs (Rust)
  then ticks with the locator of that kind. Changing such a locator
  afterwards (``locator.set_params(...)``) installs it again in place,
  everywhere it is used (`changed`). Any other helper (formatters, which
  keep what they computed for the ticks drawn, subclasses, the date and
  category helpers) is installed through an adapter that pltrs (Rust)
  calls each time it ticks the axis, handing it its description of the
  axis, which the helper's `.Axis` answers from meanwhile
  (``Axis._drawn_info``).
* A helper pltrs (Rust) installed by itself (a scale's defaults, fixed
  ticks, units, polar axes) becomes the object of matplotlib's class its
  spec describes (`_from_spec`), or, without one, a view computing through
  its token.

The objects of the helpers pltrs (Rust) ticks with natively follow the
ticks of their axis (`._tick_state`): read after a draw, they hold what
matplotlib's would.
"""

import contextlib
import weakref

import numpy as np

from . import _tick_state
from ._native import axis as _axis
from ._native import register as _register


def _registry(figure):
    """The tick helpers of *figure* by identity: ``{id: (token, helper,
    axis)}``."""
    return vars(figure).setdefault("_tick_helpers", {})


def _axis_registry(axis):
    """The tick helpers of the root figure of *axis* (that of the subfigure
    its Axes are in), or of its Axes once that was removed from its figure
    (whose axis keeps its helpers)."""
    figure = axis.axes.get_figure(root=True)
    return _registry(axis.axes if figure is None else figure)


def get(axis, kind, minor):
    """The locator (*kind* ``"locator"``) or formatter (``"formatter"``) of
    the major or *minor* ticks of *axis*, as one object per helper."""
    helper_id, token = _axis.helper(axis.axes._native, axis.axis_name, kind,
                                    bool(minor))
    registry = _axis_registry(axis)
    entry = registry.get(helper_id) or _held_by_peers(axis, helper_id)
    if entry is not None:
        return entry[1]
    spec = _axis.describe_helper(token)
    helper = _from_spec(spec, token, axis, kind)
    helper.set_axis(_locator_axis(axis))
    _watch(helper, axis)
    native = spec is None or spec["kind"] != "python"
    if kind == "formatter" and spec is not None and native:
        _watch_native_formatter(helper, axis)
    registry[helper_id] = (token, helper, axis)
    if native:
        _tick_state.follow(helper, axis, helper_id)
    return helper


def _held_by_peers(axis, helper_id):
    """The entry of the helper *helper_id* of *axis* in the figure of an
    Axes of another figure sharing the axis, whose helpers it shares: the
    figure of *axis* keeps it too from then on."""
    from .axes import _sharing
    for peer in _sharing.across(axis.axes, axis.axis_name):
        entry = _registry(peer._figure).get(helper_id)
        if entry is not None:
            _axis_registry(axis)[helper_id] = entry
            return entry
    return None


def _locator_axis(axis):
    """The axis the locators and formatters of *axis* are bound to
    (matplotlib's ``locator.axis``, ``formatter.axis``): that of the axes
    they were set on, among the axes sharing it (the last of those a clear
    reset)."""
    figure = axis.axes.figure
    owner = _axis.locator_owner(axis.axes._native, axis.axis_name)
    if owner is None or figure is None:
        return axis
    from . import _handles
    return _handles.wrap(owner, type(axis.axes), figure)._axis_map[axis.axis_name]


def adopt(axis, helper, kind, minor):
    """Make *helper*, a locator (*kind* ``"locator"``) or formatter of the
    package's class that describes the one pltrs (Rust) installed on the
    major or *minor* ticks of *axis*, the object that one reads as."""
    installed = _axis.helper(axis.axes._native, axis.axis_name, kind, bool(minor))
    helper.set_axis(axis)
    _keep(axis, helper, installed, True)


def _spec_of(helper):
    """
    The spec the locator *helper* is installed natively from: that of an
    object of a class of ``ticker`` or of polar axes (not of a subclass of
    the program's, which may compute in Python), None for the others.

    Formatters are always installed through adapters: matplotlib's keep
    what they computed for the ticks drawn last (``offset``,
    ``get_offset()``), which programs read after drawing.
    """
    spec = getattr(helper, "_held_spec", None)
    package = __name__.rpartition(".")[0]
    native = {f"{package}.{name}" for name in _NATIVE_MODULES}
    if (spec is None or not _is_locator(helper)
            or type(helper).__module__ not in native):
        return None
    return spec()


def _watch(helper, axis):
    """Have changes to *helper*, natively installed on *axis*, or to the
    helpers it wraps install it again (see `check_native`)."""
    spec = _spec_of(helper)
    if spec is None:
        # Installed through an adapter now: no spec to compare with.
        helper.__dict__.pop("_native_spec", None)
        return
    object.__setattr__(helper, "_native_spec", spec)
    object.__setattr__(helper, "_native_axis", axis)
    for part in _parts(helper):
        object.__setattr__(part, "_native_root", helper)


def _watch_native_formatter(helper, axis):
    """
    Have the first change to *helper*, the object of a formatter pltrs
    (Rust) installed on *axis*, install it in place of that formatter
    (see `check_native`), as matplotlib's formatters change in place:
    through an adapter, which follows its later changes.
    """
    spec = getattr(helper, "_spec", None)
    if spec is None:
        return
    object.__setattr__(helper, "_native_spec", spec())
    object.__setattr__(helper, "_native_axis", axis)
    object.__setattr__(helper, "_native_root", helper)


#: The modules whose helpers pltrs (Rust) has: those of the others (dates,
#: categories, the program's) are Python's, and drive pltrs (Rust) through
#: adapters.
_NATIVE_MODULES = ("ticker", "projections.polar")


def install(axis, kind, minor, helper):
    """Install *helper* as the locator or formatter of the major or *minor*
    ticks of *axis* (matplotlib's ``set_major_locator``, ...)."""
    _install(axis, kind, minor, helper, None)


def _install(axis, kind, minor, helper, replacing):
    """Install *helper* on *axis*, in place of the helper of the token
    *replacing* when given (nothing happens when that one is in use
    nowhere), and register it."""
    install_native = (_axis.install_locator if kind == "locator"
                      else _axis.install_formatter)
    description = _description(kind, helper)
    installed = install_native(axis.axes._native, axis.axis_name, bool(minor),
                               *description, replacing)
    if installed is not None:
        _keep(axis, helper, installed, description[1])
    _ticked_by_program(axis, [description])


def _description(kind, helper):
    """How pltrs (Rust) installs *helper*: from its spec (``(spec, True)``)
    or through an adapter (``(adapter, False)``)."""
    spec = _spec_of(helper)
    return (spec, True) if spec is not None else (_adapter(kind, helper), False)


def _keep(axis, helper, installed, native):
    """Register *helper*, installed on *axis* with the identity and token
    *installed*, natively (else through an adapter)."""
    _watch(helper, axis)
    registry = _axis_registry(axis)
    _prune(registry)
    helper_id, token = installed
    registry[helper_id] = (token, helper, axis)
    if native:
        _tick_state.follow(helper, axis, helper_id)
    else:
        _tick_state.unfollow(helper)


def use_axis_info(axis, info, default_limits):
    """
    Give *axis* the tick helpers and label of the `.units.AxisInfo` *info*
    of a unit converter of the program's, and its *default_limits* (in
    axis numbers, or None), as matplotlib's ``Axis._update_axisinfo``
    does: helpers replace only those still the defaults, and stay counted
    as defaults.
    """
    slots = ((info.majloc, "locator"), (info.minloc, "locator"),
             (info.majfmt, "formatter"), (info.minfmt, "formatter"))
    for helper, _ in slots:
        if helper is not None:
            helper.set_axis(axis)
    descriptions = [None if helper is None else _description(kind, helper)
                    for helper, kind in slots]
    installed = _axis.use_axis_info(
        axis.axes._native, axis.axis_name, descriptions,
        None if info.label is None else str(info.label), default_limits)
    _ticked_by_program(axis, [d for d in descriptions if d is not None])
    for (helper, _), description, identity in zip(slots, descriptions,
                                                  installed):
        if identity is not None:
            _keep(axis, helper, identity, description[1])


def _ticked_by_program(axis, descriptions):
    """Have the figure of *axis* tick its axes as matplotlib does (see
    `._tick_updates`) once one of the helpers described by *descriptions*
    is installed through an adapter."""
    if any(not native for _, native in descriptions):
        from . import _tick_updates
        _tick_updates.install(axis.axes.figure)


def _parts(helper):
    """*helper* and the helpers it wraps (polar axes' ``base``; the
    ``base`` of a log locator is a number)."""
    from . import ticker
    yield helper
    base = getattr(helper, "base", None)
    if isinstance(base, ticker.TickHelper):
        yield from _parts(base)


def _prune(registry):
    """Forget the helpers of *registry* no axis uses any more."""
    for helper_id, (token, _, _) in list(registry.items()):
        if not _axis.helper_in_use(token):
            del registry[helper_id]


def changed(helper):
    """Install *helper*, natively installed and changed since, again in
    place wherever it is used (matplotlib's helpers change in place)."""
    registry = _axis_registry(helper._native_axis)
    for helper_id, (token, entry, entry_axis) in list(registry.items()):
        if entry is not helper:
            continue
        del registry[helper_id]
        kind = "locator" if _is_locator(helper) else "formatter"
        _install(entry_axis, kind, False, helper, token)


def _is_locator(helper):
    from . import ticker
    return isinstance(helper, ticker.Locator)


def check_native(helper):
    """Install *helper* again if its spec changed since it was natively
    installed (called when an attribute of a tick helper is set), unless
    it is catching up with the ticks of its axis (see `settled`)."""
    root = helper.__dict__.get("_native_root")
    if root is None or _tick_state.catching_up(root):
        return
    installed = root.__dict__.get("_native_spec")
    if installed is None:
        return
    if _is_locator(root):
        if _spec_of(root) != installed:
            changed(root)
    elif root._spec() != installed or not hasattr(root.axis, "_native_info"):
        # A formatter is installed through an adapter from now on.
        for name in ("_native_spec", "_native_root"):
            object.__delattr__(root, name)
        changed(root)


def settled(helper):
    """
    Count what *helper* computed catching up with the ticks of its axis
    (``ndivs`` an `.AutoMinorLocator` settled) as no change of its own: it
    computed it on the ticks pltrs (Rust) computed with the spec it is
    installed from, which stays its spec.
    """
    if "_native_spec" not in helper.__dict__:
        return
    spec = _spec_of(helper) if _is_locator(helper) else helper._spec()
    if spec is not None:
        object.__setattr__(helper, "_native_spec", spec)


# --- Python helpers pltrs (Rust) calls ---------------------------------------

@contextlib.contextmanager
def drawing(helper, info):
    """While pltrs (Rust) has *helper* tick an axis it describes with *info*:
    the helper's axis answers from the description."""
    axis = getattr(helper, "axis", None)
    previous = helper.__dict__.get("_drawn_axis")
    object.__setattr__(helper, "_drawn_axis", info)
    drawn = info is not None and hasattr(axis, "_drawn_info")
    if drawn:
        before, axis._drawn_info = axis._drawn_info, info
    try:
        yield helper
    finally:
        object.__setattr__(helper, "_drawn_axis", previous)
        if drawn:
            axis._drawn_info = before


class _Adapter:
    """What pltrs (Rust) calls for a Python helper, which it holds weakly so
    that pltrs (Rust) keeps no Python object alive."""

    def __init__(self, helper):
        self._helper = weakref.ref(helper)

    def helper(self):
        """The helper, or None once it is gone."""
        return self._helper()

    def _call(self, info, fallback, compute):
        helper = self._helper()
        if helper is None:
            return fallback
        with drawing(helper, info):
            return compute(helper)


class _LocatorAdapter(_Adapter):
    """The locator methods pltrs (Rust) calls."""

    def locate(self, info):
        return self._call(info, [], lambda locator: _floats(locator()))

    def tick_values(self, vmin, vmax, info):
        return self._call(info, [], lambda locator: _floats(
            locator.tick_values(vmin, vmax)))

    def nonsingular(self, v0, v1, info):
        return self._call(info, (v0, v1), lambda locator: tuple(
            map(float, locator.nonsingular(v0, v1))))

    def view_limits(self, dmin, dmax, info):
        return self._call(info, (dmin, dmax), lambda locator: tuple(
            map(float, locator.view_limits(dmin, dmax))))

    def date_unit(self, info):
        """The days per tick unit of a date locator (``_get_unit``), None
        for other locators."""
        def unit(locator):
            get_unit = getattr(locator, "_get_unit", None)
            return None if get_unit is None else float(get_unit())
        return self._call(info, None, unit)


class _FormatterAdapter(_Adapter):
    """The formatter methods pltrs (Rust) calls."""

    def label_axis(self, locs, info):
        """The labels of the ticks at *locs* and the offset text, asked for
        as matplotlib's Axis does (`format_ticks`, then `get_offset`)."""
        def label(formatter):
            labels = formatter.format_ticks(locs)
            # Tick labels take any object, as Text.set_text does.
            return (['' if text is None else str(text) for text in labels],
                    str(formatter.get_offset()))
        return self._call(info, ([''] * len(locs), ''), label)

    def format_data(self, value):
        return self._call(None, '', lambda formatter: str(
            formatter.format_data(value)))

    def format_data_short(self, value):
        return self._call(None, '', lambda formatter: str(
            formatter.format_data_short(value)))


def _adapter(kind, helper):
    return (_LocatorAdapter if kind == "locator" else _FormatterAdapter)(helper)


def _floats(values):
    return [float(v) for v in np.asarray(values, dtype=float).ravel()]


# --- Helpers pltrs (Rust) installed ------------------------------------------

def _from_spec(spec, token, axis, kind):
    """The Python object of the helper of *token* on *axis*, described by
    *spec*."""
    if spec is None:
        ticker = _ticker()
        return (ticker._NativeLocatorView if kind == "locator"
                else ticker._NativeFormatterView)(token)
    if spec["kind"] == "python":
        return spec["adapter"].helper()
    table = _LOCATORS if kind == "locator" else _FORMATTERS
    return table[spec["kind"]](spec, axis)


def _locator_of(spec, axis):
    """The locator a (nested) locator spec describes."""
    return _LOCATORS[spec["kind"]](spec, axis)


def _ticker():
    from . import ticker
    return ticker


def _log_locator(spec, axis):
    return _ticker().LogLocator(spec["base"], spec["subs"],
                                numticks=spec["numticks"])


def _symlog_locator(spec, axis):
    locator = _ticker().SymmetricalLogLocator(
        subs=spec["subs"], linthresh=spec["linthresh"], base=spec["base"])
    locator.numticks = spec["numticks"]
    return locator


def _max_n_locator(spec, axis):
    return _ticker().MaxNLocator(
        nbins=spec["nbins"], steps=spec["steps"], integer=spec["integer"],
        symmetric=spec["symmetric"], prune=spec["prune"],
        min_n_ticks=spec["min_n_ticks"])


def _polar(name):
    from .projections import polar
    return getattr(polar, name)


def _geo_theta_formatter(round_to):
    from .projections import geo
    return geo.GeoAxes.ThetaFormatter(round_to)


def _category(name, axis):
    from . import category
    units = axis.units
    return getattr(category, name)(units._mapping if units is not None else {})


def _date_zone(axis):
    """The zone of the date helpers pltrs (Rust) gave *axis*: its unit
    (matplotlib's converters make them with ``tz=unit``), when a zone."""
    import datetime
    units = axis.units
    return units if isinstance(units, datetime.tzinfo) else None


def _date_locator(spec, axis):
    from . import dates
    return dates.AutoDateLocator(tz=_date_zone(axis))


def _date_formatter(cls_name):
    def make(spec, axis):
        from . import dates
        return getattr(dates, cls_name)(axis.get_major_locator(),
                                        tz=_date_zone(axis))
    return make


_LOCATORS = {
    "null": lambda spec, axis: _ticker().NullLocator(),
    "auto": lambda spec, axis: _ticker().AutoLocator(),
    "maxn": _max_n_locator,
    "logit": lambda spec, axis: _ticker().LogitLocator(
        spec["minor"], nbins=spec["nbins"]),
    "fixed": lambda spec, axis: _ticker().FixedLocator(
        spec["locs"], spec["nbins"]),
    "index": lambda spec, axis: _ticker().IndexLocator(
        spec["base"], spec["offset"]),
    "linear": lambda spec, axis: _ticker().LinearLocator(spec["numticks"]),
    "multiple": lambda spec, axis: _ticker().MultipleLocator(
        spec["base"], spec["offset"]),
    "log": _log_locator,
    "symlog": _symlog_locator,
    "asinh": lambda spec, axis: _ticker().AsinhLocator(
        spec["linear_width"], spec["numticks"], spec["symthresh"],
        spec["base"], spec["subs"]),
    "autominor": lambda spec, axis: _ticker().AutoMinorLocator(spec["n"]),
    "theta": lambda spec, axis: _polar("ThetaLocator")(
        _locator_of(spec["base"], axis)),
    "radial": lambda spec, axis: _polar("RadialLocator")(
        _locator_of(spec["base"], axis)),
    "category": lambda spec, axis: _category("StrCategoryLocator", axis),
    "date_auto": _date_locator,
}


def _scalar_formatter(spec, axis):
    formatter = _ticker().ScalarFormatter(
        useOffset=spec["use_offset"], useMathText=spec["use_mathtext"],
        usetex=spec["usetex"], useLocale=spec["locale"] is not None)
    formatter.set_scientific(spec["scientific"])
    formatter.set_powerlimits(spec["powerlimits"])
    return formatter


def _log_formatter(cls_name):
    def make(spec, axis):
        return getattr(_ticker(), cls_name)(
            base=spec["base"], labelOnlyBase=spec["label_only_base"],
            minor_thresholds=spec["minor_thresholds"],
            linthresh=spec["linthresh"])
    return make


def _date(spec, axis):
    from . import dates
    return dates.DateFormatter(spec["fmt"])


_FORMATTERS = {
    "null": lambda spec, axis: _ticker().NullFormatter(),
    "fixed": lambda spec, axis: _ticker().FixedFormatter(spec["seq"]),
    "format_str": lambda spec, axis: _ticker().FormatStrFormatter(spec["fmt"]),
    "str_method": lambda spec, axis: _ticker().StrMethodFormatter(spec["fmt"]),
    "scalar": _scalar_formatter,
    "log": _log_formatter("LogFormatter"),
    "log_exponent": _log_formatter("LogFormatterExponent"),
    "log_mathtext": _log_formatter("LogFormatterMathtext"),
    "log_scinotation": _log_formatter("LogFormatterSciNotation"),
    "logit": lambda spec, axis: _ticker().LogitFormatter(
        use_overline=spec["use_overline"], one_half=spec["one_half"],
        minor=spec["minor"], minor_threshold=spec["minor_threshold"],
        minor_number=spec["minor_number"]),
    "eng": lambda spec, axis: _ticker().EngFormatter(
        unit=spec["unit"], places=spec["places"], sep=spec["sep"]),
    "percent": lambda spec, axis: _ticker().PercentFormatter(
        xmax=spec["xmax"], decimals=spec["decimals"]),
    "theta": lambda spec, axis: _polar("ThetaFormatter")(),
    "geo_theta": lambda spec, axis: _geo_theta_formatter(spec["round_to"]),
    "category": lambda spec, axis: _category("StrCategoryFormatter", axis),
    "date_auto": _date_formatter("AutoDateFormatter"),
    "date_concise": _date_formatter("ConciseDateFormatter"),
    "date": _date,
}


# --- Pickling ----------------------------------------------------------------

#: The adapter pltrs (Rust) calls a Python helper through, which unpickling
#: makes again for the helper read back (see `._pickling`).
_register("tick_helper_adapter", _adapter)


def pickled(figure):
    """
    The tick helpers of *figure* as a pickle keeps them, since the tokens
    of its registry do not pickle: ``[(axis, kind, minor, helper)]`` for
    each helper in use, by an axis it is the major or minor helper of.
    """
    uses = []
    for helper_id, (_, helper, axis) in _registry(figure).items():
        kind = "locator" if _is_locator(helper) else "formatter"
        for minor in (False, True):
            in_use, _ = _axis.helper(axis.axes._native, axis.axis_name, kind,
                                     minor)
            if in_use == helper_id:
                uses.append((axis, kind, minor, helper))
                break
    return uses


def unpickled(figure, uses):
    """Register the tick helpers *uses* of *figure*, read back from a pickle
    (`pickled`), with the tokens of its axes: an axis gives back the same
    objects, which keep the helpers its adapters hold weakly alive."""
    registry = _registry(figure)
    for axis, kind, minor, helper in uses:
        helper_id, token = _axis.helper(axis.axes._native, axis.axis_name,
                                        kind, minor)
        registry[helper_id] = (token, helper, axis)
        if "_ticked_on" in helper.__dict__:
            _tick_state.follow(helper, axis, helper_id)
    # The axes read back with helpers or scales of the program's are
    # ticked as matplotlib ticks them.
    from . import _tick_updates
    _tick_updates.install(figure)
