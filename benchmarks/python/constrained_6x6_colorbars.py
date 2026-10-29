"""A 6 × 6 grid of images, each with a colorbar and a title, in
constrained layout."""

import numpy as np
import matplotlib.pyplot as plt

from _common import save

image = np.fromfunction(lambda i, j: np.sin(i * j / 40.0), (20, 20))


def draw():
    fig, axs = plt.subplots(6, 6, figsize=(14.0, 12.0), layout="constrained")
    for k, ax in enumerate(axs.flat):
        fig.colorbar(ax.imshow(image))
        ax.set_title(f"image {k}")
    return fig


if __name__ == "__main__":
    save(draw())
