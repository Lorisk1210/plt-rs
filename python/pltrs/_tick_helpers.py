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
"""

import contextlib
import weakref

import numpy as np

from ._native import axis as _axis


def _registry(figure):
    """The tick helpers of *figure* by identity: ``{id: (token, helper,
    axis)}``."""
    return vars(figure).setdefault("_tick_helpers", {})


def get(axis, kind, minor):
    """The locator (*kind* ``"locator"``) or formatter (``"formatter"``) of
    the major or *minor* ticks of *axis*, as one object per helper."""
    helper_id, token = _axis.helper(axis.axes._native, axis.axis_name, kind,
                                    bool(minor))
    registry = _registry(axis.axes.figure)
    entry = registry.get(helper_id)
    if entry is not None:
        return entry[1]
    helper = _from_spec(_axis.describe_helper(token), token, axis, kind)
    helper.set_axis(axis)
    _watch(helper, axis)
    registry[helper_id] = (token, helper, axis)
    return helper


def _spec_of(helper):
    """
    The spec the locator *helper* is installed natively from: that of an
    object of a class of ``ticker`` or of polar axes (not of a subclass of
    the program's, which may compute in Python), None for the others.

    Formatters are always installed through adapters: matplotlib's keep
    what they computed for the ticks drawn last (``offset``,
    ``get_offset()``), which programs read after drawing.
    """
    spec = getattr(helper, "_spec", None)
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
        return
    object.__setattr__(helper, "_native_spec", spec)
    object.__setattr__(helper, "_native_axis", axis)
    for part in _parts(helper):
        object.__setattr__(part, "_native_root", helper)


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
    spec = _spec_of(helper)
    native = spec is not None
    description = spec if native else _adapter(kind, helper)
    install_native = (_axis.install_locator if kind == "locator"
                      else _axis.install_formatter)
    installed = install_native(axis.axes._native, axis.axis_name, bool(minor),
                               description, native, replacing)
    if installed is None:
        return
    _watch(helper, axis)
    registry = _registry(axis.axes.figure)
    _prune(registry)
    helper_id, token = installed
    registry[helper_id] = (token, helper, axis)


def _parts(helper):
    """*helper* and the helpers it wraps (polar axes' ``base``)."""
    yield helper
    base = getattr(helper, "base", None)
    if base is not None:
        yield from _parts(base)


def _prune(registry):
    """Forget the helpers of *registry* no axis uses any more."""
    for helper_id, (token, _, _) in list(registry.items()):
        if not _axis.helper_in_use(token):
            del registry[helper_id]


def changed(helper):
    """Install *helper*, natively installed and changed since, again in
    place wherever it is used (matplotlib's helpers change in place)."""
    registry = _registry(helper._native_axis.axes.figure)
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
    installed (called when an attribute of a tick helper is set)."""
    root = helper.__dict__.get("_native_root")
    if root is None:
        return
    installed = root.__dict__.get("_native_spec")
    if installed is not None and _spec_of(root) != installed:
        changed(root)


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


def _category(name, axis):
    from . import category
    units = axis.units
    return getattr(category, name)(units._mapping if units is not None else {})


def _date_locator(spec, axis):
    from . import dates
    return dates.AutoDateLocator()


def _date_formatter(cls_name):
    def make(spec, axis):
        from . import dates
        return getattr(dates, cls_name)(axis.get_major_locator())
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
        usetex=spec["usetex"])
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
    "category": lambda spec, axis: _category("StrCategoryFormatter", axis),
    "date_auto": _date_formatter("AutoDateFormatter"),
    "date_concise": _date_formatter("ConciseDateFormatter"),
    "date": _date,
}
