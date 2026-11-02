"""``tricontour`` lines of a bump over 10⁴ uniform points at 12
levels, triangulated (Delaunay) as part of the work."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(49)
x, y = rng.uniform(10_000, -3.0, 3.0), rng.uniform(10_000, -3.0, 3.0)
z = np.exp(-(x * x) - y * y) + 0.3 * np.sin(2.0 * x)


def draw():
    fig, ax = plt.subplots()
    ax.tricontour(x, y, z, levels=12)
    return fig


if __name__ == "__main__":
    save(draw())
