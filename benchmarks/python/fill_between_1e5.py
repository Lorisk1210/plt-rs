"""A 10⁵-point random walk filled down to zero."""

import matplotlib.pyplot as plt

from _common import random_walk, save

x, y = random_walk(100_000, 14)


def draw():
    fig, ax = plt.subplots()
    ax.fill_between(x, y, 0.0)
    return fig


if __name__ == "__main__":
    save(draw())
