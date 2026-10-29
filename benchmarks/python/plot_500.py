"""500 lines of 50 points, one ``plot`` call each."""

import matplotlib.pyplot as plt

from _common import save, waves

x, sines = waves(500, 50)


def draw():
    fig, ax = plt.subplots()
    for y in sines:
        ax.plot(x, y)
    return fig


if __name__ == "__main__":
    save(draw())
