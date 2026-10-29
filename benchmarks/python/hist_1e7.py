"""``hist`` of 10⁷ normal values in 1000 bins."""

import matplotlib.pyplot as plt

from _common import FastRng, save

values = FastRng(8).normal(10_000_000, 0.0, 1.0)


def draw():
    fig, ax = plt.subplots()
    ax.hist(values, bins=1000)
    return fig


if __name__ == "__main__":
    save(draw())
