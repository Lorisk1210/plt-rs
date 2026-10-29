"""A 10⁶-point random walk as one line."""

import matplotlib.pyplot as plt

from _common import random_walk, save

x, y = random_walk(1_000_000, 1)


def draw():
    fig, ax = plt.subplots()
    ax.plot(x, y)
    return fig


if __name__ == "__main__":
    save(draw())
