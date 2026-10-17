"""Shaded relief (``matplotlib.colors.LightSource``).

Area ``lightsource``. Every op builds ``LightSource(azdeg, altdeg)`` (with
the optional HSV limits) and calls one of its methods.

Argument conventions
--------------------
* ``elevation``, ``data``: 2-D arrays as nested lists of floats (rows).
* ``rgb``: an ``(M, N, 3)`` array as nested lists; ``intensity`` an
  ``(M, N)`` array, given to the blend functions as ``(M, N, 1)``.
* ``normals``: an ``(M, N, 3)`` array of unit normals.
* ``relief``: ``{vert_exag, dx, dy, fraction}`` passed as keywords.
* ``light``: ``{azdeg, altdeg, hsv_min_val?, hsv_max_val?, hsv_min_sat?,
  hsv_max_sat?}``.
* Results are nested lists; errors are ``{"error": "<ExceptionType>"}``.

Ops
---
``lightsource.direction``      {light} -> [x, y, z]
``lightsource.hillshade``      {light, elevation, relief} -> (M, N)
``lightsource.shade_normals``  {light, normals, fraction} -> (M, N)
``lightsource.blend``          {light, mode, rgb, intensity} -> (M, N, 3);
                               mode is "hsv", "soft" or "overlay"
``lightsource.shade_rgb``      {light, rgb, elevation, blend_mode, relief} -> (M, N, 3)
``lightsource.shade``          {light, data, cmap, blend_mode, vmin?, vmax?,
                               relief} -> (M, N, 4)
"""

import numpy as np
from matplotlib import colormaps
from matplotlib.colors import LightSource

from spec_support import ProbeSet

probes = ProbeSet("lightsource")

LIGHTS = [
    {"azdeg": 315, "altdeg": 45},
    {"azdeg": 90, "altdeg": 30},
    {"azdeg": 200, "altdeg": 65, "hsv_min_val": 0.2, "hsv_max_val": 0.9,
     "hsv_min_sat": 0.8, "hsv_max_sat": 0.1},
]

RELIEFS = [
    {"vert_exag": 1, "dx": 1, "dy": 1, "fraction": 1},
    {"vert_exag": 3, "dx": 0.5, "dy": 2, "fraction": 1.5},
    {"vert_exag": 0.2, "dx": 1, "dy": 1, "fraction": 0.5},
]


def _surface(rows, cols, phase):
    return [
        [float(np.sin(0.7 * r + phase) * np.cos(0.5 * c) + 0.1 * r * c) for c in range(cols)]
        for r in range(rows)
    ]


SURFACES = [_surface(5, 6, 0.0), _surface(4, 4, 1.3), [[0.0, 1.0], [2.0, 4.0]]]


def _rgb(rows, cols):
    return [
        [[(r * 0.17 + c * 0.11) % 1, (r * 0.29 + 0.3) % 1, (c * 0.23 + 0.6) % 1] for c in range(cols)]
        for r in range(rows)
    ]


def _light(light):
    return LightSource(**light)


def _direction(light):
    return _light(light).direction


def _hillshade(light, elevation, relief):
    return _light(light).hillshade(np.asarray(elevation, dtype=float), **relief)


def _shade_normals(light, normals, fraction):
    return _light(light).shade_normals(np.asarray(normals, dtype=float), fraction)


def _blend(light, mode, rgb, intensity):
    ls = _light(light)
    function = {"hsv": ls.blend_hsv, "soft": ls.blend_soft_light, "overlay": ls.blend_overlay}[mode]
    return function(np.asarray(rgb, dtype=float), np.asarray(intensity, dtype=float)[..., np.newaxis])


def _shade_rgb(light, rgb, elevation, blend_mode, relief):
    return _light(light).shade_rgb(
        np.asarray(rgb, dtype=float), np.asarray(elevation, dtype=float), blend_mode=blend_mode, **relief)


def _shade(light, data, cmap, blend_mode, relief, vmin=None, vmax=None):
    return _light(light).shade(
        np.asarray(data, dtype=float), colormaps[cmap], blend_mode=blend_mode, vmin=vmin, vmax=vmax, **relief)


def test_direction():
    for light in LIGHTS + [{"azdeg": 0, "altdeg": 90}, {"azdeg": 270, "altdeg": 0}]:
        probes.probe("lightsource.direction", _direction, light=light)


def test_hillshade():
    for light in LIGHTS:
        for surface in SURFACES:
            for relief in RELIEFS:
                probes.probe("lightsource.hillshade", _hillshade, light=light, elevation=surface, relief=relief)
    flat = [[2.0] * 3] * 3
    probes.probe("lightsource.hillshade", _hillshade, light=LIGHTS[0], elevation=flat, relief=RELIEFS[0])
    assert isinstance(
        probes.probe("lightsource.hillshade", _hillshade, light=LIGHTS[0], elevation=[[1.0, 2.0, 3.0]],
                     relief=RELIEFS[0]),
        ValueError)


def test_shade_normals():
    normals = [
        [[0.0, 0.0, 1.0], [0.6, 0.0, 0.8], [0.0, -0.6, 0.8]],
        [[-0.48, 0.6, 0.64], [0.36, 0.48, 0.8], [0.0, 0.0, 1.0]],
    ]
    for light in LIGHTS:
        for fraction in [1, 0.5, 2]:
            probes.probe("lightsource.shade_normals", _shade_normals, light=light, normals=normals,
                         fraction=fraction)


def test_blend():
    rgb = _rgb(3, 4)
    intensity = [[(r * 4 + c) / 11 for c in range(4)] for r in range(3)]
    for light in LIGHTS:
        for mode in ["hsv", "soft", "overlay"]:
            probes.probe("lightsource.blend", _blend, light=light, mode=mode, rgb=rgb, intensity=intensity)


def test_shade_rgb():
    surface = SURFACES[0]
    rgb = _rgb(5, 6)
    for light in LIGHTS[:2]:
        for mode in ["hsv", "soft", "overlay"]:
            for relief in RELIEFS[:2]:
                probes.probe("lightsource.shade_rgb", _shade_rgb, light=light, rgb=rgb, elevation=surface,
                             blend_mode=mode, relief=relief)


def test_shade():
    for cmap in ["gist_earth", "viridis", "terrain"]:
        for mode in ["overlay", "soft", "hsv"]:
            probes.probe("lightsource.shade", _shade, light=LIGHTS[0], data=SURFACES[0], cmap=cmap,
                         blend_mode=mode, relief=RELIEFS[1])
    probes.probe("lightsource.shade", _shade, light=LIGHTS[1], data=SURFACES[1], cmap="copper",
                 blend_mode="overlay", relief=RELIEFS[0], vmin=-0.5, vmax=1.5)
