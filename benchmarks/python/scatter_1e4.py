"""10⁴ normal points with per-point sizes, coloured by their radius."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(24)
x, y = rng.normal(10_000, 0.0, 1.0), rng.normal(10_000, 0.0, 1.0)
sizes = rng.uniform(10_000, 4.0, 40.0)
radius = np.hypot(x, y)


def draw():
    fig, ax = plt.subplots()
    ax.scatter(x, y, s=sizes, c=radius)
    ax.set_title("10,000 points coloured by radius")
    ax.set_xlabel("x")
    ax.set_ylabel("y")
    return fig


if __name__ == "__main__":
    save(draw())
