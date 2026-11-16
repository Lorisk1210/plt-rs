# Streamlines of a flow around a cylinder-like field with varying density and line width.
import numpy as np
import matplotlib.pyplot as plt

Y, X = np.mgrid[-3:3:100j, -3:3:100j]
U = 1 - (X ** 2 - Y ** 2) / (X ** 2 + Y ** 2 + 0.3) ** 2
V = -2 * X * Y / (X ** 2 + Y ** 2 + 0.3) ** 2
speed = np.sqrt(U ** 2 + V ** 2)
fig, ax = plt.subplots(figsize=(6, 5))
strm = ax.streamplot(X, Y, U, V, color=speed, cmap="autumn", density=1.2, linewidth=1 + speed / speed.max())
fig.colorbar(strm.lines, ax=ax)
ax.add_patch(plt.Circle((0, 0), 0.5, color="k"))
ax.set_aspect("equal")
print(type(strm).__name__)
fig.savefig("flow.png")
