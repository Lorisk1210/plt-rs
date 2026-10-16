"""Data normalisation (``matplotlib.colors`` norms) and ScalarMappable colouring.

Area ``norms``.

Argument conventions
--------------------
* ``params`` holds the norm's constructor keyword arguments, verbatim (e.g.
  ``{"vmin": 0, "vmax": 1, "clip": false}`` for ``Normalize``,
  ``{"boundaries": [...], "ncolors": 4, "extend": "both"}`` for
  ``BoundaryNorm``). Omitted keys take matplotlib's defaults.
* ``values`` is either a JSON number (the scalar code path; the result is a
  scalar) or a JSON array converted with ``np.asarray(values, dtype=float)``.
  An optional ``mask`` (array of bools, same length) turns it into
  ``np.ma.array(values, mask=mask)``.
* Results: masked entries are recorded as ``null``; NaN/inf are kept.
  ``BoundaryNorm`` yields integers (colour indices, ``-1`` = under,
  ``ncolors`` = over when not clipping).

Ops
---
``colors.<Norm>.__call__``  {params, values, mask?, clip?} -> {result, vmin, vmax}
                            ``clip`` is the call-time override (absent = None).
                            ``vmin``/``vmax`` are the norm's limits *after*
                            the call (autoscaling fills unset limits from the
                            first data seen).
``colors.<Norm>.inverse``   {params, values} -> result (inverse mapping from [0, 1])
``colors.<Norm>.autoscale`` {params, data} -> {vmin, vmax} after ``autoscale(data)``
``colors.<Norm>.autoscale_None`` {params, data} -> {vmin, vmax} after ``autoscale_None(data)``
``cm.ScalarMappable.to_rgba`` {cmap, norm, values, alpha?, bytes?, apply_norm?} -> RGBA rows.
                            ``norm`` is null (default ``Normalize()``) or
                            ``{"class": <Norm>, "params": {...}}``; ``apply_norm``
                            maps to ``to_rgba(norm=...)`` (default true).

``<Norm>`` is one of Normalize, LogNorm, SymLogNorm, PowerNorm, BoundaryNorm,
TwoSlopeNorm, CenteredNorm, AsinhNorm, NoNorm.
"""

import math

import matplotlib.cm as mcm
import matplotlib.colors as mcolors
import numpy as np

from spec_support import ProbeSet

probes = ProbeSet("norms")

NAN = float("nan")
INF = float("inf")

NORMS = {
    "Normalize": mcolors.Normalize,
    "LogNorm": mcolors.LogNorm,
    "SymLogNorm": mcolors.SymLogNorm,
    "PowerNorm": mcolors.PowerNorm,
    "BoundaryNorm": mcolors.BoundaryNorm,
    "TwoSlopeNorm": mcolors.TwoSlopeNorm,
    "CenteredNorm": mcolors.CenteredNorm,
    "AsinhNorm": mcolors.AsinhNorm,
    "NoNorm": mcolors.NoNorm,
}


def _values(values, mask=None):
    if not isinstance(values, list):
        return values
    data = np.asarray(values, dtype=float)
    return data if mask is None else np.ma.array(data, mask=mask)


def _call(cls_name):
    def call(params, values, mask=None, clip=None):
        norm = NORMS[cls_name](**params)
        result = norm(_values(values, mask), clip=clip)
        return {"result": result, "vmin": norm.vmin, "vmax": norm.vmax}

    return call


def _inverse(cls_name):
    def inverse(params, values):
        return NORMS[cls_name](**params).inverse(_values(values))

    return inverse


def _autoscale(cls_name, method):
    def autoscale(params, data):
        norm = NORMS[cls_name](**params)
        getattr(norm, method)(_values(data))
        return {"vmin": norm.vmin, "vmax": norm.vmax}

    return autoscale


def _record(cls_name, params, value_sets, *, clips=(None,), inverse=None):
    """Probe ``__call__`` for every value set/clip and ``inverse`` if given."""
    for values in value_sets:
        for clip in clips:
            kwargs = {} if clip is None else {"clip": clip}
            probes.probe(f"colors.{cls_name}.__call__", _call(cls_name), params=params, values=values, **kwargs)
    if inverse is not None:
        probes.probe(f"colors.{cls_name}.inverse", _inverse(cls_name), params=params, values=inverse)


