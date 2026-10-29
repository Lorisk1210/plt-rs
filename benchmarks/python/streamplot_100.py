"""``streamplot`` of a swirling field on a 100 × 100 grid."""

import matplotlib.pyplot as plt

from _common import save, swirl

t, u, v = swirl(100)


def draw():
    fig, ax = plt.subplots()
    ax.streamplot(t, t, u, v)
    return fig


if __name__ == "__main__":
    save(draw())
