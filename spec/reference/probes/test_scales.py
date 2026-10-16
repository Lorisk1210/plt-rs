"""Axis scales (``matplotlib.scale``): transforms, domain limits, defaults.

Area ``scales``.

Argument conventions
--------------------
* ``<Scale>`` is one of LinearScale, LogScale, SymmetricalLogScale,
  LogitScale, AsinhScale, FuncScale, FuncScaleLog. ``params`` holds its
  constructor keyword arguments verbatim (the scale is built without an axis).
* FuncScale/FuncScaleLog take ``functions`` as a *name* (the only non-JSON
  argument); the forward/inverse pairs are:

  ==============  =====================  ======================
  name            forward(x)             inverse(y)
  ==============  =====================  ======================
  ``"square"``    ``x ** 2``             ``sqrt(y)``
  ``"cube"``      ``x ** 3``             ``cbrt(y)``
  ``"log1p"``     ``log1p(x)``           ``expm1(y)``
  ``"affine"``    ``2 * x + 1``          ``(y - 1) / 2``
  ==============  =====================  ======================

* ``values`` is ``np.asarray(values, dtype=float)`` (1-D).
* Results keep NaN/inf; a ``LogScale(nonpositive='clip')`` maps values <= 0
  to -1000 and ``LogitScale(nonpositive='clip')`` maps <= 0 to -1000 and
  >= 1 to 1000.

Ops
---
``scale.get_scale_names``                  {} -> sorted list of scale names
``scale.scale_factory``                    {scale, params} -> class name of ``scale_factory(scale, None, **params)``
``scale.<Scale>.transform``                {params, values} -> ``get_transform().transform(values)``
``scale.<Scale>.inverted.transform``       {params, values} -> ``get_transform().inverted().transform(values)``
``scale.<Scale>.limit_range_for_scale``    {params, vmin, vmax, minpos} -> [vmin, vmax]
``scale.<Scale>.val_in_range``             {params, values} -> [bool, ...]
``scale.<Scale>.default_locators_and_formatters``
                                           {params, axis: "x" | "y", rc?} -> the major/minor
                                           locator and formatter an Axes installs for
                                           ``set_{axis}scale(<name>, **params)``; each is
                                           ``{"class": <name>, ...configured parameters}``:

  * MaxNLocator family (AutoLocator, LogitLocator): nbins, steps, integer,
    symmetric, prune, min_n_ticks (+ minor for LogitLocator)
  * LogLocator: base, subs ("auto"/"all"/list), numticks
  * SymmetricalLogLocator: base, linthresh, subs, numticks
  * AsinhLocator: linear_width, numticks, symthresh, base, subs
  * AutoMinorLocator: ndivs
  * ScalarFormatter: useOffset, offset, useMathText, useLocale, usetex,
    scientific, powerlimits, offset_threshold
  * LogFormatter family: base, labelOnlyBase, minor_thresholds, linthresh
  * LogitFormatter: use_overline, one_half, minor, minor_threshold, minor_number
  * StrMethodFormatter: fmt
"""

import math

import matplotlib as mpl
import matplotlib.scale as mscale
import matplotlib.ticker as mticker
import numpy as np
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("scales")

NAN = float("nan")
INF = float("inf")

FUNCTIONS = {
    "square": (lambda x: x ** 2, np.sqrt),
    "cube": (lambda x: x ** 3, np.cbrt),
    "log1p": (np.log1p, np.expm1),
    "affine": (lambda x: 2 * x + 1, lambda y: (y - 1) / 2),
}

SCALES = {
    "LinearScale": ("linear", mscale.LinearScale),
    "LogScale": ("log", mscale.LogScale),
    "SymmetricalLogScale": ("symlog", mscale.SymmetricalLogScale),
    "LogitScale": ("logit", mscale.LogitScale),
    "AsinhScale": ("asinh", mscale.AsinhScale),
    "FuncScale": ("function", mscale.FuncScale),
    "FuncScaleLog": ("functionlog", mscale.FuncScaleLog),
}


def _kwargs(params):
    params = dict(params)
    if "functions" in params:
        params["functions"] = FUNCTIONS[params["functions"]]
    return params


def _scale(cls_name, params):
    return SCALES[cls_name][1](**_kwargs(params))


# --------------------------------------------------------------------------
# Adapters
# --------------------------------------------------------------------------


def _transform_op(cls_name):
    def transform(params, values):
        return _scale(cls_name, params).get_transform().transform(np.asarray(values, dtype=float))

    return transform


def _inverse_op(cls_name):
    def inverse(params, values):
        return _scale(cls_name, params).get_transform().inverted().transform(np.asarray(values, dtype=float))

    return inverse


