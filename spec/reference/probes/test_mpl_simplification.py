"""matplotlib's own non-image tests of path simplification and clipping
(``matplotlib/tests/test_simplification.py``).

Area ``mpl_simplification``. Paths use the ``path`` area's encoding
(``{"vertices": [[x, y], ...], "codes": [int, ...] | null}``, the cleaned
paths with matplotlib's trailing ``STOP`` vertex) and its ops' arguments:
``Path.cleaned`` and ``Path.iter_segments`` are evaluated as ``path.Path.
cleaned`` and ``path.Path.iter_segments`` are there, every argument given.
Their ``rc`` is the reference ``path.simplify*`` configuration.

Ops
---
``Path.cleaned``  {path, transform, remove_nans, clip, simplify, curves,
    stroke_width, snap, rc} -> the cleaned path, with ``Path.cleaned``'s
    defaults apart from ``clip`` and ``simplify``:

    * paths without codes, with line codes and with curves clipped to a
      rectangle they lie outside of (``test_clipping_out_of_bounds``);
    * ``CLOSEPOLY`` vertices whatever their value, in single and compound
      paths, and after an invalid ``MOVETO`` (``test_simplify_closepoly``);
    * 15 points along a ray at four angles, stepping back and forth,
      simplified to the extremes (``test_angled_antiparallel``; the random
      offsets come from ``spec_support.rng(19680801)``).
``Path.iter_segments``  {path, transform, remove_nans, clip, snap,
    stroke_width, simplify, curves, rc} -> ``[[flat_vertices, code], ...]``
    with ``iter_segments``' defaults apart from ``clip``: a long path
    entirely outside the clip rectangle, which leaves one ``MOVETO``
    (``test_start_with_moveto``), and points far outside, a line and a lone
    point inside it (``test_clipping_full``).
``Line2D.simplified``  {scene, y?} -> what the tests read from the
    display path of the line of ``ax.plot`` on ``plt.subplots()`` (in the
    test style), cleaned with ``simplify=True`` after a draw:

    * ``antiparallel``  ``ax.plot([0, 0, 0, 0, 0, 1], y)``: the simplified
      vertices but the last two, mapped back to data coordinates
      (``test_antiparallel_simplification``, without a draw, as there);
    * ``noise``  50000 samples of ``spec_support.rng(0).uniform`` times 50
      drawn with round joins, 2 pt wide: the size of the vertex array
      (``test_noise``);
    * ``sine_plus_noise``  a sine over 50000 samples plus a hundredth of
      ``spec_support.rng(0)`` noise, likewise (``test_sine_plus_noise``);
    * ``fft_peaks``  the magnitude of the FFT of a Blackman-windowed sine
      of 65536 samples: the size of the vertex array (the assertion the
      ``mpl_rendering/fft_peaks`` case leaves out).

The image tests are cases of ``mpl_rendering``. Left out:
``test_throw_rendering_complexity_exceeded``, which checks that Agg gives
up on two million unsimplified points with an ``OverflowError``: plt-rs's
rasterizer has no such cell budget.
"""

import base64

import matplotlib
import matplotlib.pyplot as plt
import numpy as np
from matplotlib import transforms
from matplotlib.path import Path

from probes._mpl_tests import prober
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_simplification")
probe = prober(probes)

#: The reference ``path.simplify*`` configuration the path ops run under.
REF_RC = {"path.simplify": True, "path.simplify_threshold": 0.111111111111}

M, L, C3, CL = Path.MOVETO, Path.LINETO, Path.CURVE3, Path.CLOSEPOLY
NAN = float("nan")


def _path(d):
    return Path(np.asarray(d["vertices"], dtype=float).reshape(-1, 2), d["codes"])


def _pd(p):
    return {"vertices": np.asarray(p.vertices).reshape(-1, 2),
            "codes": None if p.codes is None else np.asarray(p.codes)}


def pth(vertices, codes=None):
    return {"vertices": [[float(x), float(y)] for x, y in vertices],
            "codes": None if codes is None else [int(c) for c in codes]}


def _cleaned(path, transform, remove_nans, clip, simplify, curves, stroke_width, snap, rc):
    with matplotlib.rc_context(rc):
        return _pd(_path(path).cleaned(
            transform, remove_nans, None if clip is None else tuple(clip), simplify=simplify,
            curves=curves, stroke_width=stroke_width, snap=snap))


def probe_cleaned(path, clip=None, simplify=False):
    """``path.cleaned(clip=clip, simplify=simplify)``."""
    return probes.probe("Path.cleaned", _cleaned, path=path, transform=None, remove_nans=False,
                        clip=clip, simplify=simplify, curves=False, stroke_width=1.0, snap=False,
                        rc=REF_RC)


