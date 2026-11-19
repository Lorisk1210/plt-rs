"""Filled contours

Filled and line contours of a 2,200 × 2,200 field with a colour bar
(``Axes.contourf`` and ``Axes.contour``).
"""

import numpy as np
import matplotlib.pyplot as plt

n = 2_200
t = np.linspace(-4.0, 4.0, n)
x, y = np.meshgrid(t, t)
z = np.sin(2.0 * x) * np.cos(1.6 * y) * np.exp(-0.08 * (x ** 2 + y ** 2))

fig, ax = plt.subplots(figsize=(6.6, 4.3), dpi=100)
filled = ax.contourf(x, y, z, levels=22)
ax.contour(x, y, z, levels=8, colors="k", linewidths=0.35)
fig.colorbar(filled, ax=ax)
ax.set_title("2,200 × 2,200 field")
ax.set_xlabel("x")
ax.set_ylabel("y")

plt.show()
