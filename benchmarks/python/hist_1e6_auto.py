"""``hist`` of 10⁶ normal values, binned by numpy's ``"auto"`` rule."""

import matplotlib.pyplot as plt

from _common import FastRng, save

values = FastRng(15).normal(1_000_000, 0.0, 1.0)


def draw():
    fig, ax = plt.subplots()
    ax.hist(values, bins="auto")
    return fig


if __name__ == "__main__":
    save(draw())
