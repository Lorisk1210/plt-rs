"""``pcolormesh`` of a smooth 1000 × 1000 field."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(1000)


def draw():
    fig, ax = plt.subplots()
    ax.pcolormesh(z)
    return fig


if __name__ == "__main__":
    save(draw())
