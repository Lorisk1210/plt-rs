"""Grid levels, labels and extremes of curvilinear grids
(``mpl_toolkits.axisartist.angle_helper`` and ``grid_finder``).

Area ``axisartist``. Grid locators return ``[levels, n, factor]``.

Transforms are given by name (``transform``):

==================  =============================================================
name                aux transform (curvilinear -> rectilinear)
==================  =============================================================
``polar_degrees``   ``Affine2D().scale(pi / 180, 1) + PolarAxes.PolarTransform()``
``polar``           ``PolarAxes.PolarTransform()``
``shear``           ``(x, y) -> (x, y - x)``, inverse ``(x, y) -> (x, y + x)``
``sqrt``            ``(x, y) -> (sign(x) sqrt|x|, y)``, inverse ``(sign(x) x**2, y)``
``rotated``         ``Affine2D().scale(2, 1).rotate_deg(30)``
==================  =============================================================

Ops
---
``angle_helper.select_step``         {v1, v2, nv, hour, include_last, threshold_factor} -> levels
``angle_helper.select_step24``       {v1, v2, nv, include_last, threshold_factor} -> levels
``angle_helper.select_step360``      {v1, v2, nv, include_last, threshold_factor} -> levels
``angle_helper.select_step_degree``  {dv} -> [step, factor]
``angle_helper.select_step_hour``    {dv} -> [step, factor]
``angle_helper.select_step_sub``     {dv} -> [step, factor]
``angle_helper.Locator``             {kind, nbins, v1, v2, include_last?} -> levels of
                                     ``Locator<kind>(nbins, include_last)(v1, v2)``,
                                     kind one of DMS, DM, D, HMS, HM, H
``angle_helper.Formatter``           {kind, direction, factor, values} -> labels of
                                     ``Formatter<kind>()(direction, factor, values)``,
                                     kind DMS or HMS
``angle_helper.ExtremeFinderCycle``  {nx, ny, lon_cycle, lat_cycle, lon_minmax, lat_minmax,
                                     transform, bbox} -> extents of
                                     ``_find_transformed_bbox(transform.inverted(), bbox)``
``grid_finder.ExtremeFinderSimple``  {nx, ny, transform, bbox} -> extents (likewise)
``grid_finder.MaxNLocator``          {nbins, v1, v2, steps?, integer?, symmetric?, prune?} -> levels
``grid_finder.FixedLocator``         {locs, v1, v2} -> levels
``grid_finder.FormatterPrettyPrint`` {values, useMathText?} -> labels
``grid_finder.DictFormatter``        {format_dict: [[value, label], ...], values,
                                     fallback: bool} -> labels (the fallback is a
                                     ``FormatterPrettyPrint()``)
``grid_finder.get_grid_info``        {grid, bbox} -> {extremes, lon, lat} of
                                     ``GridFinder(...).get_grid_info(Bbox.from_extents(*bbox))``
                                     where each coordinate is ``{lines: [[first, last], ...],
                                     ticks: {side: [[x, y, angle, label], ...]}}``; ``grid`` names
                                     the grid of a figure case (see ``GRIDS``)
"""

import numpy as np
from matplotlib.projections import PolarAxes
from matplotlib.transforms import Affine2D, Bbox
from mpl_toolkits.axisartist import angle_helper, grid_finder
from mpl_toolkits.axisartist.grid_finder import GridFinder, _User2DTransform

from spec_support import ProbeSet

probes = ProbeSet("axisartist")

INF = float("inf")

TRANSFORMS = {
    "polar_degrees": lambda: Affine2D().scale(np.pi / 180, 1) + PolarAxes.PolarTransform(),
    "polar": lambda: PolarAxes.PolarTransform(),
    "shear": lambda: _User2DTransform(lambda x, y: (x, y - x), lambda x, y: (x, y + x)),
    "sqrt": lambda: _User2DTransform(lambda x, y: (np.sign(x) * abs(x) ** 0.5, y),
                                     lambda x, y: (np.sign(x) * x ** 2, y)),
    "rotated": lambda: Affine2D().scale(2, 1).rotate_deg(30),
}

