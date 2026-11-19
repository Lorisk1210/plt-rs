"""Ninety-six damped modes

A 12 × 8 grid of small axes, each a damped wave with its area filled,
laid out by the constrained layout engine.
"""

import numpy as np
import matplotlib.pyplot as plt

rows, cols = 12, 8
x = np.linspace(0.0, 4 * np.pi, 140)

fig, axs = plt.subplots(rows, cols, figsize=(7.0, 5.2), dpi=90, layout="constrained", squeeze=False)
for k, ax in enumerate(axs.flat):
    wave = 0.55 + 0.11 * (k % cols)
    decay = 0.05 + 0.018 * (k // cols)
    y = np.sin(wave * x) * np.exp(-decay * x)
    ax.plot(x, y, lw=0.8)
    ax.fill_between(x, y, 0, alpha=0.3)
    ax.set_xticks([])
    ax.set_yticks([])
fig.suptitle("96 damped modes")

plt.show()
