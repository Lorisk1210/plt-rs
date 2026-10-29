"""10⁵ uniform points as the circle markers of one line."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(5)
x, y = rng.uniform(100_000, 0.0, 1.0), rng.uniform(100_000, 0.0, 1.0)


def draw():
    fig, ax = plt.subplots()
    ax.plot(x, y, "o")
    return fig


if __name__ == "__main__":
    save(draw())
