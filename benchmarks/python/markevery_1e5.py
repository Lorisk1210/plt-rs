"""A 10⁵-point random walk with a circle on every 100th point."""

import matplotlib.pyplot as plt

from _common import random_walk, save

x, y = random_walk(100_000, 3)


def draw():
    fig, ax = plt.subplots()
    ax.plot(x, y, "-o", markevery=100)
    return fig


if __name__ == "__main__":
    save(draw())
