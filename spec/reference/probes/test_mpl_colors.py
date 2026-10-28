"""Colour conversion, colormaps, norms and shading, ported from matplotlib's
own ``tests/test_colors.py`` with the tests' own inputs.

Area ``mpl_colors``.

Most ops are those of the ``colors``, ``colormaps``, ``norms`` and
``lightsource`` areas, with their argument conventions (colour specs with
JSON arrays as tuples, norms from ``params``, ``mask`` for masked values,
``relief`` keywords for shading): ``colors.to_rgba``, ``colors.to_rgb``,
``colors.to_hex``, ``colors.to_rgba_array``, ``colors.is_color_like``,
``colors.same_color``, ``colors.rgb_to_hsv``, ``colors.hsv_to_rgb``,
``colors.color_sequences``, ``colors.color_sequence``, ``colormaps.info``,
``colormaps.sample``, ``colormaps.sample_alpha``, ``colormaps.get_cmap``,
``colors.ListedColormap``, ``colors.LinearSegmentedColormap.from_list``,
``colors.<Norm>.__call__`` / ``.inverse`` / ``.autoscale`` /
``.autoscale_None``, ``cm.ScalarMappable.to_rgba``, ``colors.MultiNorm``
and its ``__call__``, ``inverse``, ``autoscale`` and ``autoscale_None``,
``colorizer.Colorizer.to_rgba``, ``lightsource.hillshade`` and
``lightsource.shade``. ``colors.to_hex`` and ``colors.to_rgb`` may carry an
``rc`` (here a property cycle) applied around the call;
``cm.ScalarMappable.to_rgba`` also takes an RGB or RGBA image (rows of
pixels of three or four floats), which it passes through as matplotlib
does.

And these:

``colors.LinearSegmentedColormap`` {segmentdata, N?, bad?, under?, over?}
    -> {N, lut, under, over, bad}.
``colors.Colormap.resampled`` {kind, colors, under, over, bad, lutsize, x}
    -> {N, sample, under, over, bad} of the ``"listed"`` or ``"from_list"``
    colormap of ``colors`` resampled to ``lutsize`` entries.
``colors.ListedColormap.with_alpha`` {colors, alpha, x} -> {sample, under,
    over, bad}.
``colors.Colormap.__eq__`` {name, change} -> whether ``colormaps[name]``
    equals its copy after ``change``: null (none), ``"bad"``
    (``with_extremes(bad='y')``), ``"name"`` (renamed) or
    ``"colorbar_extend"`` (flipped).
``colormaps.register`` {name} -> registering ``colormaps[name]`` under its
    own name with ``force=True``: null, or an error for a built-in name.
``colormaps.register.get`` {name, over, as_name} -> {name, N, over} of the
    colormap read back after registering ``colormaps[name].with_extremes(
    over=over)`` under ``as_name`` (unregistered afterwards).
``colors.from_levels_and_colors.with_extremes`` {levels, colors, extend,
    bad, values, masked} -> ``cmap(norm(values))`` rows after
    ``cmap.with_extremes(bad=bad)``; with ``masked`` the values are masked.
``colors.TwoSlopeNorm.scaled`` {params, values} -> ``[before, after]``:
    ``norm.scaled()`` before and after ``norm(values)``.
``colors.CenteredNorm`` {params, steps} -> ``{vmin, vmax, halfrange,
    vcenter}`` after each step: ``["call", values]`` (``norm(values)``) or
    ``[attribute, value]`` assigning ``vcenter``, ``halfrange``, ``vmin`` or
    ``vmax``.
``colors.FuncNorm.__call__`` / ``.inverse`` {functions, vmin, vmax, values}
    -> the result; ``functions`` names the pair ``"square"`` (``x**2``,
    ``sqrt``) or ``"log10"`` (``log10``, ``10**x``).
``colorbar.Colorbar.get_ticks`` {norm} -> the ticks of
    ``ColorbarBase(Figure().add_subplot(), norm=norm)``.
``colorbar.Colorbar.ax.get_ylim`` {data, norm} -> the colorbar's y limits
    for ``imshow(data, norm=norm)``.
``colorizer.Colorizer`` {vmin?, vmax?} -> {vmin, vmax, norm_vmin, norm_vmax}
    of a ``Colorizer()`` after setting the limits given.
``color_sequences.register`` {name, colors} -> null or an error.
``color_sequences.__getitem__`` {name, register?} -> the colours (with
    ``register`` the sequence is registered under ``name`` first); an error
    for an unknown name.
``color_sequences.unregister`` {name, times} -> null after unregistering
    ``times`` times, or an error.

Registrations are undone before each op returns. Norms are given as
``{"class", "params"}``.

Cases
-----
Named after the upstream tests. Random data come from ``spec_support.rng``
(``test_BoundaryNorm`` and ``test_CenteredNorm`` draw normal samples,
``test_rgb_hsv_round_trip`` uniform ones). ``test_light_source_planar_hillshading``
records a cross-section of its azimuths and elevations. A test asserting
that two results agree records both.

Left out
--------
``test_index_dtype``, ``test_colormap_endian``, ``test_colormap_return_types``,
``test_2d_to_rgba``, ``test_set_dict_to_rgba``, ``test_ndarray_subclass_norm``,
``test_ensure_multivariate_data``, ``test_mult_norm_call_types``'
structured and 3-D arrays, the 2-D halves of
``test_colormap_bad_data_with_alpha`` and of
``test_colorizer_multinorm_implicit``, and
``test_to_rgba_array_accepts_color_alpha_tuple_with_multiple_colors`` (an
array or a list inside a tuple) (numpy dtypes, byte orders, shapes and
Python iterable types), ``test_colormap_copy``, ``test_norm_deepcopy`` and
``test_non_mutable_get_values`` (Python's copy protocol and deprecated
in-place setters), ``test_norm_callback``,
``test_set_clim_emits_single_callback`` and
``test_scalarmappable_norm_update`` (callbacks and staleness),
``test_conversions_masked``, ``test_autoscale_masked``,
``test_light_source_shading_empty_mask``,
``test_light_source_masked_shading`` and the masked halves of
``test_colormap_invalid`` and ``test_scalarmappable_to_rgba`` (numpy masked
arrays the plt-rs colour APIs do not take), the integer half of
``test_scalarmappable_to_rgba`` (uint8 images), the list cases of
``test_is_color_like`` (Python lists as opposed to tuples),
``test_has_alpha_channel`` (a private helper on Python specs),
``test_make_norm_from_scale_name`` (Python class names), the ``object()``
norm of ``test_multi_norm_creation`` (any Python object as a norm),
``test_repr_png`` and ``test_repr_html`` (notebook representations),
``test_pandas_iterable``
(pandas), ``test_invalid_cmap_n_components_zero``, ``test_colorizer_bivar_cmap``
and ``test_colorizer_multivar_cmap`` (custom Colormap subclasses and array
validation of ColorizingArtist), ``test_cm_set_cmap_error`` (the lookup
``test_colormaps_get_cmap`` probes); ``test_grey_gray`` and
``test_tableau_order`` (the ``colors`` area records the named tables),
``test_colormap_reversing`` and ``test_get_under_over_bad`` (the
``colormaps`` area reverses and samples every colormap at its extremes),
the image tests (``mpl_misc`` figure cases).
"""

