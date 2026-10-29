"""Three labelled groups of 50 points with a legend."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(22)
groups = [(rng.normal(50, center, 0.6), rng.normal(50, center / 2.0, 0.6))
          for center in (0.0, 2.0, 4.0)]


def draw():
    fig, ax = plt.subplots()
    for k, (x, y) in enumerate(groups):
        ax.scatter(x, y, label=f"group {k}")
    ax.set_xlabel("x")
    ax.set_ylabel("y")
    ax.legend()
    return fig


if __name__ == "__main__":
    save(draw())
