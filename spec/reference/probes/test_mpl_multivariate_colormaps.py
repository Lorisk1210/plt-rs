"""matplotlib's own non-image tests of multivariate and bivariate
colormaps (``matplotlib/tests/test_multivariate_colormaps.py``) that the
``multivar`` area leaves out.

Area ``mpl_multivariate_colormaps``. Every op runs in the style
matplotlib's test fixture applies (``classic`` plus
``_classic_test_patch``).

Values are given per variate as lists: a list of integers indexes the
colormap's table, any other list holds fractions (``NaN`` is bad), as
numpy's dtypes decide in matplotlib.

Ops
---
``MultivarColormap``  {colormaps, mode, values} -> the colours of
    ``MultivarColormap(colormaps, mode)`` (registered colormap names) for
    ``values``, or the error for a single colormap or an unknown mode
    (``test_multivar_creation``, ``test_multivar_bad_mode``).
``MultivarColormap.__call__``  {name, values, alpha, bytes, extremes} ->
    ``cmap(values, alpha=alpha, bytes=bytes)`` of the registered colormap
    after ``with_extremes(**extremes)`` (``bad``, and ``under``/``over``,
    one colour per variate), or the error for values of the wrong number of
    variates or extremes of the wrong length (``test_multivar_cmap_call``).
``MultivarColormap.resampled``  {name, lutshape, values, component} ->
    ``cmap.resampled(lutshape)(values)`` and, for ``component = [i, x]``,
    ``cmap.resampled(lutshape)[i](x)``, or the error for a ``lutshape`` of
    the wrong length (``test_multivar_resample``).
``BivarColormap.__call__``  {cmap, values, alpha, bytes, extremes} ->
    ``cmap(values, alpha=alpha, bytes=bytes)`` after
    ``with_extremes(**extremes)`` (``bad``, ``outside``, ``shape``), where
    ``cmap`` names a registered colormap or a table of
    ``BivarColormapFromImage``: ``ramp`` (10 x 12 RGBA, red and green
    ramps), ``rgb`` (10 x 12 RGB floats) or ``rgb_u8`` (the same as bytes);
    or the error for three variates or indices into a circle
    (``test_bivar_cmap_call_tuple``, ``test_bivar_cmap_call``,
    ``test_bivar_cmap_from_image``).
``BivarColormap.lut``  {name} -> ten slices of ``cmap.lut``: the columns
    0, 255 of channel 0, 0, 153, 255 of channel 1, the rows 0, 102, 255 of
    channel 1 and 0, 255 of channel 2 (``test_bivar_cmap_lut_smooth``).
``BivarColormap.__getitem__``  {name, axis, origin, shape, values} ->
    ``cmap[axis].colors`` after ``with_extremes(origin=..., shape=...)``,
    or with ``values``, ``[cmap(values), cmap[axis](values[axis])]``
    (``test_bivar_cmap_1d_origin``, ``test_bivar_getitem``).
``BivarColormap.resampled``  {name, method, lutshape, values} -> the
    colours of ``cmap.resampled(lutshape)``, ``cmap.reversed()`` or
    ``cmap.transposed()`` for ``values`` (``test_bivar_resample``).
``BivarColormap.with_extremes``  {shape, image} -> the error of an unknown
    shape given to ``with_extremes`` or to ``BivarColormapFromImage``
    (``test_bivar_cmap_bad_shape``).
``Colormap.__eq__``  {a, b} -> ``a == b`` for colormaps described as
    ``{"name"}`` (registered), with ``bad``, ``outside``, ``shape``,
    ``lut_scale`` (the table times a factor) or ``{"colormaps", "mode"}``
    (``MultivarColormap`` of registered colormaps or ``[name, index]``
    components) (``test_bivar_eq``, ``test_multivar_eq``).

Covered elsewhere: ``test_bivariate_cmap_shapes`` and
``test_multivar_alpha_mixing`` (cases of ``mpl_rest``); the registered
colormaps' colours (``multivar``).

Left out: array ``alpha`` (pltrs takes one opacity per call), byte-swapped
integer arrays, ``clip=False`` with ``bytes`` and colormaps given as
something else than colormaps (Python dtypes and types),
``test_bivar_cmap_bad_lut`` (a table of five channels cannot be built), the
PNG table of ``test_bivar_cmap_from_image`` (a sample data file), and the
``_repr_png_``/``_repr_html_`` tests (Jupyter display hooks).
"""

import matplotlib as mpl
import numpy as np
from matplotlib.colors import BivarColormapFromImage, MultivarColormap

from probes._mpl_tests import prober
from spec_support import ProbeSet

