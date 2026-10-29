"""Three series of 12 bars side by side, labelled, with a legend."""

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(25)
months = np.arange(12, dtype=float)
sales = [rng.uniform(12, 5.0, 20.0) for _ in range(3)]


def draw():
    fig, ax = plt.subplots()
    for k, heights in enumerate(sales):
        ax.bar(months + (k - 1) * 0.25, heights, width=0.25, label=str(2023 + k))
    ax.set_title("Monthly sales")
    ax.set_xlabel("month")
    ax.set_ylabel("units")
    ax.legend()
    return fig


if __name__ == "__main__":
    save(draw())
