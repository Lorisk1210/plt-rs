"""``imshow`` of a smooth 60 × 60 field enlarged to the axes,
bicubic (a 4 × 4 filter)."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(60)


def draw():
    fig, ax = plt.subplots()
    ax.imshow(z, interpolation="bicubic")
    return fig


if __name__ == "__main__":
    save(draw())
