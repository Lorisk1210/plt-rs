"""1000 points, each annotated with its own label: 1000 ``annotate``
calls."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(28)
x, y = rng.uniform(1000, 0.0, 10.0), rng.uniform(1000, 0.0, 10.0)


def draw():
    fig, ax = plt.subplots()
    ax.plot(x, y, "o", markersize=2)
    for i in range(1000):
        ax.annotate(f"p{i}", (x[i], y[i]), fontsize=6)
    return fig


if __name__ == "__main__":
    save(draw())
