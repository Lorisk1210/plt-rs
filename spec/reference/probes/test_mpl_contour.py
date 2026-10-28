"""matplotlib's own non-image tests of contour sets
(``matplotlib/tests/test_contour.py``) that the ``contour``,
``contour_behaviour`` and ``contour_generator`` areas leave out.

Area ``mpl_contour``. Every op runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``) on ``plt.subplots()``.
``np.random`` draws come from ``spec_support.rng``.

Ops
---
``Axes.contour``  {scene} -> {"levels"} of ``ax.contour(...)``, or its
    error:

    ``x_len``, ``y_len``, ``x2d_shape``, ``y2d_shape``, ``z_1x1``,
        ``xyz_1x1``  the inputs of ``test_contour_shape_error`` that a
        quadrilateral field can hold (zeros in place of ``np.empty``): x or
        y of the wrong length, 2-D x or y of the wrong shape, and a 1 x 1 z
        with and without coordinates.
    ``valid_1d``, ``valid_2d``  random 9 x 10 values with ``arange``
        coordinates, 1-D or as a mesh (``test_contour_shape_*_valid``).
    ``empty_levels``, ``uniform``  random 9 x 9 values at ``levels=[]``,
        and ``ones((9, 9))`` (``test_contour_no_valid_levels``).
    ``lognorm:<n>``  ``ax.contour(x, y, |sin(x) e^y|, norm=LogNorm(),
        levels=n)`` on ``np.mgrid[1:10:0.1, 1:10:0.1]``, also giving
        ``"visible"``, the number of levels within the data
        (``test_lognorm_levels``: at most n + 1).
    ``bool``, ``bool_masked``  the boolean 3 x 3 checkerboard
        ``arange(9) % 2``, with the centre masked or not
        (``test_bool_autolevel``).
``Axes.contourf``  {scene} -> {"levels"}, or the error:

    ``bool``, ``bool_masked``  as for ``Axes.contour``.
    ``algorithm:<name>``  ``[[1, 2], [3, 4]]`` contoured by ``algorithm``
        (``test_algorithm_name``; ``invalid`` is an error).
    ``algorithm:<name>:<corner_mask>``  the same with ``corner_mask``
        (``test_algorithm_supports_corner_mask``: ``mpl2005`` cannot mask
        corners).
``Axes.tricontour``, ``Axes.tricontourf``  {scene: "bool"} -> {"levels"}
    of the checkerboard flattened, at nine random points
    (``test_bool_autolevel``).
``ContourSet.clabel``  {scene} -> [{"level", "text", "display",
    "rotation", "zorder"}, ...]: the labels ``clabel`` makes, their centres
    in display pixels (``labelXYs``):

    ``outside_levels``  ``contour(arange(81).reshape(9, 9), levels=[100])``
        labelled with ``fmt={100: "%1.2f"}``: no labels
        (``test_contour_no_valid_levels``).
    ``circular``  the almost circular ``hypot`` contours of a 4 x 4 grid
        (``test_circular_contour_warning``).
    ``large_spacing``  ``exp(-x² - y²)`` at ``[0.01, 0.2, 0.5, 0.8]``
        labelled with ``inline_spacing=100``, which removes almost all of
        each line (``test_clabel_with_large_spacing``).
    ``contour_start``  the latitude-longitude wave of
        ``test_label_contour_start`` at 100 dpi, labelled at size 9: some
        labels sit at the start of their line.
    ``zorder:<use_clabeltext>:<zorder>``  ``contour(x, y, max(|x|, |y|),
        zorder=123)`` labelled with ``zorder`` (``None``: two above the
        contours) (``test_clabel_zorder``).
``ContourSet.find_nearest_contour``  {x, y, indices, pixel, filled} ->
    ``[level, segment, index, xmin, ymin, d2]`` on ``plt.contour(img, 10)``
    of the Gaussian ``img`` on ``np.indices((15, 15))``, or the error for
    ``contourf`` (``test_find_nearest_contour``,
    ``test_find_nearest_contour_no_filled``).
``ContourSet.remove``  {labels_removed} -> {"changed", "restored",
    "warned"}: whether ``contour`` and ``clabel`` changed the axes'
    children, whether ``cs.remove()`` restored them, and whether it warned
    that labels had been removed by hand (``test_contour_remove``,
    ``test_contour_remove_with_labels``).
``ContourSet.get_clip_path``  {} -> whether ``contour(data,
    clip_path=circle)`` keeps a clip path (``test_contour_clip_path``).
``ContourSet.allsegs``  {} -> {"allsegs", "allkinds"}: the number of pieces
    of each level of ``contour(x, y, sin(x) cos(y), levels=[0, 0.5])``
    (``test_allsegs_allkinds``).
``ContourSet.negative_linestyles``  {scene} -> the style negative levels
    get (``test_negative_linestyles``): ``default``, ``rc:<style>``
    (``contour.negative_linestyle``), ``kwarg:<style>`` and
    ``kwarg_linestyles:<style>`` (with ``linestyles="dashdot"`` too).

Covered elsewhere: ``test_contour_Nlevels``,
``test_contourf_decreasing_levels``, ``test_contourf_symmetric_locator``,
``test_all_nan`` (``contour``); ``test_single_color_and_extend``,
``test_contour_linewidth``, ``test_linestyles``, ``test_contour_manual_moveto``,
``test_contour_autolabel_beyond_powerlimits``, ``test_clabel_manual_subset``
and the labels of ``test_contour*_legend_elements`` (``contour_behaviour``);
the image tests (cases of ``mpl_polar``). ``test_subfigure_clabel`` is a
case of ``mpl_tests_contour``.

Left out: the mixed 1-D/2-D and 3-D inputs of ``test_contour_shape_error``
(a quadrilateral field holds 1-D or 2-D coordinates of one kind and a 2-D
z), the colours and types of the ``legend_elements`` handles (opaque in
plt-rs), ``test_label_nonagg`` (backend switching),
``test_clabel_raises_on_filled_contours`` (a deprecation warning),
``test_quadcontourset_reuse`` (contour generator identity),
``test_contour_no_args`` and ``test_contour_singular_color`` (Python
keyword checking).
"""

