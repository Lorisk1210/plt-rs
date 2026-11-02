"""``imshow`` of a smooth 60 × 60 field enlarged to the axes,
lanczos (an 8 × 8 windowed sinc)."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(60)


def draw():
    fig, ax = plt.subplots()
    ax.imshow(z, interpolation="lanczos")
    return fig


if __name__ == "__main__":
    save(draw())
