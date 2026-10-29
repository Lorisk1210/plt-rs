"""3-D surface: the "peaks" function, three bumps and two wells,
coloured by height with a colorbar."""

import numpy as np

import pltrs.pyplot as plt

x = np.linspace(-3, 3, 80)
X, Y = np.meshgrid(x, x)
Z = (
    3 * (1 - X) ** 2 * np.exp(-(X**2) - (Y + 1) ** 2)
    - 10 * (X / 5 - X**3 - Y**5) * np.exp(-(X**2) - Y**2)
    - np.exp(-((X + 1) ** 2) - Y**2) / 3
)

fig = plt.figure(figsize=(6.4, 5.2), layout="constrained")
ax = fig.add_subplot(projection="3d")
surface = ax.plot_surface(X, Y, Z, cmap="viridis", rstride=1, cstride=1, linewidth=0, antialiased=False)
fig.colorbar(surface, ax=ax, shrink=0.6, label="height")
ax.view_init(elev=30, azim=-55)
ax.set_xlabel("x")
ax.set_ylabel("y")
ax.set_zlabel("z")
ax.set_title("The peaks function")

fig.savefig("surface_3d.png")