probes = ProbeSet("mpl_multivariate_colormaps")
probe = prober(probes)

NAN = float("nan")


def _values(values):
    return [np.asarray(v) for v in values]


def _colors(rgba):
    return np.asarray(rgba).tolist()


def _image(name):
    if name == "ramp":
        im = np.ones((10, 12, 4))
        im[:, :, 0] = np.linspace(0, 1, 10)[:, np.newaxis]
        im[:, :, 1] = np.linspace(0, 1, 12)[np.newaxis, :]
        return im
    cim = np.ones((10, 12, 3))
    cim[:, :, 0] = np.arange(10)[:, np.newaxis] / 10
    cim[:, :, 1] = np.arange(12)[np.newaxis, :] / 12
    if name == "rgb_u8":
        cim = np.ones((10, 12, 3)) * 255
        cim[:, :, 0] = np.arange(10)[:, np.newaxis] / 10 * 255
        cim[:, :, 1] = np.arange(12)[np.newaxis, :] / 12 * 255
        return cim.astype(np.uint8)
    return cim


# --- multivariate colormaps ---------------------------------------------------


def multivar_op(colormaps, mode, values):
    cmap = MultivarColormap([mpl.colormaps[name] for name in colormaps], mode)
    return _colors(cmap(_values(values)))


def test_multivar_creation():
    y, x = np.mgrid[0:3, 0:3] / 2
    res = probe("MultivarColormap", multivar_op, colormaps=["Blues", "Oranges"],
                mode="sRGB_sub", values=[y.ravel().tolist(), x.ravel().tolist()])
    assert np.allclose(res[8], [0, 0, 0, 1], atol=0.01)
    assert isinstance(probe("MultivarColormap", multivar_op, colormaps=["Blues"],
                            mode="sRGB_sub", values=[[0.5]]), ValueError)


def test_multivar_bad_mode():
    assert isinstance(probe("MultivarColormap", multivar_op, colormaps=["Blues", "Oranges"],
                            mode="bad", values=[[0.5], [0.5]]), ValueError)


def multivar_call_op(name, values, alpha=None, bytes=False, extremes=None):
    cmap = mpl.multivar_colormaps[name]
    if extremes:
        cmap = cmap.with_extremes(**{key: (tuple(value) if key == "bad" else
                                           [tuple(c) for c in value])
                                     for key, value in extremes.items()})
    return _colors(cmap(_values(values), alpha=alpha, bytes=bytes))


def test_multivar_cmap_call():
    def call(name, values, **kwargs):
        return probe("MultivarColormap.__call__", multivar_call_op, name=name,
                     values=values, **kwargs)

    assert call("2VarAddA", [[0.0, 1.0], [0.0, 1.0]]) == [[0, 0, 0, 1], [1, 1, 1, 1]]
    call("2VarAddA", [[0.0], [0.0]], alpha=0.1)
    call("2VarSubA", [[0.0, 1.0], [0.0, 1.0]])
    call("2VarSubA", [[0., 0., 0., 1.2, NAN], [0., 1.2, NAN, 0., 0.]])
    assert call("2VarSubA", [[0.0], [0.0]], bytes=True) == [[255, 255, 255, 255]]
    assert isinstance(call("2VarSubA", [[0, 5, 9], [0, 0, 0], [0, 0, 0]]), ValueError)
    ints = [[0, 50, 100, 0, 0, 300], [0, 0, 0, 50, 100, 300]]
    call("2VarSubA", ints)
    call("2VarSubA", ints, bytes=True)
    call("2VarSubA", ints, alpha=0.5)
    call("2VarSubA", [[100], [120]], bytes=True, alpha=0.5)
    bad = {"bad": [1, 1, 1, 1]}
    call("2VarSubA", [[0., 1.1, NAN], [0., 1.2, 1.]], extremes=bad)
    call("2VarSubA", [[300], [300]], bytes=True, alpha=0.5, extremes=bad)
    assert isinstance(call("2VarSubA", [[0], [5], [9]], extremes=bad), ValueError)
    for key in ("over", "under"):
        assert isinstance(call("2VarAddA", [[0.0], [0.0]], extremes={key: [[0, 0, 0, 0]]}),
                          ValueError)
    call("2VarAddA", [[-1.], [0.]], extremes={"under": [[0, 0, 0, 0]] * 2})
    call("2VarAddA", [[2.], [0.]], extremes={"under": [[0, 0, 0, 0]] * 2,
                                            "over": [[0, 0, 0, 0]] * 2})