LINEAR_VALUES = [
    [-1.0, 0.0, 0.25, 0.5, 1.0, 2.0],
    [NAN, INF, -INF],
    [0.1, 0.2, 0.3, 0.7, 0.9],
    [],
]
UNIT_INVERSE = [0.0, 0.25, 0.5, 0.75, 1.0, -0.5, 1.5, NAN]


# --------------------------------------------------------------------------
# Normalize
# --------------------------------------------------------------------------


def test_normalize():
    for params in [
        {"vmin": 0.0, "vmax": 1.0},
        {"vmin": -1.0, "vmax": 1.0},
        {"vmin": -1.0, "vmax": 1.0, "clip": True},
        {"vmin": 0.1, "vmax": 1.0},
        {"vmin": 1e5, "vmax": 1e5 + 1},
        {"vmin": -1e300, "vmax": 1e300},
        {"vmin": 1e-300, "vmax": 2e-300},
        {"vmin": 3.0, "vmax": 7.0},
    ]:
        _record("Normalize", params, LINEAR_VALUES + [[params["vmin"], params["vmax"]]],
                clips=(None, True, False), inverse=UNIT_INVERSE)
    for values in [[0.0, 1.0, 2.0], [5.0], [], [NAN, 1.0, 2.0], [-3.0, 7.0, 2.0, 1.0]]:
        probes.probe("colors.Normalize.__call__", _call("Normalize"), params={}, values=values)
    probes.probe("colors.Normalize.__call__", _call("Normalize"), params={"vmin": 0.0},
                 values=[-1.0, 2.0, 4.0])
    probes.probe("colors.Normalize.__call__", _call("Normalize"), params={"vmax": 0.0},
                 values=[-4.0, -2.0, 1.0])
    # Masked entries stay masked and are ignored by autoscaling.
    probes.probe("colors.Normalize.__call__", _call("Normalize"), params={},
                 values=[0.0, 100.0, 5.0, 10.0], mask=[False, True, False, False])
    probes.probe("colors.Normalize.__call__", _call("Normalize"), params={"vmin": 0.0, "vmax": 10.0, "clip": True},
                 values=[-5.0, 100.0, 5.0, 15.0], mask=[False, True, False, False])
    # Degenerate and invalid limits.
    _record("Normalize", {"vmin": 2.0, "vmax": 2.0}, [[1.0, 2.0, 3.0, NAN]], inverse=UNIT_INVERSE)
    result = probes.probe("colors.Normalize.__call__", _call("Normalize"), params={"vmin": 2.0, "vmax": 1.0},
                          values=[1.5])
    assert isinstance(result, ValueError)
    # Scalars.
    for value in [0.5, 2.0, -1.0, NAN]:
        probes.probe("colors.Normalize.__call__", _call("Normalize"), params={"vmin": 0.0, "vmax": 4.0}, values=value)
        probes.probe("colors.Normalize.inverse", _inverse("Normalize"), params={"vmin": 0.0, "vmax": 4.0}, values=value)
    assert isinstance(probes.probe("colors.Normalize.inverse", _inverse("Normalize"), params={}, values=[0.5]),
                      ValueError)
    for data in [[1.0, 5.0, 3.0], [], [NAN, 1.0], [2.0]]:
        probes.probe("colors.Normalize.autoscale", _autoscale("Normalize", "autoscale"),
                     params={"vmin": -10.0, "vmax": 10.0}, data=data)
        probes.probe("colors.Normalize.autoscale_None", _autoscale("Normalize", "autoscale_None"),
                     params={"vmin": -10.0}, data=data)
    assert _call("Normalize")({"vmin": -1.0, "vmax": 1.0}, [-2.0, 0.0, 2.0])["result"].tolist() == [-0.5, 0.5, 1.5]


# --------------------------------------------------------------------------
# Logarithmic and power-law norms
# --------------------------------------------------------------------------

LOG_VALUES = [
    [0.0, -1.0, 0.5, 1.0, 2.0, 10.0, 50.0, 100.0, 1000.0],
    [NAN, INF, -INF, 1e-300],
    [],
]


