"""Sixty-four small images

An 8 × 8 grid of small images, each with its own colour bar, laid out by
the constrained layout engine.
"""

import numpy as np
import matplotlib.pyplot as plt

image = np.fromfunction(lambda i, j: np.sin(i / 6.0) * np.cos(j / 4.0), (32, 32))

fig, axs = plt.subplots(8, 8, figsize=(7.0, 5.4), dpi=80, layout="constrained", squeeze=False)
for k, ax in enumerate(axs.flat):
    bar = fig.colorbar(ax.imshow(image * (0.25 + k / 80.0), cmap="magma"), ax=ax, fraction=0.046, pad=0.02)
    bar.set_ticks([])
    ax.set_xticks([])
    ax.set_yticks([])
fig.suptitle("64 images")

plt.show()
