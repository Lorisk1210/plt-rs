# The same function as wireframe from four viewpoints and a filled contour in the last panel, sharing a colormap.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-2, 2, 30)
X, Y = np.meshgrid(x, x)
Z = X * np.exp(-X ** 2 - Y ** 2)
fig = plt.figure(figsize=(12, 7))
for k, (elev, azim) in enumerate([(30, -60), (60, 30), (10, 120), (90, -90)]):
    ax = fig.add_subplot(2, 3, k + 1, projection="3d")
    ax.plot_wireframe(X, Y, Z, rstride=2, cstride=2, color=plt.cm.plasma(k / 4), lw=0.6)
    ax.view_init(elev, azim)
    ax.set_title(f"{elev}/{azim}", fontsize=9)
ax5 = fig.add_subplot(2, 3, 5)
cf = ax5.contourf(X, Y, Z, levels=10, cmap="plasma")
ax5.set_aspect("equal")
fig.colorbar(cf, ax=ax5)
print([round(a.elev, 1) for a in fig.axes if a.name == "3d"], len(cf.levels))
fig.savefig("views.png", dpi=50)
