"""Colormaps: the builtin registry, sampling, and colormap construction.

Two areas are recorded:

* ``colormaps`` — behaviour of every registered colormap (and its ``_r``
  reverse) plus colormap construction/derivation APIs.
* ``colormap_luts`` — the complete lookup table of every builtin base
  colormap, so pltrs can verify its embedded data bit-for-bit.

Argument conventions
--------------------
* ``name`` is a key of ``matplotlib.colormaps``; the registry returns a copy.
* ``x`` (float samples) is ``np.asarray(x, dtype=float)``; float sampling
  maps ``x`` to index ``int(x * N)`` with ``x == 1`` mapped to ``N - 1``,
  negative values to the *under* colour, ``>= N`` (after scaling) to *over*
  and NaN to *bad*.
* ``i`` (integer samples) is ``np.asarray(i, dtype=int)``: direct LUT indices,
  ``< 0`` -> under, ``>= N`` -> over.
* Colour arguments (``bad``/``under``/``over``, ``colors`` elements) follow the
  ``colors`` area convention: JSON arrays become tuples. ``from_list`` entries
  may be ``[value, colour]`` pairs.
* RGBA results are lists of ``[r, g, b, a]`` floats (or uint8 ints for the
  ``bytes`` variant).

Ops (area ``colormaps``)
------------------------
``colormaps.names``          {} -> registry order of all names (incl. ``_r``)
``colormaps.info``           {name} -> {class, name, N, is_gray, colorbar_extend}
``colormaps.segmentdata``    {name} -> LinearSegmentedColormap segment data:
                             {channel: [[x, y0, y1], ...] | "callable:<desc>"};
                             ListedColormap -> {"listed": N}
``colormaps.sample``         {name, x} -> cmap(x)
``colormaps.sample_index``   {name, i} -> cmap(i)
``colormaps.sample_bytes``   {name, x} -> cmap(x, bytes=True); LUT bytes are
                             ``(lut * 255).astype(uint8)`` (truncation)
``colormaps.sample_alpha``   {name, x, alpha, bytes?} -> cmap(x, alpha=alpha, bytes=bytes)
``colormaps.extremes``       {name} -> {under, over, bad} (get_under/get_over/get_bad)
``colormaps.resampled``      {name, lutsize, x} -> {N, sample, under, over, bad}
``colormaps.reversed``       {name, x} -> {name, N, sample, under, over, bad} of cmap.reversed()
``colormaps.with_extremes``  {name, x, bad?, under?, over?, reverse?} -> {sample, under, over, bad};
                             when ``reverse`` the result is ``.reversed()`` afterwards
``colormaps.with_alpha``     {name, alpha, x} -> {sample, under, over, bad}
``colormaps.equal``          {a, b} -> ``colormaps[a] == colormaps[b]`` (bit-identical LUT
                             including under/over/bad, same colorbar_extend)
``colormaps.get_cmap``       {cmap} -> {name, N} of ``matplotlib.colormaps.get_cmap(cmap)``;
                             ``cmap`` null means rcParams["image.cmap"]
``pyplot.get_cmap``          {name, lut} -> {name, N, sample(linspace(0, 1, 17))}
``colors.ListedColormap``    {colors, name?, N?, bad?, under?, over?} ->
                             {name, N, lut, under, over, bad, monochrome, is_gray}
``colors.LinearSegmentedColormap`` {segmentdata, N?, gamma?} -> {N, lut, under, over, bad}
``colors.LinearSegmentedColormap.from_list``
                             {colors, N?, gamma?, bad?, under?, over?} -> {N, lut, under, over, bad}
``colors.LinearSegmentedColormap.from_list.resampled``
                             {colors, N, gamma, lutsize} -> {N, lut} of
                             ``from_list(...).resampled(lutsize)`` (note: gamma is dropped)

Ops (area ``colormap_luts``)
----------------------------
``colormaps.lut``            {name} -> cmap(np.arange(cmap.N)): the full N x RGBA table
"""

import functools

import matplotlib as mpl
import matplotlib.colors as mcolors
import numpy as np
import pytest

from spec_support import ProbeSet

probes = ProbeSet("colormaps")
luts = ProbeSet("colormap_luts")

ALIASES = {"grey": "gray", "gist_grey": "gist_gray", "gist_yerg": "gist_yarg", "Grays": "Greys"}

