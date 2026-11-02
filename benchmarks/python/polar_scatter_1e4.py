"""10⁴ points scattered on polar axes, coloured by angle."""

import math

import numpy as np
import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(47)
theta = rng.uniform(10_000, 0.0, 2.0 * math.pi)
r = np.sqrt(rng.uniform(10_000, 0.0, 1.0))


def draw():
    fig, ax = plt.subplots(subplot_kw={"projection": "polar"})
    ax.scatter(theta, r, c=theta, s=4.0)
    return fig


if __name__ == "__main__":
    save(draw())
