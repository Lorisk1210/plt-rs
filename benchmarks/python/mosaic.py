"""A ``subplot_mosaic`` of four panels of different sizes, in
constrained layout: a wide line plot, a tall histogram, bars and a
scatter."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(51)
t = np.linspace(0.0, 10.0, 500)
signal = np.sin(t) + 0.1 * t
sample = rng.normal(5000, 0.0, 1.0)
heights = rng.uniform(8, 1.0, 5.0)
px, py = rng.uniform(300, 0.0, 1.0), rng.uniform(300, 0.0, 1.0)


def draw():
    fig, axd = plt.subplot_mosaic("AAB;CDB", figsize=(8.0, 5.0), layout="constrained")
    axd["A"].plot(t, signal)
    axd["A"].set_title("signal")
    axd["B"].hist(sample, bins=40, orientation="horizontal")
    axd["B"].set_title("distribution")
    axd["C"].bar(np.arange(8, dtype=float), heights)
    axd["C"].set_title("bars")
    axd["D"].scatter(px, py, s=6.0)
    axd["D"].set_title("points")
    fig.suptitle("A mosaic of panels")
    return fig


if __name__ == "__main__":
    save(draw())
