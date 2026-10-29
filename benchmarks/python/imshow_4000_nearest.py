"""``imshow`` of a smooth 4000 × 4000 field, nearest neighbour."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(4000)


def draw():
    fig, ax = plt.subplots()
    ax.imshow(z, interpolation="nearest")
    return fig


if __name__ == "__main__":
    save(draw())