import warnings

import matplotlib.patches as mpatches
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import LogNorm

from probes._mpl_tests import prober
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_contour")
probe = prober(probes)

#: The checkerboard of ``test_bool_autolevel`` and its mask.
BOOL_Z = (np.arange(9) % 2).reshape((3, 3)).astype(bool)
BOOL_MASK = [[False, False, False], [False, True, False], [False, False, False]]

#: The shape-error inputs (``np.empty`` replaced by zeros).
SHAPE_ERRORS = {
    "x_len": (np.arange(9), np.arange(9), np.zeros((9, 10))),
    "y_len": (np.arange(10), np.arange(10), np.zeros((9, 10))),
    "x2d_shape": (np.zeros((9, 9)), np.zeros((9, 10)), np.zeros((9, 10))),
    "y2d_shape": (np.zeros((9, 10)), np.zeros((9, 9)), np.zeros((9, 10))),
    "z_1x1": ([[0]],),
    "xyz_1x1": ([0], [0], [[0]]),
}


def _levels(cs):
    return {"levels": cs.levels.tolist()}


def _contour_scene(scene):
    fig, ax = plt.subplots()
    if scene in SHAPE_ERRORS:
        return _levels(ax.contour(*SHAPE_ERRORS[scene]))
    if scene in ("valid_1d", "valid_2d"):
        x, y = np.arange(10), np.arange(9)
        z = rng(19680801).uniform(90).reshape(9, 10)
        if scene == "valid_2d":
            x, y = np.meshgrid(x, y)
        return _levels(ax.contour(x, y, z))
    if scene == "empty_levels":
        return _levels(ax.contour(rng(9).uniform(81).reshape(9, 9), levels=[]))
    if scene == "uniform":
        return _levels(ax.contour(np.ones((9, 9))))
    if scene.startswith("lognorm:"):
        n = int(scene.split(":")[1])
        x, y = np.mgrid[1:10:0.1, 1:10:0.1]
        data = np.abs(np.sin(x) * np.exp(y))
        cs = ax.contour(x, y, data, norm=LogNorm(), levels=n)
        levels = cs.levels
        visible = levels[(levels <= data.max()) & (levels >= data.min())]
        return {"levels": levels.tolist(), "visible": len(visible)}
    raise ValueError(f"unknown scene {scene!r}")