def _iter_segments(path, transform, remove_nans, clip, snap, stroke_width, simplify, curves, rc):
    with matplotlib.rc_context(rc):
        segments = _path(path).iter_segments(
            transforms.IdentityTransform() if transform is None else transform,
            remove_nans=remove_nans, clip=None if clip is None else tuple(clip), snap=snap,
            stroke_width=stroke_width, simplify=simplify, curves=curves)
        return [[[float(v) for v in vertices], int(code)] for vertices, code in segments]


def probe_iter_segments(path, clip):
    """``path.iter_segments(clip=clip)``."""
    return probes.probe("Path.iter_segments", _iter_segments, path=path, transform=None,
                        remove_nans=True, clip=clip, snap=False, stroke_width=1.0,
                        simplify=None, curves=True, rc=REF_RC)


def test_clipping_out_of_bounds():
    rect = [10, 10, 20, 20]
    for codes in (None, [M, L, L]):
        res = probe_cleaned(pth([(0, 0), (1, 2), (2, 1)], codes), clip=rect)
        assert res["vertices"].tolist() == [[0, 0]] and res["codes"].tolist() == [Path.STOP]
    curve = pth([(0, 0), (1, 2), (2, 3)], [M, C3, C3])
    plain = probe_cleaned(curve)
    clipped = probe_cleaned(curve, clip=rect)
    assert np.array_equal(plain["vertices"], clipped["vertices"])


def test_simplify_closepoly():
    for last in [(NAN, NAN), (40, 50)]:
        res = probe_cleaned(pth([(1, 1), (2, 1), (2, 2), last], [M, L, L, CL]), simplify=True)
        assert res["vertices"].tolist() == [[1, 1], [2, 1], [2, 2], [1, 1], [1, 1], [0, 0]]
    probe_cleaned(pth([(1, 1), (2, 1), (2, 2), (NAN, NAN), (-1, 0), (-2, 0), (-2, 1), (NAN, NAN)],
                      [M, L, L, CL, M, L, L, CL]), simplify=True)
    res = probe_cleaned(pth([(1, 0), (1, -1), (2, -1), (NAN, NAN), (-1, -1), (-2, 1), (-1, 1),
                             (2, 2), (0, -1)],
                            [M, L, L, M, L, L, L, CL, L]), simplify=True)
    assert res["codes"].tolist() == [M, L, L, M, L, L, L, L, L, Path.STOP]


def _angled_ray(angle, offset):
    """The 15 points of ``test_angled_antiparallel``: random offsets along
    a ray at ``angle``, the first at 0 and the second at 1."""
    offsets = (rng(19680801).uniform(15) - offset) * 5
    offsets[0] = 0
    offsets[1] = 1
    return pth(np.column_stack([np.sin(angle) * offsets, np.cos(angle) * offsets]))


def test_angled_antiparallel():
    for angle in [0, np.pi / 4, np.pi / 3, np.pi / 2]:
        for offset in [0, .5]:
            res = probe_cleaned(_angled_ray(angle, offset), simplify=True)
            assert len(res["codes"]) == (5 if offset else 4)


