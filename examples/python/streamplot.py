"""Streamplot: air flowing past a cylinder (ideal flow), the streamlines
coloured by speed."""

import numpy as np

import pltrs.pyplot as plt
from pltrs.patches import Circle

radius = 1.0
x = np.linspace(-4, 4, 160)
y = np.linspace(-2.5, 2.5, 100)
X, Y = np.meshgrid(x, y)
r2 = X**2 + Y**2
u = 1 - radius**2 * (X**2 - Y**2) / r2**2
v = -2 * radius**2 * X * Y / r2**2
inside = r2 < radius**2  # no flow inside the cylinder
u[inside] = 0
v[inside] = 0
speed = np.hypot(u, v)

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
stream = ax.streamplot(x, y, u, v, color=speed, cmap="plasma", density=1.6, linewidth=1, arrowsize=0.8)
ax.add_patch(Circle((0, 0), radius, color="0.85", zorder=3))
fig.colorbar(stream.lines, ax=ax, label="speed (relative to the free stream)", shrink=0.8)
ax.set_aspect("equal")
ax.set_xlim(-4, 4)
ax.set_ylim(-2.5, 2.5)
ax.set_title("Flow past a cylinder")

fig.savefig("streamplot.png")