def _limit_op(cls_name):
    def limit_range_for_scale(params, vmin, vmax, minpos):
        return _scale(cls_name, params).limit_range_for_scale(vmin, vmax, minpos)

    return limit_range_for_scale


def _val_in_range_op(cls_name):
    def val_in_range(params, values):
        return _scale(cls_name, params).val_in_range(np.asarray(values, dtype=float))

    return val_in_range


def _array(value):
    return None if value is None else (value if isinstance(value, str) else np.asarray(value, dtype=float))


def describe(obj):
    """Class name plus the configured parameters of a locator or formatter."""
    out = {"class": type(obj).__name__}
    if isinstance(obj, mticker.MaxNLocator):
        out.update(nbins=obj._nbins, steps=obj._steps, integer=obj._integer, symmetric=obj._symmetric,
                   prune=obj._prune, min_n_ticks=obj._min_n_ticks)
        if isinstance(obj, mticker.LogitLocator):
            out["minor"] = obj.minor
    elif isinstance(obj, mticker.LogLocator):
        out.update(base=obj._base, subs=_array(obj._subs), numticks=obj.numticks)
    elif isinstance(obj, mticker.SymmetricalLogLocator):
        out.update(base=obj._base, linthresh=obj._linthresh, subs=_array(obj._subs), numticks=obj.numticks)
    elif isinstance(obj, mticker.AsinhLocator):
        out.update(linear_width=obj.linear_width, numticks=obj.numticks, symthresh=obj.symthresh, base=obj.base,
                   subs=_array(obj.subs))
    elif isinstance(obj, mticker.AutoMinorLocator):
        out.update(ndivs=obj.ndivs)
    elif isinstance(obj, mticker.ScalarFormatter):
        out.update(useOffset=obj.get_useOffset(), offset=obj.offset, useMathText=obj.get_useMathText(),
                   useLocale=obj.get_useLocale(), usetex=obj.get_usetex(), scientific=obj._scientific,
                   powerlimits=list(obj._powerlimits), offset_threshold=obj._offset_threshold)
    elif isinstance(obj, mticker.LogFormatter):
        out.update(base=obj._base, labelOnlyBase=obj.labelOnlyBase, minor_thresholds=list(obj.minor_thresholds),
                   linthresh=obj._linthresh)
    elif isinstance(obj, mticker.LogitFormatter):
        out.update(use_overline=obj._use_overline, one_half=obj._one_half, minor=obj._minor,
                   minor_threshold=obj._minor_threshold, minor_number=obj._minor_number)
    elif isinstance(obj, mticker.StrMethodFormatter):
        out.update(fmt=obj.fmt)
    return out


def _defaults_op(cls_name):
    def defaults(params, axis, rc=None):
        with mpl.rc_context(rc or {}):
            ax = Figure().add_subplot()
            getattr(ax, f"set_{axis}scale")(SCALES[cls_name][0], **_kwargs(params))
            axis_obj = getattr(ax, f"{axis}axis")
            return {
                "major_locator": describe(axis_obj.get_major_locator()),
                "minor_locator": describe(axis_obj.get_minor_locator()),
                "major_formatter": describe(axis_obj.get_major_formatter()),
                "minor_formatter": describe(axis_obj.get_minor_formatter()),
            }

    return defaults


def scale_factory(scale, params):
    return type(mscale.scale_factory(scale, None, **_kwargs(params))).__name__


def _record_transforms(cls_name, params, forward, inverse):
    probes.probe(f"scale.{cls_name}.transform", _transform_op(cls_name), params=params, values=forward)
    probes.probe(f"scale.{cls_name}.inverted.transform", _inverse_op(cls_name), params=params, values=inverse)


# --------------------------------------------------------------------------
# Probes
# --------------------------------------------------------------------------


def test_scale_registry():
    names = probes.probe("scale.get_scale_names", mscale.get_scale_names)
    assert names == sorted(names)
    for scale, params in [("linear", {}), ("log", {"base": 2}), ("symlog", {"linthresh": 1}), ("logit", {}),
                          ("asinh", {}), ("function", {"functions": "square"}),
                          ("functionlog", {"functions": "affine"}), ("bogus", {}), ("log", {"bogus": 1})]:
        probes.probe("scale.scale_factory", scale_factory, scale=scale, params=params)


GENERIC_VALUES = [-1e300, -100.0, -1.0, -0.5, 0.0, 0.5, 1.0, 100.0, 1e300, NAN, INF, -INF]


def test_linear_scale():
    _record_transforms("LinearScale", {}, GENERIC_VALUES, GENERIC_VALUES)
    probes.probe("scale.LinearScale.limit_range_for_scale", _limit_op("LinearScale"), params={},
                 vmin=-5.0, vmax=5.0, minpos=1.0)
    probes.probe("scale.LinearScale.val_in_range", _val_in_range_op("LinearScale"), params={},
                 values=GENERIC_VALUES)


