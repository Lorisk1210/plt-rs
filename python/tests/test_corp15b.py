"""
What the release's triage of the corpora (task CORP15, item 4) showed
pltrs doing otherwise than matplotlib 3.11, reduced to matplotlib's API.
Every expected value was recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.colors import LinearSegmentedColormap


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _BuildingColormap(LinearSegmentedColormap):
    """A colormap whose `_init` reads its own table while it makes it, as
    ultraplot's do."""

    def _init(self):
        super().__init__(self.name, self._segmentdata, self.N)
        self._isinit = False
        lut = np.ones((self.N + 3, 4))
        lut[:-3, :3] = np.linspace(0, 1, self.N)[:, None]
        self._lut = lut
        self._lut[:, 3] = 1.0  # read while the table is being made
        self._isinit = True
        self._set_extremes()


def test_a_colormap_making_its_table_leaves_a_colorbars_label_as_set():
    # ultraplot's right-hand colorbars: the label set to rotation -90 and
    # va="bottom" after the colorbar is made stays so through the draw.
    cmap = _BuildingColormap("building", {c: [(0, 0, 0), (1, 1, 1)]
                                          for c in ("red", "green", "blue")})
    fig, ax = plt.subplots(figsize=(3, 2.5))
    mesh = ax.pcolormesh(np.arange(12.0).reshape(3, 4), cmap=cmap)
    colorbar = fig.colorbar(mesh, label="Shared scale", ticks=[])
    label = colorbar.ax.yaxis.label
    label.update({"ha": "center", "va": "bottom", "rotation": -90})
    fig.canvas.draw()
    assert label.get_va() == "bottom"
    extent = label.get_window_extent(fig.canvas.get_renderer())
    assert extent.x0 > colorbar.ax.bbox.x1