def test_log_norm():
    for params in [
        {"vmin": 1.0, "vmax": 100.0},
        {"vmin": 1.0, "vmax": 100.0, "clip": True},
        {"vmin": 0.01, "vmax": 1e6},
        {"vmin": 3.0, "vmax": 7.0},
        {"vmin": 1e-10, "vmax": 1e10},
    ]:
        _record("LogNorm", params, LOG_VALUES, clips=(None, True), inverse=UNIT_INVERSE)
    for values in [[1.0, 10.0, 1000.0], [0.0, -5.0, 10.0, 100.0], [-1.0, 0.0], [NAN, 2.0, 20.0], [5.0]]:
        probes.probe("colors.LogNorm.__call__", _call("LogNorm"), params={}, values=values)
    for params in [{"vmin": 0.0, "vmax": 10.0}, {"vmin": -1.0, "vmax": 10.0}, {"vmin": 10.0, "vmax": 1.0}]:
        probes.probe("colors.LogNorm.__call__", _call("LogNorm"), params=params, values=[1.0, 5.0])
        probes.probe("colors.LogNorm.inverse", _inverse("LogNorm"), params=params, values=[0.5])
    _record("LogNorm", {"vmin": 10.0, "vmax": 10.0}, [[1.0, 10.0, 100.0]])
    for value in [10.0, 0.0, -1.0]:
        probes.probe("colors.LogNorm.__call__", _call("LogNorm"), params={"vmin": 1.0, "vmax": 100.0}, values=value)
    probes.probe("colors.LogNorm.inverse", _inverse("LogNorm"), params={"vmin": 1.0, "vmax": 100.0}, values=0.25)
    probes.probe("colors.LogNorm.autoscale_None", _autoscale("LogNorm", "autoscale_None"),
                 params={}, data=[-1.0, 0.0, 0.5, 20.0, INF])
    probes.probe("colors.LogNorm.autoscale_None", _autoscale("LogNorm", "autoscale_None"),
                 params={}, data=[-1.0, 0.0])
    probes.probe("colors.LogNorm.autoscale", _autoscale("LogNorm", "autoscale"),
                 params={"vmin": 1.0, "vmax": 2.0}, data=[3.0, 30.0])
    assert _call("LogNorm")({"vmin": 1.0, "vmax": 100.0}, [10.0])["result"][0] == 0.5


def test_power_norm():
    for gamma in [0.5, 2.0, 1.0, 3.0, 0.1]:
        for params in [
            {"gamma": gamma, "vmin": 0.0, "vmax": 1.0},
            {"gamma": gamma, "vmin": -2.0, "vmax": 6.0},
            {"gamma": gamma, "vmin": 1.0, "vmax": 4.0, "clip": True},
        ]:
            _record("PowerNorm", params, LINEAR_VALUES + [[-4.0, 1.0, 3.0, 9.0]],
                    clips=(None, True), inverse=UNIT_INVERSE)
    for values in [[1.0, 2.0, 5.0], [NAN, 1.0, 3.0], [], [4.0]]:
        probes.probe("colors.PowerNorm.__call__", _call("PowerNorm"), params={"gamma": 2.0}, values=values)
    _record("PowerNorm", {"gamma": 2.0, "vmin": 1.0, "vmax": 1.0}, [[0.0, 1.0, 2.0]])
    probes.probe("colors.PowerNorm.__call__", _call("PowerNorm"), params={"gamma": 2.0, "vmin": 2.0, "vmax": 1.0},
                 values=[1.5])
    probes.probe("colors.PowerNorm.inverse", _inverse("PowerNorm"), params={"gamma": 2.0}, values=[0.5])
    probes.probe("colors.PowerNorm.__call__", _call("PowerNorm"), params={"gamma": 2.0, "vmin": 0.0, "vmax": 2.0},
                 values=1.0)


SYMLOG_VALUES = [
    [-1000.0, -100.0, -10.0, -1.0, -0.5, 0.0, 0.5, 1.0, 10.0, 100.0, 1000.0],
    [-0.01, 0.01, 2.0, -2.0, NAN, INF, -INF],
    [0.0, 0.5, 0.9],
    [],
]


