"""Wind barbs of a swirling field on a 30 × 30 grid, in knots."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save, swirl

t, u, v = swirl(30)
x, y = np.meshgrid(t, t)
u, v = 5.0 * u, 5.0 * v


def draw():
    fig, ax = plt.subplots()
    ax.barbs(x, y, u, v, length=5.0)
    return fig


if __name__ == "__main__":
    save(draw())
