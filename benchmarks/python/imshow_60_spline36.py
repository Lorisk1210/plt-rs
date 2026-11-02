"""``imshow`` of a smooth 60 × 60 field enlarged to the axes,
spline36 (a 6 × 6 filter)."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(60)


def draw():
    fig, ax = plt.subplots()
    ax.imshow(z, interpolation="spline36")
    return fig


if __name__ == "__main__":
    save(draw())
