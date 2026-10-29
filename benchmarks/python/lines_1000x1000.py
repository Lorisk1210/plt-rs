"""1000 sine waves of 1000 points, one ``plot`` call each."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

x = np.linspace(0.0, 10.0, 1000)
sines = [np.sin(x * (1.0 + k / 1000.0)) + k / 100.0 for k in range(1000)]


def draw():
    fig, ax = plt.subplots()
    for y in sines:
        ax.plot(x, y)
    return fig


if __name__ == "__main__":
    save(draw())
