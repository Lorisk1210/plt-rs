"""Smooth heatmap

A 3,000 × 3,000 mesh of a smooth field with a colour bar
(``Axes.pcolormesh``).
"""

import numpy as np
import matplotlib.pyplot as plt

n = 3_000
t = np.linspace(-3.0, 3.0, n)
x, y = np.meshgrid(t, t)
z = np.sin(5 * x) * np.cos(4 * y) * np.exp(-0.12 * (x ** 2 + y ** 2))

fig, ax = plt.subplots(figsize=(6.6, 4.3), dpi=100)
mesh = ax.pcolormesh(x, y, z, shading="auto")
fig.colorbar(mesh, ax=ax)
ax.set_title("3,000 × 3,000 cells")
ax.set_xlabel("x")
ax.set_ylabel("y")

plt.show()
