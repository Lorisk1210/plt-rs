"""``bar3d`` of 100 shaded columns on a 10 × 10 floor."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

cells = np.arange(10, dtype=float)
x, y = (grid.ravel() for grid in np.meshgrid(cells, cells))
heights = FastRng(52).uniform(100, 1.0, 10.0)


def draw():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.bar3d(x, y, 0.0, 0.8, 0.8, heights)
    return fig


if __name__ == "__main__":
    save(draw())
