"""10⁵ uniform points with per-point sizes and colour-mapped values."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(4)
x, y = rng.uniform(100_000, 0.0, 1.0), rng.uniform(100_000, 0.0, 1.0)
sizes, values = rng.uniform(100_000, 5.0, 50.0), rng.uniform(100_000, 0.0, 1.0)


def draw():
    fig, ax = plt.subplots()
    ax.scatter(x, y, s=sizes, c=values)
    return fig


if __name__ == "__main__":
    save(draw())
