"""Multivariate and bivariate colormaps (``matplotlib.colors.MultivarColormap``,
``BivarColormap`` and the ``multivar_colormaps`` and ``bivar_colormaps``
registries).

Area ``multivar``. In ``X``, ``X0`` and ``X1`` floats are fractions, ints are
table indices and ``None`` is a bad value (NaN); each variate is all floats or
all ints. Colours are ``[r, g, b, a]`` floats.

Ops
---
``multivar.call``      {name, X, alpha?, clip?, bad?, under?, over?} -> [rgba, ...]
    ``multivar_colormaps[name]``, through ``with_extremes(bad=, under=, over=)``
    when given, called on ``X`` (one list per variate).
``bivar.call``         {name, X0, X1, alpha?, shape?, origin?, bad?, outside?} -> [rgba, ...]
    ``bivar_colormaps[name]``, through ``with_extremes(...)`` when given.
``bivar.lut``          {name, resampled, transposed?, reversed?, shape?, base?} -> [[rgba, ...], ...]
    ``.resampled(resampled, transposed)`` (after ``.resampled(base)`` and
    ``.reversed(*reversed)``), then its ``lut``.
``bivar.component``    {name, axis, shape?, origin?} -> {name, lut, under, over, bad}
    ``cmap[axis]``: the 1-D colormap through the origin; ``lut`` holds every
    15th colour.
"""

import numpy as np
from matplotlib import bivar_colormaps, multivar_colormaps

from spec_support import ProbeSet

probes = ProbeSet("multivar")

FRACTIONS = [-0.2, 0.0, 0.1, 0.25, 1 / 3, 0.5, 0.75, 0.9, 0.999, 1.0, 1.2, None]
INDICES = [-3, 0, 1, 100, 254, 255, 256, 300]


def _values(values):
    if all(isinstance(v, int) for v in values):
        return np.array(values)
    return np.array([np.nan if v is None else v for v in values], dtype=float)


def _rgba(colors):
    return [[float(c) for c in color] for color in np.asarray(colors).reshape(-1, 4)]


def _multivar(name, X, alpha=None, clip=True, bad=None, under=None, over=None):
    cmap = multivar_colormaps[name]
    if bad is not None or under is not None or over is not None:
        cmap = cmap.with_extremes(bad=bad, under=under, over=over)
    return _rgba(cmap(tuple(_values(x) for x in X), alpha=alpha, clip=clip))


def _bivar_cmap(name, shape=None, origin=None, bad=None, outside=None):
    cmap = bivar_colormaps[name]
    if shape is not None or origin is not None or bad is not None or outside is not None:
        cmap = cmap.with_extremes(bad=bad, outside=outside, shape=shape, origin=origin)
    return cmap


def _bivar(name, X0, X1, alpha=None, **extremes):
    cmap = _bivar_cmap(name, **extremes)
    return _rgba(cmap((_values(X0), _values(X1)), alpha=alpha))


def _bivar_lut(name, resampled, transposed=False, reversed=None, shape=None, base=None):
    cmap = _bivar_cmap(name, shape=shape)
    if base is not None:
        cmap = cmap.resampled(tuple(base))
    if reversed is not None:
        cmap = cmap.reversed(*reversed)
    lut = cmap.resampled(tuple(resampled), transposed=transposed).lut
    return [_rgba(row) for row in lut]


def _bivar_component(name, axis, shape=None, origin=None):
    part = _bivar_cmap(name, shape=shape, origin=origin)[axis]
    return {
        "name": part.name,
        "lut": _rgba(part(np.arange(0, part.N, 15))),
        "under": _rgba(part.get_under())[0],
        "over": _rgba(part.get_over())[0],
        "bad": _rgba(part.get_bad())[0],
    }


def test_multivariate():
    for name in multivar_colormaps:
        n = multivar_colormaps[name].n_variates
        fractions = [FRACTIONS[i:] + FRACTIONS[:i] for i in range(n)]
        probes.probe("multivar.call", _multivar, name=name, X=fractions)
        probes.probe("multivar.call", _multivar, name=name, X=fractions, clip=False)
        probes.probe("multivar.call", _multivar, name=name, X=fractions, alpha=0.4)
        indices = [INDICES[i:] + INDICES[:i] for i in range(n)]
        probes.probe("multivar.call", _multivar, name=name, X=indices)
        probes.probe("multivar.call", _multivar, name=name, X=fractions, bad="r",
                     under=["k"] * n, over=["w"] * n)
        probes.probe("multivar.call", _multivar, name=name, X=fractions[:1])


def test_bivariate():
    pairs = [(a, b) for a in FRACTIONS for b in FRACTIONS[::3]]
    x0 = [a for a, _ in pairs]
    x1 = [b for _, b in pairs]
    for name in bivar_colormaps:
        probes.probe("bivar.call", _bivar, name=name, X0=x0, X1=x1)
        probes.probe("bivar.call", _bivar, name=name, X0=x0, X1=x1, alpha=0.3)
        for shape in ["square", "circle", "ignore", "circleignore"]:
            probes.probe("bivar.call", _bivar, name=name, X0=x0, X1=x1, shape=shape)
        probes.probe("bivar.call", _bivar, name=name, X0=INDICES, X1=INDICES[::-1])
        probes.probe("bivar.call", _bivar, name=name, X0=INDICES, X1=INDICES[::-1],
                     shape="ignore", bad="k", outside="y")
        probes.probe("bivar.call", _bivar, name=name, X0=INDICES, X1=FRACTIONS[:8],
                     shape="circle")
        for resampled in [[5, 7], [-4, 3]]:
            probes.probe("bivar.lut", _bivar_lut, name=name, resampled=resampled)
        for resampled in [[3, -1], [1, 4], [None, -2]]:
            probes.probe("bivar.lut", _bivar_lut, name=name, resampled=resampled, base=[4, 5])
        probes.probe("bivar.lut", _bivar_lut, name=name, resampled=[4, 6], transposed=True)
        probes.probe("bivar.lut", _bivar_lut, name=name, resampled=[1, 1], base=[5, 5],
                     reversed=[True, False])
        probes.probe("bivar.lut", _bivar_lut, name=name, resampled=[6, 6], shape="circle")
        for axis in [0, 1]:
            probes.probe("bivar.component", _bivar_component, name=name, axis=axis)
            probes.probe("bivar.component", _bivar_component, name=name, axis=axis,
                         shape="ignore", origin=[0.3, 1.0])