LOCATORS = {
    "DMS": angle_helper.LocatorDMS, "DM": angle_helper.LocatorDM, "D": angle_helper.LocatorD,
    "HMS": angle_helper.LocatorHMS, "HM": angle_helper.LocatorHM, "H": angle_helper.LocatorH,
}

FORMATTERS = {"DMS": angle_helper.FormatterDMS, "HMS": angle_helper.FormatterHMS}

#: The grids of the figure cases: ``curvelinear_grid`` (shear and polar),
#: ``curvelinear_grid2`` (sqrt) and ``axis_direction`` (polar, 5 radii).
GRIDS = {
    "shear": lambda: GridFinder(TRANSFORMS["shear"]()),
    "polar": lambda: GridFinder(
        TRANSFORMS["polar_degrees"](),
        extreme_finder=angle_helper.ExtremeFinderCycle(
            20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None, lat_minmax=(0, INF)),
        grid_locator1=angle_helper.LocatorDMS(12),
        tick_formatter1=angle_helper.FormatterDMS()),
    "polar5": lambda: GridFinder(
        TRANSFORMS["polar_degrees"](),
        extreme_finder=angle_helper.ExtremeFinderCycle(
            20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None, lat_minmax=(0, INF)),
        grid_locator1=angle_helper.LocatorDMS(12),
        grid_locator2=grid_finder.MaxNLocator(5),
        tick_formatter1=angle_helper.FormatterDMS()),
    "sqrt": lambda: GridFinder(
        TRANSFORMS["sqrt"](), extreme_finder=grid_finder.ExtremeFinderSimple(20, 20),
        grid_locator1=grid_finder.MaxNLocator(nbins=6),
        grid_locator2=grid_finder.MaxNLocator(nbins=6)),
}


def _levels(result):
    levs, n, factor = result
    return [np.asarray(levs).tolist(), n, float(factor)]


def _select_step(v1, v2, nv, hour, include_last, threshold_factor):
    return _levels(angle_helper.select_step(v1, v2, nv, hour=hour, include_last=include_last,
                                            threshold_factor=threshold_factor))


def _select_step24(v1, v2, nv, include_last, threshold_factor):
    return _levels(angle_helper.select_step24(v1, v2, nv, include_last, threshold_factor))


def _select_step360(v1, v2, nv, include_last, threshold_factor):
    return _levels(angle_helper.select_step360(v1, v2, nv, include_last, threshold_factor))


def _locator(kind, nbins, v1, v2, include_last=True):
    return _levels(LOCATORS[kind](nbins, include_last=include_last)(v1, v2))


def _formatter(kind, direction, factor, values):
    return FORMATTERS[kind]()(direction, factor, np.asarray(values, dtype=float))


def _extents(finder, transform, bbox):
    found = finder._find_transformed_bbox(TRANSFORMS[transform]().inverted(),
                                          Bbox.from_extents(*bbox))
    return list(found.extents)


def _cycle(nx, ny, lon_cycle, lat_cycle, lon_minmax, lat_minmax, transform, bbox):
    finder = angle_helper.ExtremeFinderCycle(nx, ny, lon_cycle=lon_cycle, lat_cycle=lat_cycle,
                                             lon_minmax=lon_minmax, lat_minmax=lat_minmax)
    return _extents(finder, transform, bbox)


def _simple(nx, ny, transform, bbox):
    return _extents(grid_finder.ExtremeFinderSimple(nx, ny), transform, bbox)


def _maxn(nbins, v1, v2, steps=None, integer=False, symmetric=False, prune=None):
    locator = grid_finder.MaxNLocator(nbins, steps=steps, integer=integer, symmetric=symmetric,
                                      prune=prune)
    return _levels(locator(v1, v2))


def _fixed(locs, v1, v2):
    return _levels(grid_finder.FixedLocator(locs)(v1, v2))


