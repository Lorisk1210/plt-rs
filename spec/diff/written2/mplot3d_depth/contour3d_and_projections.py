# contour/contourf on 3-D axes with zdir and offset (projections onto the walls), extend3d, and tricontour in 3-D.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

X, Y = np.meshgrid(np.linspace(-3, 3, 40), np.linspace(-3, 3, 40))
Z = np.sin(X) * np.cos(Y)
fig = plt.figure(figsize=(8, 4), dpi=50)
ax = fig.add_subplot(1, 2, 1, projection="3d")
ax.plot_surface(X, Y, Z, alpha=0.3)
c1 = ax.contour(X, Y, Z, zdir="z", offset=-2, levels=6, cmap="coolwarm")
c2 = ax.contour(X, Y, Z, zdir="x", offset=-4, levels=4)
c3 = ax.contourf(X, Y, Z, zdir="y", offset=4, levels=5, cmap="viridis")
ax.set(xlim=(-4, 3), ylim=(-3, 4), zlim=(-2, 1))
c4 = ax.contour(X, Y, Z, levels=[0.0, 0.5], extend3d=True)
fig.canvas.draw()
for c in (c1, c2, c3, c4):
    print(type(c).__name__, np.round(c.levels, 3).tolist(), len(c.get_paths()))
ax2 = fig.add_subplot(1, 2, 2, projection="3d")
t = np.linspace(0, 2 * np.pi, 30)
r = np.linspace(0.2, 1, 6)
R, T = np.meshgrid(r, t)
x, y = (R * np.cos(T)).ravel(), (R * np.sin(T)).ravel()
tz = x * y
tc = ax2.tricontour(x, y, tz, levels=5)
tcf = ax2.tricontourf(x, y, tz, levels=4, zdir="z", offset=-0.5)
fig.canvas.draw()
print(np.round(tc.levels, 3).tolist(), np.round(tcf.levels, 3).tolist(), len(tcf.get_paths()))
print([np.round(f(), 3).tolist() for f in (ax2.get_xlim, ax2.get_zlim)])
fig.savefig("contour3d.png")
