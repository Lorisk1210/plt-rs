# Heat transfer figure: temperature contours with labels, heat flux arrows (negative gradient) and a hot-spot marker.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 2, 60)
y = np.linspace(0, 1, 30)
X, Y = np.meshgrid(x, y)
T = 20 + 60 * np.exp(-((X - 0.6) ** 2 + (Y - 0.5) ** 2) / 0.05) + 30 * np.exp(-((X - 1.5) ** 2 + (Y - 0.3) ** 2) / 0.1)
dTy, dTx = np.gradient(T, y, x)
fig, ax = plt.subplots(figsize=(9, 4.5))
cf = ax.contourf(X, Y, T, levels=15, cmap="inferno")
cs = ax.contour(X, Y, T, levels=cf.levels[::3], colors="w", linewidths=0.7)
ax.clabel(cs, fmt="%.0f °C", fontsize=7)
ax.quiver(X[::3, ::4], Y[::3, ::4], -dTx[::3, ::4], -dTy[::3, ::4], color="cyan", scale=4000)
i, j = np.unravel_index(T.argmax(), T.shape)
ax.plot(X[i, j], Y[i, j], "c*", ms=15)
fig.colorbar(cf, ax=ax, label="T [°C]")
ax.set_aspect("equal")
print(round(float(T.max()), 3), (int(i), int(j)), len(cf.levels))
fig.savefig("heat.png", dpi=50)
