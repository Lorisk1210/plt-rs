"""A 3-D surface of a 200 × 200 grid, colour-mapped, default strides."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

t = np.linspace(-5.0, 5.0, 200)
x, y = np.meshgrid(t, t)
z = np.sin(np.hypot(x, y))


def draw():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot_surface(x, y, z, cmap="viridis")
    return fig


if __name__ == "__main__":
    save(draw())