LOG_FORWARD = [-1.0, 0.0, 1e-300, 1e-5, 0.001, 0.5, 1.0, 2.0, 3.0, 10.0, 100.0, 1024.0, 1e300, NAN, INF, -INF]
LOG_INVERSE = [-1000.0, -300.0, -3.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0, 10.0, 300.0, 400.0, 1100.0, NAN, INF, -INF]


def test_log_scale():
    for params in [{}, {"nonpositive": "mask"}, {"base": 2}, {"base": 2, "nonpositive": "mask"}, {"base": math.e},
                   {"base": 3.0}, {"base": 0.5}, {"base": 100.0, "nonpositive": "mask"}]:
        _record_transforms("LogScale", params, LOG_FORWARD, LOG_INVERSE)
    for params in [{"base": 1.0}, {"base": 0.0}, {"base": -10.0}, {"nonpositive": "drop"}]:
        probes.probe("scale.LogScale.transform", _transform_op("LogScale"), params=params, values=[1.0])
    for vmin, vmax, minpos in [(1.0, 10.0, 1.0), (0.0, 10.0, 0.5), (-5.0, 10.0, 0.5), (-5.0, -1.0, 0.5),
                               (0.0, 10.0, INF), (0.0, 0.0, NAN), (10.0, 0.0, 2.0)]:
        probes.probe("scale.LogScale.limit_range_for_scale", _limit_op("LogScale"), params={}, vmin=vmin,
                     vmax=vmax, minpos=minpos)
    probes.probe("scale.LogScale.val_in_range", _val_in_range_op("LogScale"), params={}, values=LOG_FORWARD)
    forward = _transform_op("LogScale")({}, [0.001, 10.0, 0.0, -1.0])
    assert list(forward) == [-3.0, 1.0, -1000.0, -1000.0]


SYMLOG_FORWARD = [-1e3, -100.0, -10.0, -2.0, -1.0, -0.5, -0.01, 0.0, 0.01, 0.5, 1.0, 2.0, 10.0, 100.0, 1e3, 1e300,
                  NAN, INF, -INF]
SYMLOG_INVERSE = [-10.0, -4.0, -3.0, -2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 10.0, 400.0,
                  NAN, INF, -INF]


def test_symlog_scale():
    for params in [{}, {"linthresh": 1.0}, {"linthresh": 0.01}, {"linthresh": 1.0, "linscale": 0.5},
                   {"linthresh": 1.0, "linscale": 2.0}, {"base": 2, "linthresh": 1.0}, {"base": math.e},
                   {"base": 10, "linthresh": 10.0, "linscale": 0.1}]:
        _record_transforms("SymmetricalLogScale", params, SYMLOG_FORWARD, SYMLOG_INVERSE)
        # All-inside vectors take matplotlib's linear fast path.
        _record_transforms("SymmetricalLogScale", params, [-0.005, 0.0, 0.005], [-0.001, 0.0, 0.001])
    for params in [{"base": 1.0}, {"base": 0.5}, {"linthresh": 0.0}, {"linthresh": -1.0}, {"linscale": 0.0}]:
        probes.probe("scale.SymmetricalLogScale.transform", _transform_op("SymmetricalLogScale"), params=params,
                     values=[1.0])
    probes.probe("scale.SymmetricalLogScale.limit_range_for_scale", _limit_op("SymmetricalLogScale"), params={},
                 vmin=-5.0, vmax=5.0, minpos=1.0)
    probes.probe("scale.SymmetricalLogScale.val_in_range", _val_in_range_op("SymmetricalLogScale"), params={},
                 values=SYMLOG_FORWARD)


LOGIT_FORWARD = [-0.1, 0.0, 1e-300, 1e-10, 0.001, 0.1, 0.25, 0.5, 0.75, 0.9, 0.999, 1 - 1e-10, 1.0, 1.1, NAN, INF,
                 -INF]
LOGIT_INVERSE = [-1000.0, -400.0, -10.0, -3.0, -1.0, 0.0, 1.0, 3.0, 10.0, 400.0, 1000.0, NAN, INF, -INF]


def test_logit_scale():
    for params in [{}, {"nonpositive": "clip"}, {"nonpositive": "mask", "one_half": "1/2"}]:
        _record_transforms("LogitScale", params, LOGIT_FORWARD, LOGIT_INVERSE)
    probes.probe("scale.LogitScale.transform", _transform_op("LogitScale"), params={"nonpositive": "drop"},
                 values=[0.5])
    for vmin, vmax, minpos in [(0.1, 0.9, 1e-3), (0.0, 1.0, 1e-3), (-1.0, 2.0, 0.01), (0.0, 1.0, INF),
                               (0.5, 0.5, 0.1)]:
        probes.probe("scale.LogitScale.limit_range_for_scale", _limit_op("LogitScale"), params={}, vmin=vmin,
                     vmax=vmax, minpos=minpos)
    probes.probe("scale.LogitScale.val_in_range", _val_in_range_op("LogitScale"), params={}, values=LOGIT_FORWARD)


