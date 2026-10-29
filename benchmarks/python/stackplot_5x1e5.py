"""Five stacked sine layers of 10⁵ points each."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

x = np.linspace(0.0, 100.0, 100_000)
layers = [1.5 + np.sin(x * (k + 1) / 7.0 + k) for k in range(5)]


def draw():
    fig, ax = plt.subplots()
    ax.stackplot(x, layers)
    return fig


if __name__ == "__main__":
    save(draw())