import math

import matplotlib as mpl
import matplotlib.colorbar as mcolorbar
import matplotlib.colorizer as mcolorizer
import matplotlib.colors as mcolors
import numpy as np
import pytest
from matplotlib.figure import Figure

from probes.test_colormaps import _spec, from_list, info, listed, sample, sample_alpha, get_cmap
from probes.test_colors import (
    _hsv_to_rgb, _rgb_to_hsv, is_color_like, same_color, to_hex, to_rgb,
    to_rgba, to_rgba_array, _color_sequence)
from probes.test_lightsource import _hillshade, _shade
from probes.test_norms import (
    NORMS, _autoscale, _call, _colorizer_to_rgba, _inverse, _multi_attrs, _multi_autoscale,
    _multi_call, _multi_inverse, scalar_mappable_to_rgba)
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_colors")

pytestmark = pytest.mark.filterwarnings("ignore::RuntimeWarning", "ignore::UserWarning",
                                        "ignore::PendingDeprecationWarning")

NAN = math.nan
INF = math.inf
E = math.e
DEFAULT_RELIEF = {"vert_exag": 1, "dx": 1, "dy": 1, "fraction": 1}


def norm_call(cls, params, values, **kw):
    return probes.probe(f"colors.{cls}.__call__", _call(cls), params=params, values=values, **kw)


def norm_inverse(cls, params, values):
    return probes.probe(f"colors.{cls}.inverse", _inverse(cls), params=params, values=values)


def _in_rc(fn):
    def evaluate(rc=None, **kw):
        with mpl.rc_context(rc or {}):
            return fn(**kw)

    return evaluate


# ---------------------------------------------------------------------------
# Colormaps
# ---------------------------------------------------------------------------


def _segmented(segmentdata, N=256, bad=None, under=None, over=None):
    cmap = mcolors.LinearSegmentedColormap("probe", segmentdata, N, bad=_spec(bad), under=_spec(under),
                                           over=_spec(over))
    return {"N": cmap.N, "lut": cmap(np.arange(cmap.N)), "under": cmap.get_under(),
            "over": cmap.get_over(), "bad": cmap.get_bad()}


def test_create_lookup_table():
    data = [[0.0, 1.0, 1.0], [0.5, 0.2, 0.2], [1.0, 0.0, 0.0]]
    for n in (5, 2, 1):
        probes.probe("colors.LinearSegmentedColormap", _segmented,
                     segmentdata={"red": data, "green": data, "blue": data}, N=n)


def _resampled(kind, colors, under, over, bad, lutsize, x):
    kw = {"under": under, "over": over, "bad": bad}
    cmap = (mcolors.LinearSegmentedColormap.from_list("lsc", colors, **kw) if kind == "from_list"
            else mcolors.ListedColormap(colors, **kw)).resampled(lutsize)
    return {"N": cmap.N, "sample": cmap(np.asarray(x, float)), "under": cmap.get_under(),
            "over": cmap.get_over(), "bad": cmap.get_bad()}


def test_resampled():
    n = 101
    colors = np.empty((n, 4), float)
    colors[:, 0] = np.linspace(0, 1, n)
    colors[:, 1] = 0.2
    colors[:, 2] = np.linspace(1, 0, n)
    colors[:, 3] = 0.7
    for kind in ("from_list", "listed"):
        probes.probe("colors.Colormap.resampled", _resampled, kind=kind, colors=colors.tolist(),
                     under="red", over="green", bad="blue", lutsize=3, x=[0, 0.5, 1, INF, -INF, NAN])


def test_monochrome():
    for colors in (["red"], ["red"] * 5, ["red", "green"]):
        probes.probe("colors.ListedColormap", listed, colors=colors)


def test_colormaps_get_cmap():
    for cmap in ("plasma", None, "AardvarksAreAwkward"):
        probes.probe("colormaps.get_cmap", get_cmap, cmap=cmap)


def test_double_register_builtin_cmap():
    def register(name):
        mpl.colormaps.register(mpl.colormaps[name], name=name, force=True)

    assert isinstance(probes.probe("colormaps.register", register, name="viridis"), ValueError)


def _equal_after(name, change=None):
    cmap = mpl.colormaps[name]
    other = cmap.copy()
    if change == "bad":
        other = other.with_extremes(bad="y")
    elif change == "name":
        other.name = "Test"
    elif change == "colorbar_extend":
        other.colorbar_extend = not cmap.colorbar_extend
    return bool(other == cmap)


def test_colormap_equals():
    for change in (None, "bad", "name", "colorbar_extend"):
        probes.probe("colors.Colormap.__eq__", _equal_after, name="plasma", change=change)


def test_colormap_invalid():
    probes.probe("colormaps.sample", sample, name="plasma", x=[-INF, -1, 0, NAN, .7, 2, INF])


EXTREMES = {"bad": "c", "under": "m", "over": "y"}


