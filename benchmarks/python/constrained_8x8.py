"""64 titled and labelled cosine panels under a suptitle, in constrained
layout."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

x = np.linspace(0.0, 10.0, 50)
curves = [np.cos(x + k / 10.0) for k in range(64)]


def draw():
    fig, axs = plt.subplots(8, 8, figsize=(16.0, 12.0), layout="constrained")
    for k, (ax, y) in enumerate(zip(axs.flat, curves)):
        ax.plot(x, y)
        ax.set_title(f"panel {k}")
        ax.set_xlabel("x")
        ax.set_ylabel("y")
    fig.suptitle("64 panels in constrained layout")
    return fig


if __name__ == "__main__":
    save(draw())
