"""16 titled and labelled panels, arranged by ``tight_layout``."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

x = np.linspace(0.0, 10.0, 100)
curves = [np.sin(x + k / 4.0) * np.exp(-x / (5.0 + k)) for k in range(16)]


def draw():
    fig, axs = plt.subplots(4, 4, figsize=(9.6, 7.2))
    for k, (ax, y) in enumerate(zip(axs.flat, curves)):
        ax.plot(x, y)
        ax.set_title(f"damping {k}")
        ax.set_xlabel("time")
        ax.set_ylabel("level")
    fig.tight_layout()
    return fig


if __name__ == "__main__":
    save(draw())
