"""``imshow`` of a smooth 2000 × 2000 field, bicubic, with a colorbar."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(2000)


def draw():
    fig, ax = plt.subplots()
    fig.colorbar(ax.imshow(z, interpolation="bicubic"))
    return fig


if __name__ == "__main__":
    save(draw())