def test_symlog_norm():
    for params in [
        {"linthresh": 1.0, "vmin": -100.0, "vmax": 100.0},
        {"linthresh": 1.0, "linscale": 0.5, "vmin": -100.0, "vmax": 100.0},
        {"linthresh": 1.0, "linscale": 2.0, "vmin": -100.0, "vmax": 100.0},
        {"linthresh": 0.01, "vmin": -10.0, "vmax": 1000.0},
        {"linthresh": 2.0, "base": 2, "vmin": -64.0, "vmax": 64.0},
        {"linthresh": 1.0, "base": math.e, "vmin": -50.0, "vmax": 50.0},
        {"linthresh": 1.0, "vmin": 0.0, "vmax": 1000.0, "clip": True},
        {"linthresh": 5.0, "vmin": -1.0, "vmax": 1.0},
    ]:
        _record("SymLogNorm", params, SYMLOG_VALUES, clips=(None, True), inverse=UNIT_INVERSE)
    for values in [[-50.0, 0.0, 300.0], [2.0], [NAN, -3.0, 3.0]]:
        probes.probe("colors.SymLogNorm.__call__", _call("SymLogNorm"), params={"linthresh": 1.0}, values=values)
    for params in [{"linthresh": 0.0, "vmin": -1.0, "vmax": 1.0}, {"linthresh": 1.0, "base": 1.0},
                   {"linthresh": 1.0, "linscale": 0.0}, {"linthresh": -1.0}]:
        probes.probe("colors.SymLogNorm.__call__", _call("SymLogNorm"), params=params, values=[0.5])
    probes.probe("colors.SymLogNorm.__call__", _call("SymLogNorm"),
                 params={"linthresh": 1.0, "vmin": -10.0, "vmax": 10.0}, values=-5.0)


def test_asinh_norm():
    for params in [
        {"vmin": -100.0, "vmax": 100.0},
        {"linear_width": 0.1, "vmin": -100.0, "vmax": 100.0},
        {"linear_width": 10.0, "vmin": -5.0, "vmax": 500.0},
        {"linear_width": 1.0, "vmin": 0.0, "vmax": 10.0, "clip": True},
    ]:
        _record("AsinhNorm", params, SYMLOG_VALUES, clips=(None, True), inverse=UNIT_INVERSE)
    probes.probe("colors.AsinhNorm.__call__", _call("AsinhNorm"), params={}, values=[-3.0, 1.0, 30.0])
    probes.probe("colors.AsinhNorm.__call__", _call("AsinhNorm"), params={"linear_width": 0.0}, values=[1.0])
    probes.probe("colors.AsinhNorm.__call__", _call("AsinhNorm"), params={"linear_width": -1.0}, values=[1.0])


# --------------------------------------------------------------------------
# Centred norms
# --------------------------------------------------------------------------


def test_two_slope_norm():
    for params in [
        {"vcenter": 0.0, "vmin": -4000.0, "vmax": 10000.0},
        {"vcenter": 0.0, "vmin": -1.0, "vmax": 1.0},
        {"vcenter": 2.0, "vmin": 0.0, "vmax": 10.0},
        {"vcenter": -5.0, "vmin": -6.0, "vmax": 100.0},
    ]:
        _record("TwoSlopeNorm", params,
                [[-8000.0, -4000.0, -2000.0, 0.0, 2500.0, 5000.0, 7500.0, 10000.0, 20000.0],
                 [-1.0, -0.5, 0.0, 0.5, 1.0, 2.0, NAN, INF, -INF], []],
                inverse=UNIT_INVERSE)
    for params, values in [
        ({"vcenter": 0.0}, [-2.0, 4.0, 1.0]),
        ({"vcenter": 0.0}, [1.0, 4.0]),
        ({"vcenter": 0.0}, [-4.0, -1.0]),
        ({"vcenter": 10.0}, [1.0, 4.0]),
        ({"vcenter": 0.0, "vmin": -2.0}, [1.0, 4.0]),
        ({"vcenter": 0.0, "vmax": 2.0}, [-6.0, 1.0]),
        ({"vcenter": 0.0}, [5.0]),
    ]:
        probes.probe("colors.TwoSlopeNorm.__call__", _call("TwoSlopeNorm"), params=params, values=values)
    for params in [{"vcenter": 1.0, "vmin": 0.0, "vmax": 1.0}, {"vcenter": 0.0, "vmin": 0.0, "vmax": 1.0},
                   {"vcenter": 2.0, "vmin": 3.0, "vmax": 1.0}]:
        assert isinstance(probes.probe("colors.TwoSlopeNorm.__call__", _call("TwoSlopeNorm"), params=params,
                                       values=[0.5]), ValueError)
    probes.probe("colors.TwoSlopeNorm.__call__", _call("TwoSlopeNorm"),
                 params={"vcenter": 0.0, "vmin": -1.0, "vmax": 3.0}, values=1.5)
    probes.probe("colors.TwoSlopeNorm.inverse", _inverse("TwoSlopeNorm"), params={"vcenter": 0.0}, values=[0.5])
    probes.probe("colors.TwoSlopeNorm.autoscale_None", _autoscale("TwoSlopeNorm", "autoscale_None"),
                 params={"vcenter": 0.0}, data=[2.0, 5.0])
    assert _call("TwoSlopeNorm")({"vcenter": 0.0, "vmin": -4000.0, "vmax": 10000.0},
                                 [-2000.0, 5000.0])["result"].tolist() == [0.25, 0.75]