#: 17 evenly spaced points plus out-of-range and invalid values.
SAMPLE_X = np.linspace(0, 1, 17).tolist() + [-0.5, 1.5, float("nan")]

EDGE_X = [float(v) for v in [
    0.0,
    -1e-12,
    1e-12,
    1 / 512,
    1 / 256,
    np.nextafter(1 / 256, 0),
    0.5,
    np.nextafter(0.5, 0),
    0.999,
    np.nextafter(1.0, 0),
    1.0,
    1.0 + 1e-12,
    np.nextafter(1.0, 2),
    2.0,
    -1.0,
    float("inf"),
    float("-inf"),
    float("nan"),
]]

REPRESENTATIVE = ["viridis", "viridis_r", "gray", "jet", "jet_r", "tab10", "tab10_r", "twilight",
                  "Blues", "gnuplot2", "flag", "cubehelix", "Set1", "hsv", "RdBu_r", "binary"]


def _spec(value):
    if isinstance(value, list):
        return tuple(_spec(v) for v in value)
    return value


def _rgba_state(cmap):
    return {"under": cmap.get_under(), "over": cmap.get_over(), "bad": cmap.get_bad()}


def _describe_segment(data):
    if callable(data):
        return f"callable:{_describe_callable(data)}"
    return np.asarray(data, dtype=float)


def _describe_callable(func):
    if isinstance(func, functools.partial):
        inner = _describe_callable(func.func)
        if func.func is mcolors.LinearSegmentedColormap._reverser:
            return f"reversed({_describe_callable(func.args[0])})"
        params = ", ".join(repr(a) for a in func.args)
        return f"{inner}({params})"
    return func.__name__


def _base_names():
    return [n for n in mpl.colormaps if not n.endswith("_r") and n not in ALIASES]


# --------------------------------------------------------------------------
# Adapters
# --------------------------------------------------------------------------


def names():
    return list(mpl.colormaps)


def info(name):
    cmap = mpl.colormaps[name]
    return {"class": type(cmap).__name__, "name": cmap.name, "N": cmap.N,
            "is_gray": bool(cmap.is_gray()), "colorbar_extend": cmap.colorbar_extend}


def segmentdata(name):
    cmap = mpl.colormaps[name]
    if isinstance(cmap, mcolors.ListedColormap):
        return {"listed": cmap.N}
    return {key: _describe_segment(value) for key, value in cmap._segmentdata.items()}


def sample(name, x):
    return mpl.colormaps[name](np.asarray(x, dtype=float))


def sample_index(name, i):
    return mpl.colormaps[name](np.asarray(i, dtype=int))


def sample_bytes(name, x):
    return mpl.colormaps[name](np.asarray(x, dtype=float), bytes=True)


def sample_alpha(name, x, alpha, bytes=False):  # noqa: A002 - mirrors matplotlib's kwarg
    if isinstance(alpha, list):
        alpha = np.asarray(alpha, dtype=float)
    return mpl.colormaps[name](np.asarray(x, dtype=float), alpha=alpha, bytes=bytes)


def extremes(name):
    return _rgba_state(mpl.colormaps[name])


def resampled(name, lutsize, x):
    cmap = mpl.colormaps[name].resampled(lutsize)
    return {"N": cmap.N, "sample": cmap(np.asarray(x, dtype=float)), **_rgba_state(cmap)}


def reversed_(name, x):
    cmap = mpl.colormaps[name].reversed()
    return {"name": cmap.name, "N": cmap.N, "sample": cmap(np.asarray(x, dtype=float)), **_rgba_state(cmap)}


def with_extremes(name, x, bad=None, under=None, over=None, reverse=False):
    cmap = mpl.colormaps[name].with_extremes(bad=_spec(bad), under=_spec(under), over=_spec(over))
    if reverse:
        cmap = cmap.reversed()
    return {"sample": cmap(np.asarray(x, dtype=float)), **_rgba_state(cmap)}


def with_alpha(name, alpha, x):
    cmap = mpl.colormaps[name].with_alpha(alpha)
    return {"sample": cmap(np.asarray(x, dtype=float)), **_rgba_state(cmap)}


def equal(a, b):
    return bool(mpl.colormaps[a] == mpl.colormaps[b])


def get_cmap(cmap):
    result = mpl.colormaps.get_cmap(cmap)
    return {"name": result.name, "N": result.N}


