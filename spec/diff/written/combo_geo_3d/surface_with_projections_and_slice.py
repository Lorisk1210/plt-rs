# 3-D surface with contour projections on the walls, a 2-D contour map beside it, and a 1-D slice through the peak.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-3, 3, 60)
X, Y = np.meshgrid(x, x)
Z = np.exp(-(X ** 2 + Y ** 2) / 2) * np.cos(X) + 0.3 * np.exp(-((X - 1.5) ** 2 + (Y + 1) ** 2))
fig = plt.figure(figsize=(12, 4.5))
ax3 = fig.add_subplot(1, 3, 1, projection="3d")
surf = ax3.plot_surface(X, Y, Z, cmap="viridis", alpha=0.85)
ax3.contour(X, Y, Z, zdir="z", offset=Z.min() - 0.3, cmap="viridis")
ax3.set_zlim(Z.min() - 0.3, Z.max())
ax3.view_init(25, -50)
ax2 = fig.add_subplot(1, 3, 2)
cf = ax2.contourf(X, Y, Z, levels=12, cmap="viridis")
i, j = np.unravel_index(Z.argmax(), Z.shape)
ax2.axhline(x[i], color="r")
fig.colorbar(cf, ax=ax2)
ax1 = fig.add_subplot(1, 3, 3)
ax1.plot(x, Z[i], color="r")
ax1.set_title(f"slice y = {x[i]:.2f}")
fig.tight_layout()
print(round(float(Z.max()), 4), (int(i), int(j)), [round(v, 3) for v in ax3.get_zlim()])
fig.savefig("surface_slice.png", dpi=50)