def test_ListedColormap_bad_under_over():
    probes.probe("colors.ListedColormap", listed, colors=["r", "g", "b"], **EXTREMES)


def test_LinearSegmentedColormap_bad_under_over():
    cdict = {
        "red": [(0., 0., 0.), (0.5, 1., 1.), (1., 1., 1.)],
        "green": [(0., 0., 0.), (0.25, 0., 0.), (0.75, 1., 1.), (1., 1., 1.)],
        "blue": [(0., 0., 0.), (0.5, 0., 0.), (1., 1., 1.)],
    }
    probes.probe("colors.LinearSegmentedColormap", _segmented,
                 segmentdata={k: [list(row) for row in v] for k, v in cdict.items()}, **EXTREMES)


def test_LinearSegmentedColormap_from_list_bad_under_over():
    probes.probe("colors.LinearSegmentedColormap.from_list", from_list, colors=["r", "g", "b"], **EXTREMES)


def _with_alpha(colors, alpha, x):
    cmap = mcolors.ListedColormap([_spec(c) for c in colors]).with_alpha(alpha)
    return {"sample": cmap(np.asarray(x, float)), "under": cmap.get_under(), "over": cmap.get_over(),
            "bad": cmap.get_bad()}


def test_colormap_with_alpha():
    colors = ["red", "green", ["blue", 0.8]]
    probes.probe("colors.ListedColormap", listed, colors=colors)
    probes.probe("colors.ListedColormap.with_alpha", _with_alpha, colors=colors, alpha=0.5, x=[0, 0.5, 1])


def test_colormap_alpha_array():
    vals = [-1, 0.5, 2]
    assert isinstance(probes.probe("colormaps.sample_alpha", sample_alpha, name="viridis", x=vals,
                                   alpha=[1, 1, 1, 1]), ValueError)
    for as_bytes in (False, True):
        probes.probe("colormaps.sample_alpha", sample_alpha, name="viridis", x=vals,
                     alpha=[0.1, 0.2, 0.3], bytes=as_bytes)


def test_colormap_bad_data_with_alpha():
    for x, alpha in [([NAN], 0.5), ([0.5, NAN], 0.5), ([0.5, NAN], [0.1, 0.2])]:
        probes.probe("colormaps.sample_alpha", sample_alpha, name="viridis", x=x, alpha=alpha)


def test_cmap_alias_names():
    for name in ("gray", "grey"):
        probes.probe("colormaps.info", info, name=name)


def test_close_error_name():
    assert isinstance(probes.probe("colormaps.info", info, name="grays"), KeyError)
    assert isinstance(probes.probe("colors.color_sequence", _color_sequence, name="set"), KeyError)


def _register_get(name, over, as_name):
    cmap = mpl.colormaps[name].with_extremes(over=over)
    cmap.name = "test-cmap"
    mpl.colormaps.register(name=as_name, cmap=cmap)
    try:
        returned = mpl.colormaps[as_name]
        return {"name": returned.name, "N": returned.N, "over": returned.get_over()}
    finally:
        mpl.colormaps.unregister(as_name)


def test_set_cmap_mismatched_name():
    probes.probe("colormaps.register.get", _register_get, name="viridis", over="r", as_name="wrong-cmap")


def test_LinearSegmentedColormap_from_list_color_alpha_tuple():
    probes.probe("colors.LinearSegmentedColormap.from_list", from_list,
                 colors=[["red", 0.3], ["0.42", 0.1], "green"], N=3)
    probes.probe("colors.to_rgba_array", to_rgba_array, c=[["red", 0.3], ["0.42", 0.1], "green"])


def test_LinearSegmentedColormap_from_list_invalid_inputs():
    for colors in ([[0.42, "blue"], [.1, .1, .1, .1]], ["blue", [0.42, "red"]],
                   ["blue", [.1, .1, .1, .1], ["red", 2]], [[0, "red"], [1.1, "blue"]],
                   [[0.52, "red"], [0.42, "blue"]]):
        error = probes.probe("colors.LinearSegmentedColormap.from_list", from_list, colors=colors)
        assert isinstance(error, ValueError)


def test_LinearSegmentedColormap_from_list_value_color_tuple():
    probes.probe("colors.LinearSegmentedColormap.from_list", from_list,
                 colors=[[0, "red"], [0.6, "blue"], [1, "green"]], N=11)


def test_LinearSegmentedColormap_from_list_repeat_values():
    probes.probe("colors.LinearSegmentedColormap.from_list", from_list,
                 colors=[[0, "red"], [0.6, "red"], [0.6, "blue"], [1, "green"]], N=11)


# ---------------------------------------------------------------------------
# Norms
# ---------------------------------------------------------------------------


def test_boundary_norm():
    vals = [-1, 0, 1, 2, 2.2, 4]
    norm_call("BoundaryNorm", {"boundaries": [0, 1.1, 2.2], "ncolors": 2}, vals)
    norm_call("BoundaryNorm", {"boundaries": [0, 1.1, 2.2], "ncolors": 3}, vals)
    norm_call("BoundaryNorm", {"boundaries": [0, 2.2], "ncolors": 3}, vals)
    norm_call("BoundaryNorm", {"boundaries": [0, 1, 2, 3], "ncolors": 5}, [-1, 0.1, 1.1, 2.2, 4])
    for params in ({"ncolors": 2}, {"ncolors": 3}, {"ncolors": 3, "clip": True}):
        params = {"boundaries": [0, 1, 2], **params}
        for v in (-1, 0.1, 1.1, 2.2):
            norm_call("BoundaryNorm", params, v)
            norm_call("BoundaryNorm", params, [v])
    masked = {"values": [-1.0, NAN, 0, 1.4, 9], "mask": [False, True, False, False, False]}
    for ncolors in (2, 3):
        norm_call("BoundaryNorm", {"boundaries": [0, 1.1, 2.2], "ncolors": ncolors}, **masked)
    for values in ([INF, NAN], [INF]):
        norm_call("BoundaryNorm", {"boundaries": [0, 1.1, 2.2], "ncolors": 3}, values,
                  mask=[True] * len(values))
    for params in ({"ncolors": 5, "extend": "both", "clip": True}, {"ncolors": 2},
                   {"ncolors": 3, "extend": "min"}, {"ncolors": 4, "extend": "both"}):
        error = norm_call("BoundaryNorm", {"boundaries": [0, 1, 2, 3], **params}, [0.5])
        assert isinstance(error, ValueError)