def pyplot_get_cmap(name, lut):
    import matplotlib.pyplot as plt

    cmap = plt.get_cmap(name, lut)
    return {"name": cmap.name, "N": cmap.N, "sample": cmap(np.linspace(0, 1, 17))}


def _table(cmap):
    return {"N": cmap.N, "lut": cmap(np.arange(cmap.N)), **_rgba_state(cmap)}


def listed(colors, name="unnamed", N=None, bad=None, under=None, over=None):
    kwargs = {} if N is None else {"N": N}
    cmap = mcolors.ListedColormap([_spec(c) for c in colors] if isinstance(colors, list) else colors,
                                  name=name, bad=_spec(bad), under=_spec(under), over=_spec(over), **kwargs)
    return {"name": cmap.name, **_table(cmap), "monochrome": bool(cmap.monochrome),
            "is_gray": bool(cmap.is_gray())}


def segmented(segmentdata, N=256, gamma=1.0):
    cmap = mcolors.LinearSegmentedColormap("probe", segmentdata, N, gamma)
    return _table(cmap)


def from_list(colors, N=256, gamma=1.0, bad=None, under=None, over=None):
    cmap = mcolors.LinearSegmentedColormap.from_list(
        "probe", [_spec(c) for c in colors], N, gamma, bad=_spec(bad), under=_spec(under), over=_spec(over))
    return _table(cmap)


def from_list_resampled(colors, N, gamma, lutsize):
    cmap = mcolors.LinearSegmentedColormap.from_list("probe", [_spec(c) for c in colors], N, gamma)
    cmap = cmap.resampled(lutsize)
    return {"N": cmap.N, "lut": cmap(np.arange(cmap.N))}


def lut(name):
    cmap = mpl.colormaps[name]
    return cmap(np.arange(cmap.N))


# --------------------------------------------------------------------------
# Registry-wide probes
# --------------------------------------------------------------------------


def test_registry_names():
    all_names = probes.probe("colormaps.names", names)
    assert len(all_names) == 2 * len(set(n.removesuffix("_r") for n in all_names))
    assert "viridis" in all_names and "viridis_r" in all_names


