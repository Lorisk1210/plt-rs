"""``hexbin`` of 10⁶ correlated normal points."""

import matplotlib.pyplot as plt

from _common import correlated_normals, save

x, y = correlated_normals(6)


def draw():
    fig, ax = plt.subplots()
    ax.hexbin(x, y)
    return fig


if __name__ == "__main__":
    save(draw())