def multivar_resampled_op(name, lutshape, values, component):
    cmap = mpl.multivar_colormaps[name].resampled(
        tuple(lutshape) if isinstance(lutshape, list) else lutshape)
    index, x = component
    return {"call": _colors(cmap(_values(values))), "component": list(cmap[index](x))}


def test_multivar_resample():
    res = probe("MultivarColormap.resampled", multivar_resampled_op, name="3VarAddA",
                lutshape=[None, 10, 3], values=[[0, 1], [0.25, 0.25], [0, 1]],
                component=[1, 0.25])
    assert np.allclose(res["component"], (0.093, 0.116, 0.059, 1.0), atol=1e-3)
    assert isinstance(probe("MultivarColormap.resampled", multivar_resampled_op,
                            name="3VarAddA", lutshape=4, values=[[0], [0], [0]],
                            component=[0, 0.0]), ValueError)


# --- bivariate colormaps ------------------------------------------------------


def _bivar(cmap):
    if cmap in ("ramp", "rgb", "rgb_u8"):
        return BivarColormapFromImage(_image(cmap))
    return mpl.bivar_colormaps[cmap]


def _with_extremes(cmap, extremes):
    if not extremes:
        return cmap
    return cmap.with_extremes(**{key: (tuple(value) if isinstance(value, list) else value)
                                 for key, value in extremes.items()})


def bivar_call_op(cmap, values, alpha=None, bytes=False, extremes=None):
    cmap = _with_extremes(_bivar(cmap), extremes)
    return _colors(cmap(_values(values), alpha=alpha, bytes=bytes))


def test_bivar_cmap_call_tuple():
    for values, kwargs in (([[1.0, 0.0, 0.2], [1.0, 0.0, 0.8]], {}),
                           ([[0.0], [0.0]], {"alpha": 0.1})):
        probe("BivarColormap.__call__", bivar_call_op, cmap="BiOrangeBlue",
              values=values, **kwargs)


def test_bivar_cmap_call():
    def call(values, **kwargs):
        return probe("BivarColormap.__call__", bivar_call_op, cmap="ramp", values=values,
                     **kwargs)

    ints = [[0, 5, 9, 0, 0, 10], [0, 0, 0, 5, 11, 12]]
    call(ints)
    outside = {"outside": [1, 0, 0, 0]}
    call([[0.5, 0], [0, 3]], extremes=outside)
    call(ints, bytes=True, extremes=outside)
    call(ints, alpha=0.5, extremes=outside)
    call([[10], [12]], bytes=True, alpha=0.5, extremes=outside)
    ignore = {"outside": [1, 0, 0, 0], "bad": [1, 1, 1, 0], "shape": "ignore"}
    call([[0., 1.1, NAN], [0., 1.2, 1.]], extremes=ignore)
    call([[10], [12]], bytes=True, alpha=0.5, extremes=ignore)
    call([[0, 10], [0, 12]], extremes=ignore)
    assert isinstance(call([[0], [5], [9]], extremes=ignore), ValueError)
    assert isinstance(call([[0, 5, 9, 0, 0, 9], [0, 0, 0, 5, 11, 11]],
                           extremes={**ignore, "shape": "circle"}), NotImplementedError)


def test_bivar_cmap_from_image():
    data_0 = (np.arange(6).reshape((2, 3)) / 5).ravel().tolist()
    data_1 = (np.arange(6).reshape((3, 2)).T / 5).ravel().tolist()
    for cmap in ("rgb", "rgb_u8"):
        res = probe("BivarColormap.__call__", bivar_call_op, cmap=cmap,
                    values=[data_0, data_1], tol=1e-6)
        assert np.allclose(res[4], [0.8, 0.58333333, 1, 1], atol=0.01)


def lut_op(name):
    lut = mpl.bivar_colormaps[name].lut
    return [lut[:, 0, 0].tolist(), lut[:, 255, 0].tolist(), lut[:, 0, 1].tolist(),
            lut[:, 153, 1].tolist(), lut[:, 255, 1].tolist(), lut[0, :, 1].tolist(),
            lut[102, :, 1].tolist(), lut[255, :, 1].tolist(), lut[0, :, 2].tolist(),
            lut[255, :, 2].tolist()]


def test_bivar_cmap_lut_smooth():
    res = probe("BivarColormap.lut", lut_op, name="BiOrangeBlue")
    assert np.allclose(res[3], np.linspace(0.3, 0.8, 256))


