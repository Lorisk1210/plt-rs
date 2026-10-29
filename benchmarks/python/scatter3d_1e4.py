"""A 3-D scatter of 10⁴ uniform points."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(12)
x, y, z = [rng.uniform(10_000, 0.0, 1.0) for _ in range(3)]


def draw():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.scatter(x, y, z)
    return fig


if __name__ == "__main__":
    save(draw())
