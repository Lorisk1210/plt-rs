"""``contourf`` of a smooth 1000 × 1000 field at 20 levels, black contour
lines at the same levels labelled inline, and a colorbar."""

import matplotlib.pyplot as plt

from _common import save, smooth_field

z = smooth_field(1000)


def draw():
    fig, ax = plt.subplots()
    filled = ax.contourf(z, levels=20)
    lines = ax.contour(z, levels=filled.levels, colors="k", linewidths=0.5)
    ax.clabel(lines, fontsize=8)
    fig.colorbar(filled)
    return fig


if __name__ == "__main__":
    save(draw())
