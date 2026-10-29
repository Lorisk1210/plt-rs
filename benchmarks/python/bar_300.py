"""300 bars, one ``bar`` call each."""

import matplotlib.pyplot as plt

from _common import FastRng, save

heights = FastRng(27).uniform(300, 1.0, 10.0)


def draw():
    fig, ax = plt.subplots()
    for i, height in enumerate(heights):
        ax.bar(i, height)
    return fig


if __name__ == "__main__":
    save(draw())