def test_asinh_scale():
    for params in [{}, {"linear_width": 0.1}, {"linear_width": 10.0}, {"linear_width": 1e-5},
                   {"linear_width": 1.0, "base": 2}]:
        _record_transforms("AsinhScale", params, SYMLOG_FORWARD, SYMLOG_INVERSE)
    for params in [{"linear_width": 0.0}, {"linear_width": -1.0}]:
        probes.probe("scale.AsinhScale.transform", _transform_op("AsinhScale"), params=params, values=[1.0])
    probes.probe("scale.AsinhScale.limit_range_for_scale", _limit_op("AsinhScale"), params={}, vmin=-5.0,
                 vmax=5.0, minpos=1.0)
    probes.probe("scale.AsinhScale.val_in_range", _val_in_range_op("AsinhScale"), params={}, values=SYMLOG_FORWARD)


def test_func_scales():
    positive = [0.0, 0.25, 0.5, 1.0, 2.0, 10.0, 1e100, NAN, INF]
    for name in FUNCTIONS:
        _record_transforms("FuncScale", {"functions": name}, positive, positive)
        _record_transforms("FuncScaleLog", {"functions": name}, positive, [-2.0, -1.0, 0.0, 0.5, 1.0, 2.0, NAN])
    _record_transforms("FuncScaleLog", {"functions": "affine", "base": 2}, positive, [-1.0, 0.0, 3.0])
    probes.probe("scale.FuncScale.limit_range_for_scale", _limit_op("FuncScale"), params={"functions": "square"},
                 vmin=-1.0, vmax=4.0, minpos=1.0)
    probes.probe("scale.FuncScaleLog.limit_range_for_scale", _limit_op("FuncScaleLog"),
                 params={"functions": "square"}, vmin=-1.0, vmax=4.0, minpos=0.5)
    probes.probe("scale.FuncScale.val_in_range", _val_in_range_op("FuncScale"), params={"functions": "square"},
                 values=GENERIC_VALUES)
    probes.probe("scale.FuncScaleLog.val_in_range", _val_in_range_op("FuncScaleLog"),
                 params={"functions": "square"}, values=GENERIC_VALUES)


def test_default_locators_and_formatters():
    cases = [
        ("LinearScale", {}),
        ("LogScale", {}),
        ("LogScale", {"base": 2}),
        ("LogScale", {"subs": [2, 5]}),
        ("LogScale", {"base": 2, "subs": [1.5], "nonpositive": "mask"}),
        ("SymmetricalLogScale", {}),
        ("SymmetricalLogScale", {"base": 2, "linthresh": 0.5, "linscale": 2.0, "subs": [2, 3]}),
        ("LogitScale", {}),
        ("LogitScale", {"one_half": "1/2", "use_overline": True}),
        ("AsinhScale", {}),
        ("AsinhScale", {"linear_width": 0.5, "base": 2}),
        ("AsinhScale", {"base": 16}),
        ("AsinhScale", {"base": 7}),
        ("AsinhScale", {"base": 0}),
        ("AsinhScale", {"subs": [3]}),
        ("FuncScale", {"functions": "square"}),
        ("FuncScaleLog", {"functions": "affine"}),
    ]
    for cls_name, params in cases:
        for axis in ["x", "y"]:
            probes.probe(f"scale.{cls_name}.default_locators_and_formatters", _defaults_op(cls_name),
                         params=params, axis=axis)
    for cls_name, params in [("LinearScale", {}), ("FuncScale", {"functions": "affine"})]:
        for axis, rc in [("x", {"xtick.minor.visible": True}), ("y", {"xtick.minor.visible": True}),
                         ("y", {"ytick.minor.visible": True})]:
            probes.probe(f"scale.{cls_name}.default_locators_and_formatters", _defaults_op(cls_name),
                         params=params, axis=axis, rc=rc)
    for cls_name, params in [("LinearScale", {}), ("LogScale", {})]:
        probes.probe(f"scale.{cls_name}.default_locators_and_formatters", _defaults_op(cls_name), params=params,
                     axis="x", rc={"_internal.classic_mode": True})
    defaults = _defaults_op("LogScale")({}, "x")
    assert defaults["major_locator"]["class"] == "LogLocator"
    assert defaults["major_formatter"]["class"] == "LogFormatterSciNotation"