def test_boundary_norm_extend():
    bounds = [1, 2, 3]
    x = (rng(19680801).normal(100) * 10 + 2).tolist()
    norm_call("BoundaryNorm", {"boundaries": bounds, "ncolors": 256, "extend": "both"}, x)
    norm_call("BoundaryNorm", {"boundaries": [0, *bounds, 4], "ncolors": 256}, x)
    for extend, ncolors, ref_ncolors, xs in [("both", 4, 2, [-1, 1.2, 2.3, 9.6]),
                                            ("min", 3, 2, [-1, 1.2, 2.3]),
                                            ("max", 3, 2, [1.2, 2.3, 4])]:
        params = {"boundaries": bounds, "ncolors": ncolors, "extend": extend}
        norm_call("BoundaryNorm", params, xs)
        norm_call("BoundaryNorm", params, x)
        norm_call("BoundaryNorm", {"boundaries": bounds, "ncolors": ref_ncolors}, x)
    for v in (0.9, 1.1, 2.9, 3.1):
        norm_call("BoundaryNorm", {"boundaries": bounds, "ncolors": 4, "extend": "both"}, v)
    for colors, extremes in [(["white", "blue", "red", "black"], {}),
                             (["blue", "red"], {"under": "white", "over": "black"}),
                             (["white", "blue", "red"], {}), (["blue", "red"], {"under": "white"}),
                             (["blue", "red", "black"], {}), (["blue", "red"], {"over": "black"})]:
        probes.probe("colors.ListedColormap", listed, colors=colors, **extremes)


def _centered(params, steps):
    norm = mcolors.CenteredNorm(**params)
    states = []
    for name, value in steps:
        if name == "call":
            norm(np.asarray(value, float))
        else:
            setattr(norm, name, value)
        states.append({"vmin": norm.vmin, "vmax": norm.vmax, "halfrange": norm.halfrange,
                       "vcenter": norm.vcenter})
    return states


def test_centered_norm():
    stream = rng(0)
    x = stream.normal(100).tolist()
    x_maxabs = max(abs(v) for v in x)
    norm_call("Normalize", {"vmin": -x_maxabs, "vmax": x_maxabs}, x)
    norm_call("CenteredNorm", {}, x)
    vcenter = int(stream.normal(1, scale=50)[0])
    probes.probe("colors.CenteredNorm.autoscale_None", _autoscale("CenteredNorm", "autoscale_None"),
                 params={"vcenter": vcenter}, data=[1, 2])
    probes.probe("colors.CenteredNorm.autoscale_None", _autoscale("CenteredNorm", "autoscale_None"),
                 params={"halfrange": 1.0}, data=[1, 3000])
    x = stream.normal(10).tolist()
    for params in ({"vcenter": 0.5, "halfrange": 0.5}, {"vcenter": 1, "halfrange": 1},
                   {"vcenter": 2, "halfrange": 2}):
        norm_call("CenteredNorm", params, x)
    for steps in ([["vcenter", 2], ["halfrange", 2], ["call", x]],
                  [["halfrange", 2], ["vcenter", 2], ["call", x]],
                  [["call", np.linspace(-1.0, 0.0, 10).tolist()], ["vcenter", 1], ["vmin", -1],
                   ["vmax", 2]]):
        probes.probe("colors.CenteredNorm", _centered, params={}, steps=steps)


def test_lognorm_invalid():
    for vmin, vmax in ((-1, 2), (3, 1)):
        params = {"vmin": vmin, "vmax": vmax}
        assert isinstance(norm_call("LogNorm", params, 1), ValueError)
        assert isinstance(norm_inverse("LogNorm", params, 1), ValueError)


def test_LogNorm():
    norm_call("LogNorm", {"clip": True, "vmax": 5}, [1, 6])


def test_LogNorm_inverse():
    params = {"vmin": 0.1, "vmax": 10}
    norm_call("LogNorm", params, [0.5, 0.4])
    norm_call("LogNorm", params, 0.4)
    norm_inverse("LogNorm", params, [0.349485, 0.30103])
    norm_inverse("LogNorm", params, [0.30103])


def test_PowerNorm():
    a = [0, 0.5, 1, 1.5]
    norm_call("PowerNorm", {"gamma": 1}, a)
    norm_call("Normalize", {}, a)
    params = {"gamma": 2, "vmin": 0, "vmax": 8}
    a = [-0.5, 0, 2, 4, 8]
    forward = norm_call("PowerNorm", params, a)["result"]
    for v in (a[0], a[2]):
        norm_call("PowerNorm", params, v)
    norm_inverse("PowerNorm", params, forward.tolist())
    a = [-0.5, 0, 1, 8, 16]
    clipped = {"gamma": 2, "vmin": 2, "vmax": 8}
    norm_call("PowerNorm", {**clipped, "clip": True}, a)
    for v in (a[0], a[-1]):
        norm_call("PowerNorm", {**clipped, "clip": True}, v)
        norm_call("PowerNorm", {**clipped, "clip": False}, v, clip=True)
    norm_call("PowerNorm", {**clipped, "clip": False}, a, clip=True)
    norm_call("PowerNorm", {**clipped, "clip": False}, [5, 2], mask=[True, False], clip=True)


def test_PowerNorm_translation_invariance():
    norm_call("PowerNorm", {"vmin": 0, "vmax": 1, "gamma": 3}, [0, 0.5, 1])
    norm_call("PowerNorm", {"vmin": -2, "vmax": -1, "gamma": 3}, [-2, -1.5, -1])


def _colorbar_ylim(data, norm):
    fig = Figure()
    ax = fig.add_subplot()
    im = ax.imshow(np.asarray(data), norm=NORMS[norm["class"]](**norm["params"]))
    return list(fig.colorbar(im).ax.get_ylim())


