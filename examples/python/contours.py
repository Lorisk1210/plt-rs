"""Contour and filled contour plot: a relief map of two hills and a
valley, with labelled contour lines over coloured height bands."""

import numpy as np

import pltrs.pyplot as plt

x = np.linspace(0, 10, 200)
y = np.linspace(0, 6, 120)
X, Y = np.meshgrid(x, y)
height = (
    900 * np.exp(-((X - 3) ** 2 + (Y - 3.5) ** 2) / 3.0)
    + 650 * np.exp(-((X - 7.2) ** 2 + (Y - 2.2) ** 2) / 2.0)
    - 250 * np.exp(-((X - 5.5) ** 2 + (Y - 5) ** 2) / 1.5)
    + 120 * np.sin(X / 1.7) * np.cos(Y / 1.3)
    + 300
)

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
bands = ax.contourf(X, Y, height, levels=14, cmap="terrain")
lines = ax.contour(X, Y, height, levels=bands.levels[::2], colors="0.2", linewidths=0.7)
ax.clabel(lines, fmt="%d m", fontsize=8)
fig.colorbar(bands, ax=ax, label="height (m)", shrink=0.8)
ax.set_aspect("equal")
ax.set_title("Relief map")
ax.set_xlabel("east (km)")
ax.set_ylabel("north (km)")

fig.savefig("contours.png")
