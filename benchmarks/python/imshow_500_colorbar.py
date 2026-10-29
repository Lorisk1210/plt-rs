"""A 500 × 500 image with a colorbar and a title."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(500)


def draw():
    fig, ax = plt.subplots()
    image = ax.imshow(z)
    fig.colorbar(image)
    ax.set_title("A 500 x 500 field")
    return fig


if __name__ == "__main__":
    save(draw())
