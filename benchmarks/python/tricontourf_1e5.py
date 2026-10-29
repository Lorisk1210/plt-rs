"""``tricontourf`` of 10⁵ scattered points (a Delaunay triangulation)."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(11)
x, y = rng.uniform(100_000, -3.0, 3.0), rng.uniform(100_000, -3.0, 3.0)
z = np.exp(-x * x - y * y) + 0.3 * np.sin(2.0 * x)


def draw():
    fig, ax = plt.subplots()
    ax.tricontourf(x, y, z)
    return fig


if __name__ == "__main__":
    save(draw())
