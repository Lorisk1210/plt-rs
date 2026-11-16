# 3-D surface of a sinc function with a colour bar and a chosen viewing angle.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-6, 6, 60)
X, Y = np.meshgrid(x, x)
R = np.hypot(X, Y) + 1e-9
Z = np.sin(R) / R
fig = plt.figure(figsize=(7, 5))
ax = fig.add_subplot(projection="3d")
surf = ax.plot_surface(X, Y, Z, cmap="coolwarm", linewidth=0, antialiased=False, rstride=1, cstride=1)
fig.colorbar(surf, shrink=0.5, aspect=10)
ax.view_init(elev=30, azim=-60)
ax.set_zlim(-0.3, 1.0)
ax.set_xlabel("x")
ax.set_ylabel("y")
ax.set_zlabel("sinc r")
print(ax.elev, ax.azim, ax.get_zlim())
fig.savefig("sinc.png")
