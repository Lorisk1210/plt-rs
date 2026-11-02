"""A 16 × 16 grid of 256 small line plots without labels."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

x = np.linspace(0.0, 2.0 * np.pi, 30)
curves = [np.sin(x * (1.0 + k / 64.0)) for k in range(256)]


def draw():
    fig, axs = plt.subplots(16, 16, figsize=(16.0, 16.0))
    for ax, y in zip(axs.flat, curves):
        ax.plot(x, y)
    return fig


if __name__ == "__main__":
    save(draw())