def getitem_op(name, axis, origin=None, shape=None, values=None):
    extremes = {key: value for key, value in (("origin", origin), ("shape", shape))
                if value is not None}
    cmap = _with_extremes(mpl.bivar_colormaps[name], extremes)
    if values is None:
        return cmap[axis].colors.tolist()
    return [_colors(cmap(_values(values))), _colors(cmap[axis](np.asarray(values[axis])))]


def test_bivar_cmap_1d_origin():
    for origin in (None, [0, 1], [0.2, 0.4]):
        for axis in (0, 1):
            probe("BivarColormap.__getitem__", getitem_op, name="BiOrangeBlue", axis=axis,
                  origin=origin)


def test_bivar_getitem():
    sweep = [.0, .25, .5, .75, 1., -1, 2]
    for name, fixed in (("BiPeak", .5), ("BiOrangeBlue", .0)):
        for shape in (None, "ignore"):
            for axis in (0, 1):
                values = [sweep, [fixed] * 7] if axis == 0 else [[fixed] * 7, sweep]
                whole, part = probe("BivarColormap.__getitem__", getitem_op, name=name,
                                    axis=axis, shape=shape, values=values)
                assert whole == part


def resampled_op(name, method, values, lutshape=None):
    cmap = mpl.bivar_colormaps[name]
    if method == "resampled":
        cmap = cmap.resampled(tuple(lutshape))
    else:
        cmap = getattr(cmap, method)()
    return _colors(cmap(_values(values)))


def test_bivar_resample():
    for lutshape in ([2, 2], [-2, 2], [2, -2], [-2, -2]):
        probe("BivarColormap.resampled", resampled_op, name="BiOrangeBlue",
              method="resampled", lutshape=lutshape, values=[[0.25], [0.25]])
    probe("BivarColormap.resampled", resampled_op, name="BiOrangeBlue",
          method="reversed", values=[[1 - 0.8], [1 - 0.4]])
    probe("BivarColormap.resampled", resampled_op, name="BiOrangeBlue",
          method="transposed", values=[[0.2], [0.6]])


def bad_shape_op(shape, image):
    if image:
        BivarColormapFromImage(np.ones((3, 3, 4)), shape=shape)
    else:
        mpl.bivar_colormaps["BiCone"].with_extremes(shape=shape)


def test_bivar_cmap_bad_shape():
    for image in (False, True):
        assert isinstance(probe("BivarColormap.with_extremes", bad_shape_op,
                                shape="bad_shape", image=image), ValueError)


# --- equality -----------------------------------------------------------------


def _described(spec):
    if "colormaps" in spec:
        components = [mpl.multivar_colormaps[c[0]][c[1]] if isinstance(c, list)
                      else mpl.colormaps[c] for c in spec["colormaps"]]
        return MultivarColormap(components, spec["mode"])
    name = spec["name"]
    if name in mpl.multivar_colormaps:
        cmap = mpl.multivar_colormaps[name]
        return cmap.with_extremes(bad=spec["bad"]) if "bad" in spec else cmap
    cmap = mpl.bivar_colormaps[name]
    for key in ("bad", "outside", "shape"):
        if key in spec:
            cmap = cmap.with_extremes(**{key: spec[key]})
    if "lut_scale" in spec:
        cmap._init()
        cmap._lut *= spec["lut_scale"]
    return cmap


def eq_op(a, b):
    return _described(a) == _described(b)


def test_bivar_eq():
    peak = {"name": "BiPeak"}
    assert probe("Colormap.__eq__", eq_op, a=peak, b={"name": "BiPeak"})
    for other in ({"name": "2VarAddA"}, {"name": "BiCone"}, {"name": "BiPeak", "bad": "k"},
                  {"name": "BiPeak", "outside": "k"}, {"name": "BiPeak", "lut_scale": 0.5},
                  {"name": "BiPeak", "shape": "ignore"}):
        assert not probe("Colormap.__eq__", eq_op, a=peak, b=other)


def test_multivar_eq():
    add = {"name": "2VarAddA"}
    assert probe("Colormap.__eq__", eq_op, a=add, b={"name": "2VarAddA"})
    for other in ({"name": "BiPeak"},
                  {"colormaps": [["2VarAddA", 0], ["2VarAddA", 0]], "mode": "sRGB_add"},
                  {"name": "3VarAddA"}, {"name": "2VarAddA", "bad": "k"},
                  {"colormaps": [["2VarAddA", 0], ["2VarAddA", 1]], "mode": "sRGB_sub"}):
        assert not probe("Colormap.__eq__", eq_op, a=add, b=other)
    assert probe("Colormap.__eq__", eq_op, a=add,
                 b={"colormaps": [["2VarAddA", 0], ["2VarAddA", 1]], "mode": "sRGB_add"})
