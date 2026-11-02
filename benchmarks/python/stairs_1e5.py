"""10⁵ steps of a random walk drawn as stairs, filled."""

import matplotlib.pyplot as plt

from _common import random_walk, save

_, levels = random_walk(100_000, 46)
levels = levels - levels.min()


def draw():
    fig, ax = plt.subplots()
    ax.stairs(levels, fill=True)
    return fig


if __name__ == "__main__":
    save(draw())
