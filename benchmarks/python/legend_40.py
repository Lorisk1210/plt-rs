"""40 labelled lines under a legend of 40 entries in two columns."""

import matplotlib.pyplot as plt

from _common import save, waves

x, lines = waves(40, 100)


def draw():
    fig, ax = plt.subplots(figsize=(9.6, 7.2))
    for k, y in enumerate(lines):
        ax.plot(x, y, label=f"series {k}")
    ax.legend(ncols=2, fontsize=7, loc="upper left")
    return fig


if __name__ == "__main__":
    save(draw())
