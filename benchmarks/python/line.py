"""A single line plot of 200 points with a title and axis labels."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

x = np.linspace(0.0, 10.0, 200)
y = np.sin(x)


def draw():
    fig, ax = plt.subplots()
    ax.plot(x, y)
    ax.set_title("A sine wave")
    ax.set_xlabel("time (s)")
    ax.set_ylabel("amplitude")
    return fig


if __name__ == "__main__":
    save(draw())
