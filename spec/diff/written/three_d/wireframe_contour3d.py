# Wireframe with contours projected on the walls (contour with zdir and offset).
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-3, 3, 40)
X, Y = np.meshgrid(x, x)
Z = np.exp(-(X ** 2 + Y ** 2) / 3) * np.cos(X)
fig = plt.figure(figsize=(6, 5))
ax = fig.add_subplot(projection="3d")
ax.plot_wireframe(X, Y, Z, rstride=3, cstride=3, color="k", lw=0.5)
ax.contour(X, Y, Z, zdir="z", offset=-1, cmap="coolwarm")
ax.contour(X, Y, Z, zdir="x", offset=-3, cmap="coolwarm")
ax.contourf(X, Y, Z, zdir="y", offset=3, cmap="coolwarm", alpha=0.5)
ax.set(zlim=(-1, 1), xlim=(-3, 3), ylim=(-3, 3))
print(len(ax.collections))
fig.savefig("wire.png")
