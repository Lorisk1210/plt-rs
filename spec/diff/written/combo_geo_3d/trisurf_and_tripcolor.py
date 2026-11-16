# Scattered measurements as a 3-D trisurf, a 2-D tripcolor with the triangulation drawn, and a tricontour.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.tri as mtri

rng = np.random.default_rng(62)
x, y = rng.uniform(-1, 1, (2, 120))
z = np.cos(3 * x) * np.sin(2 * y)
tri = mtri.Triangulation(x, y)
fig = plt.figure(figsize=(12, 4))
a = fig.add_subplot(1, 3, 1, projection="3d")
a.plot_trisurf(tri, z, cmap="coolwarm")
b = fig.add_subplot(1, 3, 2)
tp = b.tripcolor(tri, z, cmap="coolwarm", shading="gouraud")
b.triplot(tri, "k-", lw=0.2)
fig.colorbar(tp, ax=b)
c = fig.add_subplot(1, 3, 3)
tc = c.tricontourf(tri, z, levels=10, cmap="coolwarm")
c.plot(x, y, "k.", ms=2)
for ax in (b, c):
    ax.set_aspect("equal")
fig.tight_layout()
print(len(tri.triangles), len(tc.levels))
fig.savefig("tri.png", dpi=50)