def _bool_z(scene):
    return np.ma.array(BOOL_Z, mask=BOOL_MASK) if scene == "bool_masked" else BOOL_Z


def contour_op(scene):
    if scene in ("bool", "bool_masked"):
        fig, ax = plt.subplots()
        return _levels(ax.contour(_bool_z(scene)))
    return _contour_scene(scene)


def contourf_op(scene):
    fig, ax = plt.subplots()
    if scene in ("bool", "bool_masked"):
        return _levels(ax.contourf(_bool_z(scene)))
    _, algorithm, *corner = scene.split(":")
    kwargs = {"corner_mask": corner[0] == "True"} if corner else {}
    z = np.array([[1.0, 2.0], [3.0, 4.0]])
    return _levels(ax.contourf(z, algorithm=algorithm, **kwargs))


def _tri_bool(filled):
    def run(scene):
        assert scene == "bool"
        stream = rng(5)
        x, y = stream.uniform(9), stream.uniform(9)
        fig, ax = plt.subplots()
        func = ax.tricontourf if filled else ax.tricontour
        return _levels(func(x, y, BOOL_Z.ravel()))

    return run


def test_contour_shape_error():
    for scene in SHAPE_ERRORS:
        assert isinstance(probe("Axes.contour", contour_op, scene=scene), TypeError)


def test_contour_shape_1d_valid():
    probe("Axes.contour", contour_op, scene="valid_1d")


def test_contour_shape_2d_valid():
    probe("Axes.contour", contour_op, scene="valid_2d")


def test_contour_no_valid_levels():
    assert probe("Axes.contour", contour_op, scene="empty_levels") == {"levels": []}
    probe("Axes.contour", contour_op, scene="uniform")
    assert probe("ContourSet.clabel", clabel_op, scene="outside_levels") == []


def test_lognorm_levels():
    for n in (2, 3, 4, 5, 6):
        res = probe("Axes.contour", contour_op, scene=f"lognorm:{n}")
        assert res["visible"] <= n + 1


def test_bool_autolevel():
    for scene in ("bool", "bool_masked"):
        assert probe("Axes.contour", contour_op, scene=scene) == {"levels": [0.5]}
        assert probe("Axes.contourf", contourf_op, scene=scene) == {"levels": [0, 0.5, 1]}
    assert probe("Axes.tricontour", _tri_bool(False), scene="bool") == {"levels": [0.5]}
    assert probe("Axes.tricontourf", _tri_bool(True), scene="bool") == {"levels": [0, 0.5, 1]}


def test_algorithm_name():
    for algorithm in ("mpl2005", "mpl2014", "serial", "threaded"):
        probe("Axes.contourf", contourf_op, scene=f"algorithm:{algorithm}")
    assert isinstance(probe("Axes.contourf", contourf_op, scene="algorithm:invalid"),
                      ValueError)


def test_algorithm_supports_corner_mask():
    for algorithm in ("mpl2005", "mpl2014", "serial", "threaded"):
        probe("Axes.contourf", contourf_op, scene=f"algorithm:{algorithm}:False")
        res = probe("Axes.contourf", contourf_op, scene=f"algorithm:{algorithm}:True")
        assert isinstance(res, ValueError) == (algorithm == "mpl2005")