def test_powernorm_cbar_limits():
    data = (np.arange(100).reshape(10, 10) + 300).tolist()
    probes.probe("colorbar.Colorbar.ax.get_ylim", _colorbar_ylim, data=data,
                 norm={"class": "PowerNorm", "params": {"gamma": 0.2, "vmin": 300, "vmax": 1000}})


def test_normalize():
    vals = [float(v) for v in range(-10, 10)]
    result = norm_call("Normalize", {}, vals)
    norm_inverse("Normalize", {"vmin": -10.0, "vmax": 9.0}, result["result"].tolist())
    for v in vals:
        norm_call("Normalize", {}, v)
    norm_call("Normalize", {}, vals, mask=[True] + [False] * (len(vals) - 1))
    norm_call("Normalize", {"vmin": -128, "vmax": 127}, [-128, 127])


FUNCTIONS = {
    "square": (lambda x: x ** 2, np.sqrt),
    "log10": (np.log10, lambda x: 10 ** x),
}


def _func_norm(method):
    def evaluate(functions, vmin, vmax, values):
        norm = mcolors.FuncNorm(FUNCTIONS[functions], vmin=vmin, vmax=vmax)
        return getattr(norm, method)(np.asarray(values, float))

    return evaluate


def test_func_norm():
    probes.probe("colors.FuncNorm.__call__", _func_norm("__call__"), functions="square", vmin=0,
                 vmax=10, values=[0, 5, 10])
    probes.probe("colors.FuncNorm.inverse", _func_norm("inverse"), functions="square", vmin=0,
                 vmax=10, values=[0, 0.25, 1])
    for method in ("__call__", "inverse"):
        probes.probe(f"colors.FuncNorm.{method}", _func_norm(method), functions="log10", vmin=0.1,
                     vmax=10, values=[0.2, 5, 10])
    norm_call("LogNorm", {"vmin": 0.1, "vmax": 10}, [0.2, 5, 10])
    norm_inverse("LogNorm", {"vmin": 0.1, "vmax": 10}, [0.2, 5, 10])


def _two_slope_error(vmin, vcenter, vmax):
    error = norm_call("TwoSlopeNorm", {"vmin": vmin, "vcenter": vcenter, "vmax": vmax}, [0.0])
    assert isinstance(error, ValueError)


def test_TwoSlopeNorm_autoscale():
    probes.probe("colors.TwoSlopeNorm.autoscale", _autoscale("TwoSlopeNorm", "autoscale"),
                 params={"vcenter": 20}, data=[10, 20, 30, 40])


def test_TwoSlopeNorm_autoscale_None_vmin():
    params = {"vcenter": 2, "vmin": 0, "vmax": None}
    probes.probe("colors.TwoSlopeNorm.autoscale_None", _autoscale("TwoSlopeNorm", "autoscale_None"),
                 params=params, data=[1, 2, 3, 4, 5])
    norm_call("TwoSlopeNorm", params, [1, 2, 3, 4, 5])


def test_TwoSlopeNorm_autoscale_None_vmax():
    params = {"vcenter": 2, "vmin": None, "vmax": 10}
    probes.probe("colors.TwoSlopeNorm.autoscale_None", _autoscale("TwoSlopeNorm", "autoscale_None"),
                 params=params, data=[1, 2, 3, 4, 5])
    norm_call("TwoSlopeNorm", params, [1, 2, 3, 4, 5])


def _scaled(params, values):
    norm = mcolors.TwoSlopeNorm(**params)
    before = norm.scaled()
    norm(np.asarray(values, float))
    return [before, norm.scaled()]


def test_TwoSlopeNorm_scale():
    probes.probe("colors.TwoSlopeNorm.scaled", _scaled, params={"vcenter": 2}, values=[1, 2, 3, 4])
    norm_call("TwoSlopeNorm", {"vcenter": 2}, [1, 2, 3, 4])


def test_TwoSlopeNorm_scaleout_center():
    norm_call("TwoSlopeNorm", {"vcenter": 0}, [0, 1, 2, 3, 5])


def test_TwoSlopeNorm_scaleout_center_max():
    norm_call("TwoSlopeNorm", {"vcenter": 0}, [0, -1, -2, -3, -5])


def test_TwoSlopeNorm_Even():
    norm_call("TwoSlopeNorm", {"vmin": -1, "vcenter": 0, "vmax": 4},
              [-1.0, -0.5, 0.0, 1.0, 2.0, 3.0, 4.0])


def test_TwoSlopeNorm_Odd():
    norm_call("TwoSlopeNorm", {"vmin": -2, "vcenter": 0, "vmax": 5},
              [-2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0])


def test_TwoSlopeNorm_VminEqualsVcenter():
    _two_slope_error(-2, -2, 2)


def test_TwoSlopeNorm_VmaxEqualsVcenter():
    _two_slope_error(-2, 2, 2)


def test_TwoSlopeNorm_VminGTVcenter():
    _two_slope_error(10, 0, 20)


def test_TwoSlopeNorm_TwoSlopeNorm_VminGTVmax():
    _two_slope_error(10, 0, 5)


def test_TwoSlopeNorm_VcenterGTVmax():
    _two_slope_error(10, 25, 20)


def test_TwoSlopeNorm_premature_scaling():
    assert isinstance(norm_inverse("TwoSlopeNorm", {"vcenter": 2}, [0.1, 0.5, 0.9]), ValueError)


def test_SymLogNorm():
    vals = [-30.0, -1.0, 2.0, 6.0]
    params = {"linthresh": 3, "vmax": 5, "linscale": 1.2, "base": E}
    result = norm_call("SymLogNorm", params, vals)
    norm_inverse("SymLogNorm", {**params, "vmin": -30}, result["result"].tolist())
    for v in vals:
        norm_call("SymLogNorm", params, v)
    norm_call("SymLogNorm", params, vals, mask=[True, False, False, False])
    norm_call("SymLogNorm", {**params, "vmin": -30}, vals)
    e = [-E ** 3, -E ** 2, -E, -1, 0, 1, E, E ** 2, E ** 3]
    norm_call("SymLogNorm", {"linthresh": 1, "vmin": -E ** 3, "vmax": E ** 3, "base": E}, e)
    tens = [-10 ** 3, -10 ** 2, -10, -1, 0, 1, 10, 10 ** 2, 10 ** 3]
    norm_call("SymLogNorm", {"linthresh": 1, "vmin": -10 ** 3, "vmax": 10 ** 3, "base": 10}, tens)


