"""``hist2d`` of 10⁶ correlated normal points in 100 × 100 bins."""

import matplotlib.pyplot as plt

from _common import correlated_normals, save

x, y = correlated_normals(7)


def draw():
    fig, ax = plt.subplots()
    ax.hist2d(x, y, bins=100)
    return fig


if __name__ == "__main__":
    save(draw())
