"""100 subplots, each a titled and labelled sine wave."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

x = np.linspace(0.0, 10.0, 100)
curves = [np.sin(x + k / 10.0) for k in range(100)]


def draw():
    fig, axs = plt.subplots(10, 10, figsize=(12.8, 9.6))
    for k, (ax, y) in enumerate(zip(axs.flat, curves)):
        ax.plot(x, y)
        ax.set_title(f"panel {k}")
        ax.set_xlabel("x")
        ax.set_ylabel("y")
    return fig


if __name__ == "__main__":
    save(draw())
