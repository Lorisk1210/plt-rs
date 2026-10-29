"""A 3-D surface of a 200 × 200 grid at full resolution (every one of its
39 601 quads), colour-mapped, without edges or antialiasing."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

t = np.linspace(-5.0, 5.0, 200)
x, y = np.meshgrid(t, t)
z = np.sin(np.hypot(x, y))


def draw():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot_surface(x, y, z, rstride=1, cstride=1, cmap="coolwarm", linewidth=0,
                    antialiased=False)
    return fig


if __name__ == "__main__":
    save(draw())
