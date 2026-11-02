"""``voxels`` of a ball in a 12 × 12 × 12 block, shaded."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

i, j, k = np.indices((12, 12, 12))
ball = (i - 5.5) ** 2 + (j - 5.5) ** 2 + (k - 5.5) ** 2 < 30.0


def draw():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.voxels(ball, edgecolor="k", linewidth=0.3)
    return fig


if __name__ == "__main__":
    save(draw())
