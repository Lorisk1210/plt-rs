"""Twenty measurements with error bars and caps."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(41)
x = np.arange(20, dtype=float)
y = 0.5 * x + rng.normal(20, 0.0, 1.0)
yerr = rng.uniform(20, 0.3, 1.5)


def draw():
    fig, ax = plt.subplots()
    ax.errorbar(x, y, yerr=yerr, fmt="o", capsize=3.0)
    ax.set_xlabel("trial")
    ax.set_ylabel("response")
    return fig


if __name__ == "__main__":
    save(draw())
