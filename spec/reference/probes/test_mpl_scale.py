"""matplotlib's own tests of axis scales (``tests/test_scale.py``), with the
tests' own inputs: transform round trips, scale keyword checks, log minor
ticks with ``subs``, invalid log limits, scale objects passed to
``set_xscale`` and the tight box of a logit axes.

Area ``mpl_scale``. ``<Scale>`` and ``params`` follow the ``scales`` area
(``test_scales.py``): a scale class name and its constructor keywords.

Ops
---
``scale.<Scale>.transform``  {params, values} -> the forward transform (the
    ``scales`` area's).
``scale.<Scale>.round_trip``  {params, values} -> ``inverted()`` applied to
    the forward transform of ``values``.
``scale.<Scale>.default_locators_and_formatters``  {params, axis} -> the
    tick helpers an axes installs (the ``scales`` area's).
``scale.scale_factory``  {scale, params} -> the class name, or the error
    rejecting a keyword (the ``scales`` area's).
``axes.Axes.set_yscale.ticks``  {scale, params} -> ``{"ylim", "major",
    "minor"}``: the y limits and major and minor tick locations of an empty
    ``Figure().subplots()`` axes after ``set_yscale(scale, **params)`` and a
    draw.
``axes.Axes.set_lim``  {axis, low?, high?} -> ``[before, after]``: the
    limits of ``axis`` of an axes holding ``scatter(range(4), range(4))``
    with that axis on a log scale, before and after ``set_xlim(left=low,
    right=high)`` (``set_ylim(bottom=low, top=high)`` for ``"y"``).
``axes.Axes.set_xscale.object``  {scale} -> ``[xaxis.get_scale(),
    yaxis.get_scale()]`` after both axes get a ``LogScale(axis=None)``
    object (``scale`` names the class).
``axes.Axes.get_tightbbox``  {x, y, xscale, grid} -> the tight box in
    display pixels of a ``Figure().subplots()`` axes after ``plot(x, y)``,
    ``set_xscale(xscale)`` and ``grid(grid)``.

Cases
-----
Named after the upstream tests. ``test_symlog_mask_nan``'s transform
``SymmetricalLogTransform(10, 2, 1)`` is ``SymmetricalLogScale(base=10,
linthresh=2, linscale=1)``'s; ``TestAsinhScale.test_transforms``' inverse of
the inverse is the forward transform.

Already covered
---------------
By the ``scales`` area: ``test_logscale_invert_transform`` (base-2 inverse
transforms), ``TestAsinhScale.test_init``, ``test_fmtloc`` and
``test_bad_scale``, ``test_val_in_range``, ``test_val_in_range_base_fallback``
and ``test_val_in_range_array``. By the ``mpl_small`` figure group:
``test_log_scales``, the image of ``test_logit_scales``,
``test_logscale_mask``, ``test_logscale_nonpos_values`` and
``test_function_scale``. ``test_log_scatter`` only saves a scatter plot to
PDF, EPS and SVG, which the vector cases do.

Left out
--------
``test_optional_axis_signature`` and ``test_custom_scale_with_axis``
(Python constructor signatures and their deprecation),
``test_custom_scale_without_axis`` (registering a Python scale class; plt-rs
takes custom scale objects through ``Axes::set_scale`` and
``ScaleRegistry::register``), ``test_logscale_transform_repr`` (Python
``repr``), ``test_scale_deepcopy``
(Python copies), the masked-array half of ``test_symlog_mask_nan`` (numpy
masked arrays) and the warnings ``test_invalid_log_lims`` expects (plt-rs
has no warnings).
"""

import numpy as np
from matplotlib.figure import Figure
import matplotlib.scale as mscale

from probes.test_scales import _defaults_op, _scale, _transform_op, scale_factory
from spec_support import ProbeSet

probes = ProbeSet("mpl_scale")


