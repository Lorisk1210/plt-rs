"""10⁴ points of a random walk with error bars in both directions."""

import matplotlib.pyplot as plt

from _common import FastRng, random_walk, save

x, y = random_walk(10_000, 44)
rng = FastRng(45)
xerr, yerr = rng.uniform(10_000, 0.1, 0.5), rng.uniform(10_000, 0.5, 2.0)


def draw():
    fig, ax = plt.subplots()
    ax.errorbar(x, y, xerr=xerr, yerr=yerr, elinewidth=0.5)
    return fig


if __name__ == "__main__":
    save(draw())
