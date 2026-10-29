"""A 2 × 2 grid: two labelled lines with a legend, a scatter, bars and a
histogram, each panel titled."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(21)
t = np.linspace(0.0, 2.0 * np.pi, 100)
sin_t, cos_t = np.sin(t), np.cos(t)
points_x, points_y = rng.uniform(100, 0.0, 1.0), rng.uniform(100, 0.0, 1.0)
heights = rng.uniform(5, 1.0, 10.0)
sample = rng.normal(1000, 0.0, 1.0)


def draw():
    fig, axs = plt.subplots(2, 2)
    axs[0, 0].plot(t, sin_t, label="sin")
    axs[0, 0].plot(t, cos_t, label="cos")
    axs[0, 0].set_title("lines")
    axs[0, 0].legend()
    axs[0, 1].scatter(points_x, points_y)
    axs[0, 1].set_title("scatter")
    axs[1, 0].bar(range(5), heights)
    axs[1, 0].set_title("bars")
    axs[1, 1].hist(sample, bins=20)
    axs[1, 1].set_title("histogram")
    return fig


if __name__ == "__main__":
    save(draw())
