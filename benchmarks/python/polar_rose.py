"""A five-petalled rose of 500 points on polar axes."""

import math

import numpy as np
import matplotlib.pyplot as plt

from _common import save

theta = np.linspace(0.0, 2.0 * math.pi, 500)
r = 1.0 + 0.5 * np.sin(5.0 * theta)


def draw():
    fig, ax = plt.subplots(subplot_kw={"projection": "polar"})
    ax.plot(theta, r)
    ax.set_title("A rose")
    return fig


if __name__ == "__main__":
    save(draw())
