"""``plot_wireframe`` of ``sin(r)`` on a 60 × 60 grid."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

t = np.linspace(-5.0, 5.0, 60)
x, y = np.meshgrid(t, t)
z = np.sin(np.hypot(x, y))


def draw():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot_wireframe(x, y, z, rstride=2, cstride=2, linewidth=0.5)
    return fig


if __name__ == "__main__":
    save(draw())