def _colorbar_ticks(norm):
    fig = Figure()
    cbar = mcolorbar.ColorbarBase(fig.add_subplot(), norm=NORMS[norm["class"]](**norm["params"]))
    return cbar.get_ticks()


def test_SymLogNorm_colorbar():
    probes.probe("colorbar.Colorbar.get_ticks", _colorbar_ticks,
                 norm={"class": "SymLogNorm",
                       "params": {"linthresh": 0.1, "vmin": -1, "vmax": 1, "linscale": 1, "base": E}})


def test_SymLogNorm_single_zero():
    probes.probe("colorbar.Colorbar.get_ticks", _colorbar_ticks,
                 norm={"class": "SymLogNorm", "params": {"linthresh": 1e-5, "vmin": -1, "vmax": 1, "base": E}})


def test_asinh_norm():
    norm_call("AsinhNorm", {"linear_width": 2, "vmin": -4, "vmax": 4}, np.arange(-3.5, 3.5, 10).tolist())


def test_multi_norm_creation():
    for norms in ("linear", [], None, ["linear", "bad_norm_name"], ["linear", "linear"]):
        probes.probe("colors.MultiNorm", _multi_attrs, norms=norms)


def test_multi_norm_call_vmin_vmax():
    lin_log = ["linear", "log"]
    probes.probe("colors.MultiNorm", _multi_attrs, norms=lin_log, vmin=[1, 1], vmax=[2, 2])
    for key, value in (("vmin", 1), ("vmax", 1), ("vmin", [1, 2, 3]), ("vmax", [1, 2, 3])):
        error = probes.probe("colors.MultiNorm", _multi_attrs, norms=lin_log, **{key: value})
        assert isinstance(error, ValueError)


def test_multi_norm_call_clip_inverse():
    lin_log = ["linear", "log"]
    limits = {"vmin": [1, 1], "vmax": [2, 2]}
    for clip in ([False, False], [True, True], [True, False]):
        probes.probe("colors.MultiNorm.__call__", _multi_call, norms=lin_log, values=[3, 3],
                     call_clip=clip, **limits)
        probes.probe("colors.MultiNorm.__call__", _multi_call, norms=lin_log, values=[3, 3], clip=clip,
                     **limits)
    for clip in (True, [True, False, True]):
        error = probes.probe("colors.MultiNorm", _multi_attrs, norms=lin_log, clip=clip, **limits)
        assert isinstance(error, ValueError)
        error = probes.probe("colors.MultiNorm.__call__", _multi_call, norms=lin_log, values=[3, 3],
                             call_clip=clip, **limits)
        assert isinstance(error, ValueError)
    probes.probe("colors.MultiNorm.inverse", _multi_inverse, norms=lin_log, values=[0.5, 0.5849625007211562],
                 **limits)


def test_multi_norm_autoscale():
    lin_log = ["linear", "log"]
    probes.probe("colors.MultiNorm.autoscale", _multi_autoscale("autoscale"), norms=lin_log,
                 data=[[0, 1, 2, 3], [0.1, 1, 2, 3]])
    two_slope = {"class": "TwoSlopeNorm", "params": {"vcenter": 2, "vmin": 0, "vmax": None}}
    probes.probe("colors.MultiNorm.autoscale_None", _multi_autoscale("autoscale_None"),
                 norms=[two_slope, "linear"], vmax=[None, 50], data=[[1, 2, 3, 4, 5], [-50, 1, 0, 1, 500]])
    probes.probe("colors.MultiNorm.__call__", _multi_call, norms=[two_slope, "linear"], vmin=[0, -50],
                 vmax=[5, 50], values=[5, 0])


def test_multi_norm_call_types():
    limits = {"vmin": [-2, -2], "vmax": [2, 2]}
    lin2 = ["linear", "linear"]
    for values in ([[0, 2, 4], [1, 3, 5]], [0, 1], [[0, 0, 0], [0, 0, 0]], [0, 0], 1, [3, 2, 1],
                   [[0, 0], [0, 0], [0, 0]], [0, 0, 0]):
        probes.probe("colors.MultiNorm.__call__", _multi_call, norms=lin2, values=values, **limits)
    probes.probe("colors.MultiNorm.__call__", _multi_call,
                 norms=["linear", {"class": "NoNorm", "params": {}}], values=[[0, 2, 4], [1, 3, 5]])


def test_colorizer_multinorm():
    op = "colorizer.Colorizer.to_rgba"
    bi = {"cmap": "BiOrangeBlue", "vmin": [0, 0], "vmax": [1, 1]}
    for values in ([0.1, 0.2], [[0.1, 0.2], [0.3, 0.4]], [0.1, 0.2, 0.3], [[0.1]]):
        probes.probe(op, _colorizer_to_rgba, values=values, **bi)
    probes.probe(op, _colorizer_to_rgba, cmap="3VarAddA", values=[0.1, 0.1, 0.1], vmin=[-0.1, -0.2, -0.3],
                 vmax=[0.1, 0.2, 0.3])
    for norm in ("linear", ["linear", "linear", "log"]):
        error = probes.probe(op, _colorizer_to_rgba, cmap="BiOrangeBlue", values=[0.1, 0.2],
                             norm=norm if isinstance(norm, list) else [norm])
        assert isinstance(error, ValueError)
    # test_colorizer_multinorm_explicit: one norm per variate needs a
    # multivariate colormap.
    error = probes.probe(op, _colorizer_to_rgba, cmap="viridis", values=[0.1, 0.2],
                         norm=["linear", "linear"])
    assert isinstance(error, TypeError)
    probes.probe(op, _colorizer_to_rgba, cmap="BiOrangeBlue", values=[0.1, 0.2],
                 norm=[{"class": "Normalize", "params": {}}, "log"], vmin=[0, 0.01], vmax=[1, 1])


