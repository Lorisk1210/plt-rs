"""Four panels: labelled lines with a legend, a colour-mapped scatter,
bars, and a histogram under a mathtext title."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(13)
x = np.linspace(0.0, 10.0, 2000)
scatter_x, scatter_y = rng.uniform(2000, 0.0, 1.0), rng.uniform(2000, 0.0, 1.0)
heights = rng.uniform(30, 1.0, 10.0)
sample = rng.normal(10_000, 0.0, 1.0)
waves = [np.sin(x + k) * np.exp(-x / 10.0) for k in range(3)]


def draw():
    fig, axs = plt.subplots(2, 2)
    lines = axs[0, 0]
    for k, y in enumerate(waves):
        lines.plot(x, y, label=f"phase {k}")
    lines.set_title("damped waves")
    lines.set_xlabel("time")
    lines.set_ylabel("amplitude")
    lines.legend()
    axs[0, 1].scatter(scatter_x, scatter_y, c=scatter_y)
    axs[1, 0].bar(np.arange(len(heights), dtype=float), heights)
    axs[1, 1].hist(sample, bins=50)
    axs[1, 1].set_title(r"$\mathcal{N}(0, 1)$: $\frac{1}{\sqrt{2\pi}} e^{-x^2/2}$")
    return fig


if __name__ == "__main__":
    save(draw())