def test_centered_norm():
    for params in [
        {},
        {"vcenter": 0.0, "halfrange": 4.0},
        {"vcenter": 1.0, "halfrange": 2.0},
        {"vcenter": 1.0, "halfrange": -2.0},
        {"vcenter": 0.0, "halfrange": 4.0, "clip": True},
        {"vcenter": -3.0},
    ]:
        _record("CenteredNorm", params,
                [[-2.0, 0.0, 4.0], [-10.0, -4.0, 0.0, 1.0, 4.0, 10.0, NAN], [3.0, 5.0], [-1.0, -1.0]],
                clips=(None, True))
    probes.probe("colors.CenteredNorm.inverse", _inverse("CenteredNorm"), params={"halfrange": 4.0},
                 values=UNIT_INVERSE)
    probes.probe("colors.CenteredNorm.inverse", _inverse("CenteredNorm"), params={}, values=[0.5])
    probes.probe("colors.CenteredNorm.__call__", _call("CenteredNorm"), params={"vcenter": 2.0}, values=[2.0, 2.0])
    probes.probe("colors.CenteredNorm.__call__", _call("CenteredNorm"), params={"halfrange": 1.0}, values=0.5)
    for data in [[-2.0, 6.0], [3.0, 4.0], [1.0]]:
        probes.probe("colors.CenteredNorm.autoscale", _autoscale("CenteredNorm", "autoscale"),
                     params={"vcenter": 1.0, "halfrange": 10.0}, data=data)
        probes.probe("colors.CenteredNorm.autoscale_None", _autoscale("CenteredNorm", "autoscale_None"),
                     params={"vcenter": 1.0}, data=data)
    assert _call("CenteredNorm")({"halfrange": 4.0}, [-2.0, 0.0, 4.0])["result"].tolist() == [0.25, 0.5, 1.0]


# --------------------------------------------------------------------------
# Discrete and pass-through norms
# --------------------------------------------------------------------------

BOUNDARY_VALUES = [
    [-1.0, 0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.99, 5.0, 7.0, 9.999, 10.0, 11.0],
    [NAN, INF, -INF],
    [],
]


def test_boundary_norm():
    boundaries = [0.0, 1.0, 2.0, 5.0, 10.0]
    for extend in ["neither", "min", "max", "both"]:
        for ncolors in [4, 6, 8, 11, 256]:
            params = {"boundaries": boundaries, "ncolors": ncolors, "extend": extend}
            _record("BoundaryNorm", params, BOUNDARY_VALUES)
    for ncolors in [4, 9, 256]:
        _record("BoundaryNorm", {"boundaries": boundaries, "ncolors": ncolors, "clip": True}, BOUNDARY_VALUES,
                clips=(None, False))
        _record("BoundaryNorm", {"boundaries": boundaries, "ncolors": ncolors}, BOUNDARY_VALUES[:1], clips=(True,))
    for params in [
        {"boundaries": [0.0, 1.0], "ncolors": 1},
        {"boundaries": [0.0, 1.0], "ncolors": 5},
        {"boundaries": [0.0, 1.0], "ncolors": 5, "extend": "both"},
        {"boundaries": [-1.0, 0.0, 1.0], "ncolors": 2},
        {"boundaries": [-1.0, 0.0, 1.0], "ncolors": 3},
        {"boundaries": [-1.0, -0.5, 0.0, 0.5, 1.0], "ncolors": 256, "extend": "max"},
        {"boundaries": [1.0, 10.0, 100.0, 1000.0], "ncolors": 7},
    ]:
        _record("BoundaryNorm", params, [[-2.0, -1.0, -0.75, 0.0, 0.25, 0.5, 1.0, 5.0, 50.0, 500.0, 5000.0]])
    probes.probe("colors.BoundaryNorm.__call__", _call("BoundaryNorm"),
                 params={"boundaries": boundaries, "ncolors": 4}, values=[0.5, 3.0, 7.0],
                 mask=[False, True, False])
    for value in [-1.0, 0.5, 3.0, 10.0]:
        probes.probe("colors.BoundaryNorm.__call__", _call("BoundaryNorm"),
                     params={"boundaries": boundaries, "ncolors": 8}, values=value)
    for params in [
        {"boundaries": boundaries, "ncolors": 3},
        {"boundaries": boundaries, "ncolors": 4, "extend": "both"},
        {"boundaries": boundaries, "ncolors": 8, "clip": True, "extend": "min"},
        {"boundaries": [0.0], "ncolors": 4},
        {"boundaries": boundaries, "ncolors": 8, "extend": "sideways"},
    ]:
        probes.probe("colors.BoundaryNorm.__call__", _call("BoundaryNorm"), params=params, values=[0.5])
    probes.probe("colors.BoundaryNorm.inverse", _inverse("BoundaryNorm"),
                 params={"boundaries": boundaries, "ncolors": 4}, values=[0.5])
    assert _call("BoundaryNorm")({"boundaries": boundaries, "ncolors": 4},
                                 [-1.0, 0.5, 7.0, 10.0])["result"].tolist() == [-1, 0, 3, 4]