def _colorizer_limits(vmin=None, vmax=None):
    colorizer = mcolorizer.Colorizer()
    if vmin is not None:
        colorizer.vmin = vmin
    if vmax is not None:
        colorizer.vmax = vmax
    return {"vmin": colorizer.vmin, "vmax": colorizer.vmax, "norm_vmin": colorizer.norm.vmin,
            "norm_vmax": colorizer.norm.vmax}


def test_colorizer_vmin_vmax():
    probes.probe("colorizer.Colorizer", _colorizer_limits)
    probes.probe("colorizer.Colorizer", _colorizer_limits, vmin=1, vmax=3)


def _levels_with_extremes(levels, colors, extend, bad, values, masked=False):
    cmap, norm = mcolors.from_levels_and_colors(levels, [_spec(c) for c in colors], extend=extend)
    cmap = cmap.with_extremes(bad=_spec(bad))
    data = np.ma.array(values, mask=True) if masked else np.asarray(values, float)
    return cmap(norm(data))


def test_cmap_and_norm_from_levels_and_colors2():
    levels = [-1, 2, 2.5, 3]
    colors = ["red", [0, 1, 0], "blue", [0.5, 0.5, 0.5], [0.0, 0.0, 0.0, 1.0]]
    bad = [0.1, 0.1, 0.1, 0.1]
    for extend, i1 in (("both", None), ("min", -1), ("max", -1), ("neither", -2)):
        for masked in (False, True):
            probes.probe("colors.from_levels_and_colors.with_extremes", _levels_with_extremes,
                         levels=levels, colors=colors[0:i1], extend=extend, bad=bad,
                         values=[1] if masked else [-2, -1, 2, 2.25, 3, 3.5], masked=masked)
    probes.probe("colors.to_rgba_array", to_rgba_array, c=colors)
    error = probes.probe("colors.from_levels_and_colors.with_extremes", _levels_with_extremes,
                         levels=levels, colors=colors, extend="neither", bad=bad, values=[0.0])
    assert isinstance(error, ValueError)


# ---------------------------------------------------------------------------
# Conversions
# ---------------------------------------------------------------------------


def test_rgb_hsv_round_trip():
    for shape in ((100, 3), (1, 3), (3,)):
        tt = rng(0).uniform(int(np.prod(shape))).reshape(shape).tolist()
        probes.probe("colors.rgb_to_hsv", _rgb_to_hsv, arr=tt)
        probes.probe("colors.hsv_to_rgb", _hsv_to_rgb, hsv=tt)
        probes.probe("colors.hsv_to_rgb", _hsv_to_rgb, hsv=_rgb_to_hsv(tt).tolist())
        probes.probe("colors.rgb_to_hsv", _rgb_to_hsv, arr=_hsv_to_rgb(tt).tolist())


def test_rgb_to_hsv_int():
    probes.probe("colors.rgb_to_hsv", _rgb_to_hsv, arr=[0, 1, 0])


def test_hsv_to_rgb_int():
    probes.probe("colors.hsv_to_rgb", _hsv_to_rgb, hsv=[0, 1, 1])


def test_color_names():
    for name in ("blue", "xkcd:blue", "tab:blue"):
        probes.probe("colors.to_hex", to_hex, c=name)


def test_cn():
    for cycle, specs in (("cycler('color', ['blue', 'r'])", ["C0", "C1"]),
                         ("cycler('color', ['xkcd:blue', 'r'])", ["C0", "C1", "C10", "C11"]),
                         ("cycler('color', ['8e4585', 'r'])", ["C0"])):
        for spec in specs:
            probes.probe("colors.to_hex", _in_rc(to_hex), c=spec, rc={"axes.prop_cycle": cycle})
    probes.probe("colors.to_rgb", _in_rc(to_rgb), c="C0",
                 rc={"axes.prop_cycle": "cycler('color', ['8e4585', 'r'])"})


def test_conversions():
    for c in ("none", [], [".2", ".5", ".8"]):
        probes.probe("colors.to_rgba_array", to_rgba_array, c=c)
    probes.probe("colors.to_rgba", to_rgba, c=[1, 1, 1], alpha=.5)
    probes.probe("colors.to_rgba", to_rgba, c=".1", alpha=.5)
    probes.probe("colors.to_hex", to_hex, c=[.7, .7, .7])
    probes.probe("colors.to_hex", to_hex, c="#1234abcd", keep_alpha=True)


def test_to_rgba_array_single_str():
    probes.probe("colors.to_rgba_array", to_rgba_array, c="red")
    assert isinstance(probes.probe("colors.to_rgba_array", to_rgba_array, c="rgb"), ValueError)


def test_to_rgba_array_2tuple_str():
    probes.probe("colors.to_rgba_array", to_rgba_array, c=["k", "w"], tuple=True)


def test_to_rgba_array_alpha_array():
    error = probes.probe("colors.to_rgba_array", to_rgba_array, c=[[1, 1, 1]] * 5, ndarray=True,
                         alpha=[1, 1])
    assert isinstance(error, ValueError)
    probes.probe("colors.to_rgba_array", to_rgba_array, c=[[1, 1, 1]] * 2, ndarray=True, alpha=[0.5, 0.6])
    probes.probe("colors.to_rgba_array", to_rgba_array, c=["r", "g"], alpha=[0.5, 0.6])


def test_to_rgba_array_accepts_color_alpha_tuple():
    probes.probe("colors.to_rgba_array", to_rgba_array, c=["black", 0.9], tuple=True)


def test_to_rgba_array_explicit_alpha_overrides_tuple_alpha():
    probes.probe("colors.to_rgba_array", to_rgba_array, c=["black", 0.9], tuple=True, alpha=0.5)


def test_to_rgba_array_error_with_color_invalid_alpha_tuple():
    assert isinstance(probes.probe("colors.to_rgba_array", to_rgba_array, c=["black", 2.0], tuple=True),
                      ValueError)