#: ``test_start_with_moveto``'s path: little-endian int32 vertex pairs.
START_WITH_MOVETO = b"""
ZwAAAAku+v9UAQAA+Tj6/z8CAADpQ/r/KAMAANlO+v8QBAAAyVn6//UEAAC6ZPr/2gUAAKpv+v+8
BgAAm3r6/50HAACLhfr/ewgAAHyQ+v9ZCQAAbZv6/zQKAABepvr/DgsAAE+x+v/lCwAAQLz6/7wM
AAAxx/r/kA0AACPS+v9jDgAAFN36/zQPAAAF6Pr/AxAAAPfy+v/QEAAA6f36/5wRAADbCPv/ZhIA
AMwT+/8uEwAAvh77//UTAACwKfv/uRQAAKM0+/98FQAAlT/7/z0WAACHSvv//RYAAHlV+/+7FwAA
bGD7/3cYAABea/v/MRkAAFF2+//pGQAARIH7/6AaAAA3jPv/VRsAACmX+/8JHAAAHKL7/7ocAAAP
rfv/ah0AAAO4+/8YHgAA9sL7/8QeAADpzfv/bx8AANzY+/8YIAAA0OP7/78gAADD7vv/ZCEAALf5
+/8IIgAAqwT8/6kiAACeD/z/SiMAAJIa/P/oIwAAhiX8/4QkAAB6MPz/HyUAAG47/P+4JQAAYkb8
/1AmAABWUfz/5SYAAEpc/P95JwAAPmf8/wsoAAAzcvz/nCgAACd9/P8qKQAAHIj8/7cpAAAQk/z/
QyoAAAWe/P/MKgAA+aj8/1QrAADus/z/2isAAOO+/P9eLAAA2Mn8/+AsAADM1Pz/YS0AAMHf/P/g
LQAAtur8/10uAACr9fz/2C4AAKEA/f9SLwAAlgv9/8ovAACLFv3/QDAAAIAh/f+1MAAAdSz9/ycx
AABrN/3/mDEAAGBC/f8IMgAAVk39/3UyAABLWP3/4TIAAEFj/f9LMwAANm79/7MzAAAsef3/GjQA
ACKE/f9+NAAAF4/9/+E0AAANmv3/QzUAAAOl/f+iNQAA+a/9/wA2AADvuv3/XDYAAOXF/f+2NgAA
29D9/w83AADR2/3/ZjcAAMfm/f+7NwAAvfH9/w44AACz/P3/XzgAAKkH/v+vOAAAnxL+//04AACW
Hf7/SjkAAIwo/v+UOQAAgjP+/905AAB5Pv7/JDoAAG9J/v9pOgAAZVT+/606AABcX/7/7zoAAFJq
/v8vOwAASXX+/207AAA/gP7/qjsAADaL/v/lOwAALZb+/x48AAAjof7/VTwAABqs/v+LPAAAELf+
/788AAAHwv7/8TwAAP7M/v8hPQAA9df+/1A9AADr4v7/fT0AAOLt/v+oPQAA2fj+/9E9AADQA///
+T0AAMYO//8fPgAAvRn//0M+AAC0JP//ZT4AAKsv//+GPgAAojr//6U+AACZRf//wj4AAJBQ///d
PgAAh1v///c+AAB+Zv//Dz8AAHRx//8lPwAAa3z//zk/AABih///TD8AAFmS//9dPwAAUJ3//2w/
AABHqP//ej8AAD6z//+FPwAANb7//48/AAAsyf//lz8AACPU//+ePwAAGt///6M/AAAR6v//pj8A
AAj1//+nPwAA/////w=="""


def test_start_with_moveto():
    verts = np.frombuffer(base64.decodebytes(START_WITH_MOVETO), dtype="<i4").reshape(-1, 2)
    res = probe_iter_segments(pth(verts), clip=[0.0, 0.0, 100.0, 100.0])
    assert len(res) == 1 and res[0][1] == Path.MOVETO


def test_clipping_full():
    clip = [0, 0, 100, 100]
    assert probe_iter_segments(pth([[1e30, 1e30]] * 5), clip) == []
    assert probe_iter_segments(pth([[50, 40], [75, 65]], [M, L]), clip) == [
        [[50, 40], 1], [[75, 65], 2]]
    assert probe_iter_segments(pth([[50, 40]], [M]), clip) == [[[50, 40], 1]]


def _scene_data(scene, y):
    """The plotted values of a ``Line2D.simplified`` scene and whether the
    figure is drawn first."""
    if scene == "antiparallel":
        return [0, 0, 0, 0, 0, 1], y, False
    if scene == "noise":
        return None, rng(0).uniform(50000) * 50, True
    if scene == "sine_plus_noise":
        return None, np.sin(np.linspace(0, np.pi * 2.0, 50000)) + rng(0).uniform(50000) * 0.01, True
    if scene == "fft_peaks":
        t = np.arange(65536)
        return None, abs(np.fft.fft(np.sin(2 * np.pi * .01 * t) * np.blackman(len(t)))), True
    raise ValueError(scene)


def _line_simplified(scene, y=None):
    x, y, draw = _scene_data(scene, y)
    fig, ax = plt.subplots()
    style = {} if scene in ("antiparallel", "fft_peaks") else {
        "solid_joinstyle": "round", "linewidth": 2.0}
    line, = ax.plot(y, **style) if x is None else ax.plot(x, y, **style)
    if draw:
        fig.canvas.draw()
    transform = line.get_transform()
    simplified = transform.transform_path(line.get_path()).cleaned(simplify=True)
    if scene == "antiparallel":
        return transform.inverted().transform_path(simplified).vertices[:-2, :]
    return simplified.vertices.size


#: The y values of ``test_antiparallel_simplification``: ending on a
#: maximum, on a minimum, in between, and without anti-parallel steps
#: ending at the maximum and in the middle.
ANTIPARALLEL = [[.5, 1, -1, 1, 2, .5], [.5, 1, -1, 1, -2, .5], [.5, 1, -1, 1, 0, .5],
                [.5, 1, 2, 1, 3, .5], [.5, 1, 2, 1, 1, .5]]


def test_antiparallel_simplification():
    for y in ANTIPARALLEL:
        probe("Line2D.simplified", _line_simplified, tol=1e-6, scene="antiparallel", y=y)


def test_noise_and_peaks():
    assert probe("Line2D.simplified", _line_simplified, scene="fft_peaks") == 36
    for scene in ["noise", "sine_plus_noise"]:
        probe("Line2D.simplified", _line_simplified, scene=scene)