def _pretty(values, useMathText=True):  # noqa: N803 - matplotlib's keyword
    return grid_finder.FormatterPrettyPrint(useMathText=useMathText)(
        "bottom", 1, np.asarray(values, dtype=float))


def _dict(format_dict, values, fallback):
    formatter = grid_finder.DictFormatter(
        {value: label for value, label in format_dict},
        grid_finder.FormatterPrettyPrint() if fallback else None)
    return formatter("bottom", 1, np.asarray(values, dtype=float))


def _grid_info(grid, bbox):
    info = GRIDS[grid]().get_grid_info(Bbox.from_extents(*bbox))

    def coordinate(gi):
        return {
            "lines": [[list(line[0]), list(line[-1])] for line in gi["lines"]],
            "ticks": {side: [[*tick["loc"][0], tick["loc"][1], tick["label"]]
                             for tick in ticks]
                      for side, ticks in gi["ticks"].items()},
        }

    return {"extremes": list(info["extremes"].extents),
            "lon": coordinate(info["lon"]), "lat": coordinate(info["lat"])}


def test_select_step():
    for v1, v2, nv in [(0, 360, 12), (-10, 80, 12), (359, 1, 4), (10.5, 10.6, 5),
                       (0.001, 0.0021, 4), (0, 1e-5, 3), (-90, 90, 6), (0, 3000, 12)]:
        for hour in [False, True]:
            for include_last in [True, False]:
                probes.probe("angle_helper.select_step", _select_step, v1=v1, v2=v2, nv=nv,
                             hour=hour, include_last=include_last, threshold_factor=3600.0)
        for threshold in [1, 60, 3600]:
            probes.probe("angle_helper.select_step24", _select_step24, v1=v1, v2=v2, nv=nv,
                         include_last=True, threshold_factor=threshold)
            probes.probe("angle_helper.select_step360", _select_step360, v1=v1, v2=v2, nv=nv,
                         include_last=False, threshold_factor=threshold)
    for dv in [1e-4, 3e-4, 0.01, 0.03, 0.2, 1.0, 1.5, 2.9, 7.0, 25.0, 100.0, 400.0, 600.0]:
        probes.probe("angle_helper.select_step_degree", angle_helper.select_step_degree, dv=dv)
        probes.probe("angle_helper.select_step_hour", angle_helper.select_step_hour, dv=dv)
    for dv in [0.012, 0.03, 0.07, 0.09, 1.2, 2.5, 6.9, 8.0, 150.0]:
        probes.probe("angle_helper.select_step_sub", angle_helper.select_step_sub, dv=dv)


def test_locators():
    ranges = [(0.0, 360.0), (-13.7, 62.4), (120.0, 210.0), (10.0, 10.5), (44.99, 45.01),
              (-0.001, 0.0005), (210.0, 120.0)]
    for kind in LOCATORS:
        for v1, v2 in ranges:
            for nbins in [3, 4, 12]:
                probes.probe("angle_helper.Locator", _locator, kind=kind, nbins=nbins, v1=v1, v2=v2)
        probes.probe("angle_helper.Locator", _locator, kind=kind, nbins=12, v1=-180.0, v2=180.0,
                     include_last=False)


def test_formatters():
    cases = [
        (1, [0, 30, 60, 90, -30, -150]),
        (10, [0, 5, 15, -25, 120]),
        (100, [0, 5, 15, -25, 1234]),
        (60, [0, 30, 90, 150, -45, 600]),
        (600, [0, 5, 15, -25, 1234]),
        (3600, [0, 30, 1800, 3600, 3630, 7230]),
        (3600, [-7230, -3630, -3600, 0]),
        (36000, [0, 5, 15, -25, 36015]),
        (360000, [0, 5, 15, 125]),
        (7200, [0, 1, 2, 3]),
        (1, []),
    ]
    for kind in FORMATTERS:
        for factor, values in cases:
            for direction in ["bottom", "left"]:
                probes.probe("angle_helper.Formatter", _formatter, kind=kind, direction=direction,
                             factor=factor, values=values)
    for values in [[-15.0, 0.0, 15.0, 30.0], [120.0, 135.0, 150.0, 165.0, 180.0, 195.0]]:
        probes.probe("angle_helper.Formatter", _formatter, kind="HMS", direction="bottom",
                     factor=1, values=values)