def test_to_rgba_array_none_color_with_alpha_param():
    probes.probe("colors.to_rgba_array", to_rgba_array, c=["blue", "none"], alpha=[1, 1])


def test_to_rgba_accepts_color_alpha_tuple():
    for spec in (["white", 0.5], ["#ffffff", 0.5], ["#ffffff00", 0.5], [[1.0, 1.0, 1.0, 1.0], 0.5]):
        probes.probe("colors.to_rgba", to_rgba, c=spec)


def test_to_rgba_explicit_alpha_overrides_tuple_alpha():
    probes.probe("colors.to_rgba", to_rgba, c=["red", 0.1], alpha=0.9)


def test_to_rgba_error_with_color_invalid_alpha_tuple():
    assert isinstance(probes.probe("colors.to_rgba", to_rgba, c=["blue", 2.0]), ValueError)


def test_failed_conversions():
    for spec in ("5", "-1", "nan", "unknown_color", 0.4):
        assert isinstance(probes.probe("colors.to_rgba", to_rgba, c=spec), ValueError)


def test_same_color():
    for c1, c2 in (("k", [0, 0, 0]), ("w", [1, 1, 0]), (["red", "blue"], ["r", "b"]), ("none", "none"),
                   ("none", "red"), (["r", "g", "b"], ["r"]), (["red", "green"], "none")):
        probes.probe("colors.same_color", same_color, c1=c1, c2=c2)


def test_hex_shorthand_notation():
    for c1, c2 in (("#123", "#112233"), ("#123a", "#112233aa")):
        probes.probe("colors.same_color", same_color, c1=c1, c2=c2)


def test_is_color_like():
    for spec in ("red", ["red", 0.5], ["red", 2], ["red", "blue"], "C3", ["C3", 0.5]):
        probes.probe("colors.is_color_like", is_color_like, c=spec)


def test_scalarmappable_to_rgba():
    rgba = np.full((2, 3, 4), 0.5)
    for as_bytes in (False, True):
        probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap=None, norm=None,
                     values=rgba.tolist(), bytes=as_bytes)
        probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap=None, norm=None,
                     values=rgba[..., :3].tolist(), bytes=as_bytes)


def test_scalarmappable_nan_to_rgba():
    x = np.full((2, 3, 4), 0.5)
    x[0, 0, 0] = np.nan
    for as_bytes in (False, True):
        probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap=None, norm=None,
                     values=x.tolist(), bytes=as_bytes)
        probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap=None, norm=None,
                     values=x[..., :3].tolist(), bytes=as_bytes)
        y = x.copy()
        y[1, 0, 0] = 42
        error = probes.probe("cm.ScalarMappable.to_rgba", scalar_mappable_to_rgba, cmap=None, norm=None,
                             values=y[..., :3].tolist(), bytes=as_bytes)
        assert isinstance(error, ValueError)


def _register_sequence(name, colors):
    mpl.color_sequences.register(name, colors)
    mpl.color_sequences.unregister(name)


def _get_sequence(name, register=None):
    if register is not None:
        mpl.color_sequences.register(name, register)
    try:
        return [mcolors.to_rgba(c) for c in mpl.color_sequences[name]]
    finally:
        if register is not None:
            mpl.color_sequences.unregister(name)


def _unregister_sequence(name, times):
    for _ in range(times):
        mpl.color_sequences.unregister(name)


def test_color_sequences():
    probes.probe("colors.color_sequences", lambda: list(mpl.color_sequences))
    for name in ("tab10", "tab20"):
        probes.probe("colors.color_sequence", _color_sequence, name=name)
    for name, colors in (("tab10", ["r", "g", "b"]), ("invalid", ["not a color"]), ("rgb", ["r", "g", "b"])):
        probes.probe("color_sequences.register", _register_sequence, name=name, colors=colors)
    probes.probe("color_sequences.__getitem__", _get_sequence, name="rgb", register=["r", "g", "b"])
    assert isinstance(probes.probe("color_sequences.__getitem__", _get_sequence, name="rgb"), KeyError)
    for name, times in (("rgb", 2), ("tab10", 1)):
        probes.probe("color_sequences.unregister", _unregister_sequence, name=name, times=times)


# ---------------------------------------------------------------------------
# Shading
# ---------------------------------------------------------------------------


def _bump():
    y, x = np.mgrid[-1.2:1.2:8j, -1.2:1.2:8j]
    return (10 * np.cos(x ** 2 + y ** 2)).tolist()


def test_light_source_shading_default():
    probes.probe("lightsource.shade", _shade, light={"azdeg": 315, "altdeg": 45}, data=_bump(),
                 cmap="copper", blend_mode="hsv", relief=DEFAULT_RELIEF)


def test_light_source_hillshading():
    y, x = np.mgrid[5:0:-1, :5]
    cone = (-np.hypot(x - x.mean(), y - y.mean())).tolist()
    for az in range(0, 390, 30):
        for elev in range(0, 105, 15):
            probes.probe("lightsource.hillshade", _hillshade, light={"azdeg": az, "altdeg": elev},
                         elevation=cone, relief=DEFAULT_RELIEF)


def _azimuth2math(azimuth, elevation):
    return np.radians((90 - azimuth) % 360), np.radians(90 - elevation)


def _plane(azimuth, elevation, angle, x, y):
    elevation = elevation + angle
    if elevation > 90:
        azimuth = (azimuth + 180) % 360
        elevation = (90 - elevation) % 90
    theta, phi = _azimuth2math(azimuth, elevation)
    a, b, c = np.cos(theta) * np.sin(phi), np.sin(theta) * np.sin(phi), np.cos(phi)
    return -(a * x + b * y) / c


def test_light_source_planar_hillshading():
    y, x = np.mgrid[5:0:-1, :5]
    for az in (0, 90, 210, 330):
        for elev in (0, 45, 75):
            for angle in range(0, 105, 15):
                probes.probe("lightsource.hillshade", _hillshade, light={"azdeg": az, "altdeg": elev},
                             elevation=_plane(az, elev, angle, x, y).tolist(), relief=DEFAULT_RELIEF)