def test_no_norm():
    for params in [{}, {"vmin": 0.0, "vmax": 1.0, "clip": True}]:
        _record("NoNorm", params, [[0.0, 1.0, 5.0, -3.0, 2.5, NAN]], clips=(None, True), inverse=[0.0, 3.0, -1.0])
    probes.probe("colors.NoNorm.__call__", _call("NoNorm"), params={}, values=3.0)
    probes.probe("colors.NoNorm.inverse", _inverse("NoNorm"), params={}, values=3.0)


# --------------------------------------------------------------------------
# ScalarMappable
# --------------------------------------------------------------------------


def scalar_mappable_to_rgba(cmap, norm, values, alpha=None, bytes=False, apply_norm=True):  # noqa: A002
    norm_obj = None if norm is None else NORMS[norm["class"]](**norm["params"])
    mappable = mcm.ScalarMappable(norm=norm_obj, cmap=cmap)
    return mappable.to_rgba(_values(values), alpha=alpha, bytes=bytes, norm=apply_norm)


def test_scalar_mappable_to_rgba():
    values = [-1.0, 0.0, 0.5, 2.0, 5.0, 9.0, 10.0, 11.0, NAN]
    norms = [
        None,
        {"class": "Normalize", "params": {"vmin": 0.0, "vmax": 10.0}},
        {"class": "Normalize", "params": {"vmin": 0.0, "vmax": 10.0, "clip": True}},
        {"class": "LogNorm", "params": {"vmin": 0.5, "vmax": 10.0}},
        {"class": "SymLogNorm", "params": {"linthresh": 1.0, "vmin": -10.0, "vmax": 10.0}},
        {"class": "PowerNorm", "params": {"gamma": 0.5, "vmin": 0.0, "vmax": 10.0}},
        {"class": "BoundaryNorm", "params": {"boundaries": [0.0, 2.0, 5.0, 10.0], "ncolors": 256}},
        {"class": "BoundaryNorm", "params": {"boundaries": [0.0, 2.0, 5.0, 10.0], "ncolors": 256, "extend": "both"}},
        {"class": "BoundaryNorm", "params": {"boundaries": [0.0, 2.0, 5.0, 10.0], "ncolors": 3}},
        {"class": "TwoSlopeNorm", "params": {"vcenter": 2.0, "vmin": 0.0, "vmax": 10.0}},
        {"class": "CenteredNorm", "params": {"vcenter": 5.0}},
        {"class": "AsinhNorm", "params": {"vmin": -10.0, "vmax": 10.0}},
        {"class": "NoNorm", "params": {}},
    ]
    for cmap in ["viridis", "tab10", "gray_r"]:
        for norm in norms:
            probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap=cmap, norm=norm, values=values)
    for norm in norms[:2]:
        probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap="viridis", norm=norm, values=values,
                     alpha=0.5)
        probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap="viridis", norm=norm, values=values,
                     bytes=True)
        probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap="viridis", norm=norm, values=values,
                     alpha=0.5, bytes=True)
    probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap="viridis", norm=None,
                 values=[0.0, 0.25, 0.5, 1.0, 1.5, -0.5, NAN], apply_norm=False)
    probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap=None, norm=None, values=[0.0, 1.0, 2.0])
    probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap="viridis", norm=None, values=4.0)
