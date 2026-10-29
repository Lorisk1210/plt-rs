"""A thin Lissajous curve of 10⁵ points (frequencies 37 and 41), with path
simplification off so that every segment reaches the document."""

import math

import numpy as np
import matplotlib.pyplot as plt

from _common import save

plt.rcParams["path.simplify"] = False
t = np.linspace(0.0, 2.0 * math.pi, 100_000)
x, y = np.sin(37.0 * t), np.sin(41.0 * t + 0.5)


def draw():
    fig, ax = plt.subplots()
    ax.plot(x, y, linewidth=0.5)
    return fig


if __name__ == "__main__":
    save(draw())
