"""``contourf`` of a smooth 1000 × 1000 field at 20 levels."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(1000)


def draw():
    fig, ax = plt.subplots()
    ax.contourf(z, levels=20)
    return fig


if __name__ == "__main__":
    save(draw())