# --- labels -------------------------------------------------------------------


def _labels(cs, texts):
    return [{"level": float(cv), "text": t.get_text(),
             "display": [float(xy[0]), float(xy[1])],
             "rotation": float(t.get_rotation()), "zorder": t.get_zorder()}
            for t, xy, cv in zip(texts, cs.labelXYs, cs.labelCValues)]


def _wave():
    lats = lons = np.linspace(-np.pi / 2, np.pi / 2, 50)
    lons, lats = np.meshgrid(lons, lats)
    wave = 0.75 * (np.sin(2 * lats) ** 8) * np.cos(4 * lons)
    mean = 0.5 * np.cos(2 * lats) * ((np.sin(2 * lats)) ** 2 + 2)
    return lons, lats, wave + mean


def clabel_op(scene):
    name, _, rest = scene.partition(":")
    if name == "outside_levels":
        fig, ax = plt.subplots()
        cs = ax.contour(np.arange(81).reshape((9, 9)), levels=[100])
        return _labels(cs, ax.clabel(cs, fmt={100: "%1.2f"}))
    if name == "circular":
        fig, ax = plt.subplots()
        x, y = np.meshgrid(np.linspace(-2, 2, 4), np.linspace(-2, 2, 4))
        cs = ax.contour(x, y, np.hypot(x, y))
        return _labels(cs, ax.clabel(cs))
    if name == "large_spacing":
        fig, ax = plt.subplots()
        x = y = np.arange(-3.0, 3.01, 0.05)
        X, Y = np.meshgrid(x, y)
        cs = ax.contour(X, Y, np.exp(-X**2 - Y**2), levels=[0.01, 0.2, .5, .8])
        return _labels(cs, ax.clabel(cs, inline_spacing=100))
    if name == "contour_start":
        fig, ax = plt.subplots(dpi=100)
        cs = ax.contour(*_wave())
        return _labels(cs, cs.clabel(fontsize=9))
    if name == "zorder":
        use_clabeltext, zorder = rest.split(":")
        fig, ax = plt.subplots()
        x, y = np.meshgrid(np.arange(0, 10), np.arange(0, 10))
        z = np.max(np.dstack([abs(x), abs(y)]), 2)
        cs = ax.contour(x, y, z, zorder=123)
        texts = cs.clabel(zorder=None if zorder == "None" else float(zorder),
                          use_clabeltext=use_clabeltext == "True")
        return _labels(cs, texts)
    raise ValueError(f"unknown scene {scene!r}")


def test_clabel():
    probe("ContourSet.clabel", clabel_op, scene="circular")
    probe("ContourSet.clabel", clabel_op, scene="large_spacing")
    assert probe("ContourSet.clabel", clabel_op, scene="contour_start", tol=1e-6)


def test_clabel_zorder():
    for use_clabeltext in (True, False):
        for zorder in (1234, None):
            labels = probe("ContourSet.clabel", clabel_op,
                           scene=f"zorder:{use_clabeltext}:{zorder}")
            assert all(label["zorder"] == (zorder or 125) for label in labels)


# --- nearest contours ---------------------------------------------------------


def _gaussian():
    xy = np.indices((15, 15))
    return np.exp(-np.pi * (np.sum((xy - 5)**2, 0) / 5.**2))


def nearest_op(x, y, indices, pixel, filled):
    cs = (plt.contourf if filled else plt.contour)(_gaussian(), 10)
    level, segment, index, xmin, ymin, d2 = cs.find_nearest_contour(
        x, y, indices=indices, pixel=pixel)
    return [int(level), int(segment), int(index), float(xmin), float(ymin), float(d2)]


