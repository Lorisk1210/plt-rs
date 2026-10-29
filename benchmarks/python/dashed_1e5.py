"""A dashed 10⁵-point random walk."""

import matplotlib.pyplot as plt

from _common import random_walk, save

x, y = random_walk(100_000, 2)


def draw():
    fig, ax = plt.subplots()
    ax.plot(x, y, "--")
    return fig


if __name__ == "__main__":
    save(draw())