@pytest.mark.parametrize("name", list(mpl.colormaps))
def test_every_colormap(name):
    probes.probe("colormaps.info", info, name=name)
    probes.probe("colormaps.sample", sample, name=name, x=SAMPLE_X)
    n = mpl.colormaps[name].N
    probes.probe("colormaps.sample_index", sample_index, name=name, i=[0, 1, n // 2, n - 2, n - 1, n, n + 1, -1, -2])


@pytest.mark.parametrize("name", _base_names())
def test_base_colormap_data(name):
    probes.probe("colormaps.segmentdata", segmentdata, name=name)
    table = luts.probe("colormaps.lut", lut, name=name)
    assert table.shape == (mpl.colormaps[name].N, 4)


def test_reversed_luts_sampled():
    # Reversed segmented maps re-evaluate the data at 1 - x, so their LUTs can
    # differ from the flipped base LUT by an ulp; record a few in full.
    for name in ["viridis_r", "jet_r", "gnuplot2_r", "flag_r", "cubehelix_r", "Blues_r", "tab10_r", "twilight_r"]:
        luts.probe("colormaps.lut", lut, name=name)


def test_aliases():
    for alias, original in ALIASES.items():
        assert probes.probe("colormaps.equal", equal, a=alias, b=original)
        probes.probe("colormaps.equal", equal, a=alias + "_r", b=original + "_r")
    for a, b in [("viridis", "viridis_r"), ("gray", "binary_r"), ("gray", "Greys_r"), ("gist_yarg", "binary"),
                 ("jet", "jet"), ("twilight", "twilight_shifted")]:
        probes.probe("colormaps.equal", equal, a=a, b=b)


# --------------------------------------------------------------------------
# Sampling variants
# --------------------------------------------------------------------------


def test_sample_edges():
    for name in REPRESENTATIVE:
        probes.probe("colormaps.extremes", extremes, name=name)
        probes.probe("colormaps.sample", sample, name=name, x=EDGE_X)
        probes.probe("colormaps.sample", sample, name=name, x=[])
        probes.probe("colormaps.sample_bytes", sample_bytes, name=name, x=SAMPLE_X)
        probes.probe("colormaps.sample_index", sample_index, name=name, i=[0, 5, 10, 1000, -1000])
    probes.probe("colormaps.sample", sample, name="viridis", x=[[0.0, 0.5], [1.0, float("nan")]])


def test_sample_alpha():
    x = [0.0, 0.25, 0.5, 1.0, -1.0, 2.0, float("nan")]
    for name in ["viridis", "tab10", "gray"]:
        for alpha in [0.0, 0.5, 1.0, 1.5, -0.5]:
            probes.probe("colormaps.sample_alpha", sample_alpha, name=name, x=x, alpha=alpha)
            probes.probe("colormaps.sample_alpha", sample_alpha, name=name, x=x, alpha=alpha, bytes=True)
        probes.probe("colormaps.sample_alpha", sample_alpha, name=name, x=x, alpha=[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6])
        probes.probe("colormaps.sample_alpha", sample_alpha, name=name, x=x, alpha=[0.5, 0.5])


# --------------------------------------------------------------------------
# Derived colormaps
# --------------------------------------------------------------------------


def test_resampled():
    for name in ["viridis", "viridis_r", "jet", "tab10", "twilight", "cubehelix"]:
        for lutsize in [1, 2, 3, 5, 10, 300]:
            probes.probe("colormaps.resampled", resampled, name=name, lutsize=lutsize, x=SAMPLE_X)


def test_reversed():
    for name in ["viridis", "jet", "tab10", "gnuplot", "cubehelix", "Blues", "viridis_r", "jet_r"]:
        result = probes.probe("colormaps.reversed", reversed_, name=name, x=SAMPLE_X)
        assert result["name"] == name + "_r"


def test_with_extremes():
    x = [-0.5, 0.0, 0.5, 1.0, 1.5, float("nan")]
    variants = [
        dict(),
        dict(under="red"),
        dict(over="blue"),
        dict(bad="green"),
        dict(under="red", over="blue", bad="0.5"),
        dict(bad=[1, 0, 0, 0.5]),
        dict(bad=["red", 0.25]),
        dict(under="none", over="none"),
        dict(under="notacolor"),
    ]
    for name in ["viridis", "jet", "tab10"]:
        for kwargs in variants:
            probes.probe("colormaps.with_extremes", with_extremes, name=name, x=x, **kwargs)
            probes.probe("colormaps.with_extremes", with_extremes, name=name, x=x, reverse=True, **kwargs)
    reversed_map = with_extremes("viridis", x, under="red", over="blue", reverse=True)
    assert tuple(reversed_map["under"]) == (0.0, 0.0, 1.0, 1.0)


def test_with_alpha():
    x = [-0.5, 0.0, 0.5, 1.0, 1.5, float("nan")]
    for name in ["viridis", "tab10"]:
        for alpha in [0.0, 0.3, 1.0, 1.2, -0.1]:
            probes.probe("colormaps.with_alpha", with_alpha, name=name, alpha=alpha, x=x)


def test_get_cmap():
    for cmap in [None, "viridis", "jet_r", "grey", "Grays", "notacmap", "Viridis"]:
        probes.probe("colormaps.get_cmap", get_cmap, cmap=cmap)
    assert get_cmap(None)["name"] == "viridis"
    for name, lut_ in [(None, None), ("viridis", None), ("viridis", 8), ("jet", 5), ("tab10", 4), ("tab10", 20),
                       ("gray_r", 3), ("notacmap", None)]:
        probes.probe("pyplot.get_cmap", pyplot_get_cmap, name=name, lut=lut_)


def test_listed_colormap():
    cases = [
        dict(colors=["red", "green", "blue"]),
        dict(colors=["red", "green", "blue"], name="rgb"),
        dict(colors=[[1, 0, 0], [0, 1, 0, 0.5], "#0000ff80", "C3", "0.5"]),
        dict(colors=["red"]),
        dict(colors=["red", "red", "red"]),
        dict(colors=["0.2", "0.4", "0.6"]),
        dict(colors=["red", "green", "blue"], under="k", over="w", bad="0.5"),
        dict(colors=["red", "green", "blue"], N=5),
        dict(colors=["red", "green", "blue"], N=2),
        dict(colors="red", N=3),
        dict(colors=0.5, N=2),
        dict(colors=["red", "notacolor"]),
        dict(colors=[[0.1, 0.2, 0.3], [0.4, 0.5, 0.6], [0.7, 0.8, 0.9], [1.0, 1.0, 1.0]]),
    ]
    for case in cases:
        probes.probe("colors.ListedColormap", listed, **case)


def test_from_list():
    x_colors = ["red", "green", "blue"]
    cases = [
        dict(colors=x_colors),
        dict(colors=x_colors, N=3),
        dict(colors=x_colors, N=2),
        dict(colors=x_colors, N=1),
        dict(colors=x_colors, N=7),
        dict(colors=x_colors, N=16, gamma=2.0),
        dict(colors=x_colors, N=16, gamma=0.5),
        dict(colors=["white", "black"], N=11),
        dict(colors=["#ff000000", "#ff0000ff"], N=5),
        dict(colors=[[0.0, "red"], [0.25, "green"], [1.0, "blue"]], N=9),
        dict(colors=[[0.0, "red"], [0.5, "red"], [0.5, "blue"], [1.0, "blue"]], N=8),
        dict(colors=[[0.0, "red"], [0.9, "green"], [1.0, "blue"]], N=10, gamma=1.5),
        dict(colors=[[0.2, "red"], [1.0, "blue"]], N=5),
        dict(colors=[[0.0, "red"], [0.8, "blue"]], N=5),
        dict(colors=[[0.0, "red"], [0.6, "green"], [0.4, "blue"], [1.0, "k"]], N=5),
        dict(colors=[[-0.1, "red"], [1.0, "blue"]], N=5),
        dict(colors=["red"], N=4),
        dict(colors=[], N=4),
        dict(colors=["red", "notacolor"], N=4),
        dict(colors=["C0", "C1", "C2", "C3"], N=12, under="k", over="w", bad="r"),
        dict(colors=[[0.1, 0.2, 0.3], [0.9, 0.8, 0.7, 0.5]], N=6),
    ]
    for case in cases:
        probes.probe("colors.LinearSegmentedColormap.from_list", from_list, **case)
    table = from_list(x_colors, N=3)
    assert table["lut"].tolist() == [[1.0, 0.0, 0.0, 1.0], [0.0, 0.5019607843137255, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]]
    for lutsize in [3, 8, 256]:
        probes.probe("colors.LinearSegmentedColormap.from_list.resampled", from_list_resampled,
                     colors=x_colors, N=16, gamma=2.0, lutsize=lutsize)


def test_segmented():
    linear = {"red": [[0, 0, 0], [1, 1, 1]], "green": [[0, 0, 0], [1, 1, 1]], "blue": [[0, 0, 0], [1, 1, 1]]}
    stepped = {
        "red": [[0.0, 0.0, 0.0], [0.5, 1.0, 1.0], [1.0, 1.0, 1.0]],
        "green": [[0.0, 0.0, 0.0], [0.25, 0.0, 0.0], [0.75, 1.0, 1.0], [1.0, 1.0, 1.0]],
        "blue": [[0.0, 0.0, 0.0], [0.5, 0.0, 1.0], [1.0, 0.5, 0.5]],
    }
    with_alpha_channel = {**linear, "alpha": [[0, 1, 1], [1, 0, 0]]}
    sawtooth = {"red": [[0, 1, 0], [0.2, 1, 0], [0.4, 1, 0], [0.6, 1, 0], [0.8, 1, 0], [1, 1, 0]],
                "green": [[0, 0, 0], [1, 0, 0]], "blue": [[0, 0, 0], [1, 0, 0]]}
    overshoot = {"red": [[0, -0.5, -0.5], [1, 1.5, 1.5]], "green": [[0, 0, 0], [1, 0, 0]],
                 "blue": [[0, 0, 0], [1, 0, 0]]}
    bad_start = {"red": [[0.1, 0, 0], [1, 1, 1]], "green": [[0, 0, 0], [1, 1, 1]], "blue": [[0, 0, 0], [1, 1, 1]]}
    unsorted = {"red": [[0, 0, 0], [0.7, 1, 1], [0.3, 0, 0], [1, 1, 1]], "green": [[0, 0, 0], [1, 1, 1]],
                "blue": [[0, 0, 0], [1, 1, 1]]}
    for data in [linear, stepped, with_alpha_channel, sawtooth, overshoot, bad_start, unsorted]:
        for N, gamma in [(33, 1.0), (11, 1.0), (10, 2.0), (2, 1.0), (1, 1.0), (7, 0.3)]:
            probes.probe("colors.LinearSegmentedColormap", segmented, segmentdata=data, N=N, gamma=gamma)
    probes.probe("colors.LinearSegmentedColormap", segmented, segmentdata=stepped, N=256, gamma=1.0)
