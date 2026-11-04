"""
Smoke tests of what the differential tester's `api` programs
(`spec/diff/generators/api.py`) found pltrs (Python) answering otherwise
than matplotlib: secondary axes placed by a location string in data
coordinates, the error of an unknown rcParams key, contour sets made by
their constructor, the grids of `streamplot`, the log formatter's error for
NaN, inverted norm limits met when drawing a triangle mesh, and the
colorizer of a `PatchCollection` matching its patches. The expected values
were recorded from matplotlib 3.11.2 running the same code.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, contour, patches, streamplot, ticker

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))


def test_secondary_axis_location_string_in_data_coordinates():
    fig, ax = plt.subplots()
    sec = ax.secondary_yaxis('right', transform=ax.transData)
    assert type(sec).__name__ == "SecondaryAxis"
    fig.canvas.draw()


def test_unknown_rc_key_error_names_the_key():
    with pytest.raises(KeyError) as info:
        pltrs.rcParams['nope']
    assert str(info.value) == "'nope'"
    with pytest.raises(KeyError) as info:
        pltrs.rcParams['a', 2.5]
    assert str(info.value) == "('a', 2.5)"


def test_quad_contour_set_constructor_contours_its_arguments():
    fig, ax = plt.subplots()
    cs = contour.QuadContourSet(ax, Z)
    assert list(cs.levels) == [-2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0]
    assert cs in ax.collections
    assert not cs.filled
    filled = contour.QuadContourSet(ax, Z, levels=3, filled=True)
    assert list(filled.levels) == [-2.0, -1.0, 0.0, 1.0, 2.0]
    assert filled.filled


def test_streamplot_grids():
    grid = streamplot.Grid(np.arange(6.0), np.arange(5.0))
    mask = streamplot.StreamMask(10)
    domain = streamplot.DomainMap(grid, mask)
    assert grid.shape == (5, 6) and grid.nx == 6 and grid.dx == 1.0
    assert grid.within_grid(4.5, 3) and not grid.within_grid(6, 0)
    assert mask.shape == (300, 300) and mask.nx == 300
    assert domain.grid2mask(2.0, 3.0) == (120, 224)
    assert domain.mask2grid(9, 9) == pytest.approx((0.15050167224080266, 0.12040133779264214))
    assert domain.data2grid(1.5, 2.5) == (1.5, 2.5)
    assert domain.grid2data(1, 1) == (1.0, 1.0)
    assert streamplot.interpgrid(np.arange(12.0).reshape(3, 4), 1.5, 0.5) == 3.5
    with pytest.raises(ValueError, match="'x' must be strictly increasing"):
        streamplot.Grid([0, 2, 1], [0, 1])


def test_log_formatter_nan_is_python_rounding_error():
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        ticker.LogFormatter()(float('nan'))


def test_inverted_norm_limits_of_a_triangle_mesh_raise_value_error():
    fig, ax = plt.subplots()
    ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], vmin=5, vmax=2)
    with pytest.raises(ValueError, match="^minvalue must be less than or equal to maxvalue$"):
        fig.canvas.draw()


def test_patch_collection_matching_its_patches_maps_an_array():
    pc = collections.PatchCollection(
        [patches.Circle((0, 0), 1), patches.Rectangle((1, 1), 1, 2)], match_original=True)
    pc.set_array(np.arange(2.0))
    assert list(pc.get_array()) == [0.0, 1.0]
    assert pc.get_cmap().name == "viridis"
