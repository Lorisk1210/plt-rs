"""``quiver`` of a swirling field: 10⁴ arrows."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save, swirl

t, u, v = swirl(100)
x, y = np.meshgrid(t, t)


def draw():
    fig, ax = plt.subplots()
    ax.quiver(x, y, u, v)
    return fig


if __name__ == "__main__":
    save(draw())