def _round_trip_op(cls_name):
    def round_trip(params, values):
        transform = _scale(cls_name, params).get_transform()
        x = np.asarray(values, dtype=float)
        return transform.inverted().transform_non_affine(transform.transform_non_affine(x))

    return round_trip


def test_symlog_mask_nan():
    params = {"base": 10, "linthresh": 2, "linscale": 1}
    x = np.arange(-1.5, 5, 0.5)
    for values in (x.copy(), np.where(np.arange(len(x)) == 4, np.nan, x)):
        r = probes.probe("scale.SymmetricalLogScale.round_trip",
                         _round_trip_op("SymmetricalLogScale"), params=params,
                         values=values.tolist())
        np.testing.assert_allclose(r, values)


def _ticks(scale, params):
    fig = Figure()
    ax = fig.subplots()
    ax.set_yscale(scale, **params)
    fig.draw_without_rendering()
    return {"ylim": ax.get_ylim(), "major": ax.yaxis.get_majorticklocs(),
            "minor": ax.yaxis.get_minorticklocs()}


def test_logscale_subs():
    probes.probe("axes.Axes.set_yscale.ticks", _ticks, scale="log", params={"subs": [2, 3, 4]})


def test_extra_kwargs_raise():
    for scale in ["linear", "log", "symlog"]:
        r = probes.probe("scale.scale_factory", scale_factory, scale=scale,
                         params={"foo": "mask"})
        assert isinstance(r, TypeError)


def _set_lim(axis, low=None, high=None):
    ax = Figure().subplots()
    ax.scatter(range(0, 4), range(0, 4))
    getattr(ax, f"set_{axis}scale")("log")
    get_lim = getattr(ax, f"get_{axis}lim")
    before = get_lim()
    if axis == "x":
        ax.set_xlim(left=low, right=high)
    else:
        ax.set_ylim(bottom=low, top=high)
    return [before, get_lim()]


def test_invalid_log_lims():
    for axis, limit in [("x", {"low": 0}), ("x", {"high": -1}), ("y", {"low": 0}),
                        ("y", {"high": -1})]:
        before, after = probes.probe("axes.Axes.set_lim", _set_lim, axis=axis, **limit)
        assert before == after


def _pass_scale(scale):
    ax = Figure().subplots()
    ax.set_xscale(getattr(mscale, scale)(axis=None))
    ax.set_yscale(getattr(mscale, scale)(axis=None))
    return [ax.xaxis.get_scale(), ax.yaxis.get_scale()]


def test_pass_scale():
    r = probes.probe("axes.Axes.set_xscale.object", _pass_scale, scale="LogScale")
    assert r == ["log", "log"]


def _tightbbox(x, y, xscale, grid):
    fig = Figure()
    ax = fig.subplots()
    ax.plot(x, y)
    ax.set_xscale(xscale)
    ax.grid(grid)
    return ax.get_tightbbox().get_points()


def test_logit_scales():
    x = np.array([0.001, 0.003, 0.01, 0.03, 0.1, 0.2, 0.3, 0.4, 0.5,
                  0.6, 0.7, 0.8, 0.9, 0.97, 0.99, 0.997, 0.999])
    r = probes.probe("axes.Axes.get_tightbbox", _tightbbox, x=x.tolist(), y=(1.0 / x).tolist(),
                     xscale="logit", grid=True)
    assert np.isfinite(r).all()


class TestAsinhScale:
    def test_transforms(self):
        params = {"linear_width": 17.0}
        a = np.linspace(-50, 50, 100).tolist()
        probes.probe("scale.AsinhScale.round_trip", _round_trip_op("AsinhScale"), params=params,
                     values=a)
        probes.probe("scale.AsinhScale.transform", _transform_op("AsinhScale"), params=params,
                     values=a)

    def test_base_init(self):
        for params in [{"base": 3}, {"base": 7, "subs": [2, 4]}]:
            for axis in ["x", "y"]:
                probes.probe("scale.AsinhScale.default_locators_and_formatters",
                             _defaults_op("AsinhScale"), params=params, axis=axis)