def test_find_nearest_contour():
    expected = {(1, 1, None): (1, 0, 33, 1.965966, 1.965966, 1.866183),
                (8, 1, None): (1, 0, 5, 7.550173, 1.587542, 0.547550),
                (2, 5, None): (3, 0, 21, 1.884384, 5.023335, 0.013911),
                (2, 5, (5, 7)): (5, 0, 16, 2.628202, 5.0, 0.394638)}
    for (x, y, indices), want in expected.items():
        res = probe("ContourSet.find_nearest_contour", nearest_op, x=x, y=y,
                    indices=indices, pixel=False, filled=False)
        np.testing.assert_array_almost_equal(res, want)


def test_find_nearest_contour_no_filled():
    for x, y, indices, pixel in ((1, 1, None, False), (1, 10, (5, 7), False),
                                 (2, 5, (2, 7), True)):
        res = probe("ContourSet.find_nearest_contour", nearest_op, x=x, y=y,
                    indices=indices, pixel=pixel, filled=True)
        assert isinstance(res, ValueError)


# --- the set as a whole -------------------------------------------------------


def remove_op(labels_removed):
    ax = plt.figure().add_subplot()
    orig_children = ax.get_children()
    cs = ax.contour(np.arange(16).reshape((4, 4)))
    labels = cs.clabel()
    changed = ax.get_children() != orig_children
    if labels_removed:
        labels[0].remove()
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        cs.remove()
    warned = any("Some labels were manually removed" in str(w.message) for w in caught)
    return {"changed": changed, "restored": ax.get_children() == orig_children,
            "warned": warned}


def test_contour_remove():
    assert probe("ContourSet.remove", remove_op, labels_removed=False) == {
        "changed": True, "restored": True, "warned": False}
    assert probe("ContourSet.remove", remove_op, labels_removed=True)["warned"]


def clip_path_op():
    fig, ax = plt.subplots()
    circle = mpatches.Circle([0.5, 0.5], 0.5, transform=ax.transAxes)
    cs = ax.contour([[0, 1], [1, 0]], clip_path=circle)
    return cs.get_clip_path() is not None


def test_contour_clip_path():
    assert probe("ContourSet.get_clip_path", clip_path_op)


def allsegs_op():
    x, y = np.meshgrid(np.arange(0, 10, 2), np.arange(0, 10, 2))
    cs = plt.contour(x, y, np.sin(x) * np.cos(y), levels=[0, 0.5])
    return {"allsegs": [len(level) for level in cs.allsegs],
            "allkinds": [len(level) for level in cs.allkinds]}


def test_allsegs_allkinds():
    assert probe("ContourSet.allsegs", allsegs_op) == {"allsegs": [5, 4],
                                                       "allkinds": [5, 4]}


def _two_gaussians():
    x = np.arange(-3.0, 3.0, 0.025)
    y = np.arange(-2.0, 2.0, 0.025)
    X, Y = np.meshgrid(x, y)
    return X, Y, (np.exp(-X**2 - Y**2) - np.exp(-(X - 1)**2 - (Y - 1)**2)) * 2


def negative_linestyles_op(scene):
    kind, _, style = scene.partition(":")
    kwargs = {"colors": "k"}
    if kind == "rc":
        plt.rcParams["contour.negative_linestyle"] = style
    elif kind == "kwarg":
        kwargs["negative_linestyles"] = style
    elif kind == "kwarg_linestyles":
        kwargs.update(linestyles="dashdot", negative_linestyles=style)
    fig, ax = plt.subplots()
    cs = ax.contour(*_two_gaussians(), 6, **kwargs)
    ax.clabel(cs, fontsize=9, inline=True)
    return cs.negative_linestyles


def test_negative_linestyles():
    assert probe("ContourSet.negative_linestyles", negative_linestyles_op,
                 scene="default") == "dashed"
    for style in ("solid", "dashed", "dashdot", "dotted"):
        for kind in ("rc", "kwarg", "kwarg_linestyles"):
            assert probe("ContourSet.negative_linestyles", negative_linestyles_op,
                         scene=f"{kind}:{style}") == style