def test_extreme_finders():
    boxes = [[-5, -5, 12, 10], [1, 1, 3, 2], [-3, -4, -1, 4], [0, 0, 10, 10], [12, -5, -5, 10]]
    for bbox in boxes:
        probes.probe("angle_helper.ExtremeFinderCycle", _cycle, nx=20, ny=20, lon_cycle=360,
                     lat_cycle=None, lon_minmax=None, lat_minmax=[0, INF],
                     transform="polar_degrees", bbox=bbox)
        probes.probe("angle_helper.ExtremeFinderCycle", _cycle, nx=7, ny=9, lon_cycle=360,
                     lat_cycle=360, lon_minmax=[-90, 90], lat_minmax=[-90, 90],
                     transform="polar_degrees", bbox=bbox)
        probes.probe("angle_helper.ExtremeFinderCycle", _cycle, nx=20, ny=20, lon_cycle=None,
                     lat_cycle=None, lon_minmax=None, lat_minmax=None, transform="polar",
                     bbox=bbox)
        for transform in ["shear", "sqrt", "rotated", "polar"]:
            probes.probe("grid_finder.ExtremeFinderSimple", _simple, nx=20, ny=20,
                         transform=transform, bbox=bbox)
        probes.probe("grid_finder.ExtremeFinderSimple", _simple, nx=5, ny=3, transform="shear",
                     bbox=bbox)


def test_grid_locators_and_formatters():
    for v1, v2 in [(0, 10), (-2.3, 7.9), (1e-3, 3e-3), (5, 5), (10, -10), (12345, 12346)]:
        for nbins in [2, 4, 6, 10]:
            probes.probe("grid_finder.MaxNLocator", _maxn, nbins=nbins, v1=v1, v2=v2)
        probes.probe("grid_finder.MaxNLocator", _maxn, nbins=5, v1=v1, v2=v2, steps=[1, 3, 10])
        probes.probe("grid_finder.MaxNLocator", _maxn, nbins=5, v1=v1, v2=v2, integer=True)
        probes.probe("grid_finder.MaxNLocator", _maxn, nbins=5, v1=v1, v2=v2, symmetric=True,
                     prune="both")
        probes.probe("grid_finder.FixedLocator", _fixed, locs=[0, 0.5, 1.5, 3, 7], v1=v1, v2=v2)
    for values in [[0, 2.5, 5, 7.5, 10], [-1.5, 0, 1.5], [1e-5, 2e-5, 3e-5], [1e6, 2e6],
                   [12345.6, 12345.7], [3, 3], [], [0.1, 0.25, 1.0, 4.0]]:
        probes.probe("grid_finder.FormatterPrettyPrint", _pretty, values=values)
        probes.probe("grid_finder.FormatterPrettyPrint", _pretty, values=values, useMathText=False)
    table = [[0.0, "$0$"], [0.25 * np.pi, r"$\frac{1}{4}\pi$"], [0.5 * np.pi, r"$\frac{1}{2}\pi$"]]
    for fallback in [False, True]:
        probes.probe("grid_finder.DictFormatter", _dict, format_dict=table,
                     values=[0.0, 0.25 * np.pi, 1.0, 0.5 * np.pi], fallback=fallback)


def test_grid_info():
    for grid, bbox in [("shear", [0, 0, 10, 10]), ("polar", [-5, -5, 12, 10]),
                       ("polar5", [-5, -5, 12, 10]), ("sqrt", [-0.5, -0.5, 4.5, 4.5]),
                       ("polar", [2, 1, 9, 6]), ("shear", [10, 0, 0, 10])]:
        probes.probe("grid_finder.get_grid_info", _grid_info, grid=grid, bbox=bbox)
