"""``pcolor`` of a smooth 200 × 200 field: one polygon per cell."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(200)


def draw():
    fig, ax = plt.subplots()
    ax.pcolor(z)
    return fig


if __name__ == "__main__":
    save(draw())
