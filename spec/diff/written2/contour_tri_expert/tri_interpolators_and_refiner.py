# LinearTriInterpolator, CubicTriInterpolator (kinds) and UniformTriRefiner: values and gradients at fixed points, refined sizes.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.tri as mtri

r = np.linspace(0.3, 1, 5)
t = np.linspace(0, np.pi, 9)
R, T = np.meshgrid(r, t)
x, y = (R * np.cos(T)).ravel(), (R * np.sin(T)).ravel()
z = x ** 2 - y
tri = mtri.Triangulation(x, y)
pts = np.array([[0.5, 0.3], [-0.6, 0.4], [0.0, 0.8], [2.0, 2.0]])
lin = mtri.LinearTriInterpolator(tri, z)
print(np.round(lin(pts[:, 0], pts[:, 1]), 4).tolist())
print([np.round(g, 4).tolist() for g in lin.gradient(pts[:, 0], pts[:, 1])])
for kind in ("min_E", "geom"):
    cub = mtri.CubicTriInterpolator(tri, z, kind=kind)
    print(kind, np.round(cub(pts[:, 0], pts[:, 1]), 4).tolist())
dz = np.c_[2 * x, -np.ones_like(x)]
cub_u = mtri.CubicTriInterpolator(tri, z, kind="user", dz=(dz[:, 0], dz[:, 1]))
print("user", np.round(cub_u(pts[:, 0], pts[:, 1]), 4).tolist())
refiner = mtri.UniformTriRefiner(tri)
for subdiv in (1, 2):
    rt, rz = refiner.refine_field(z, subdiv=subdiv)
    print(subdiv, len(rt.x), len(rt.triangles), round(float(rz.max()), 4))
rt2, idx = refiner.refine_triangulation(return_tri_index=True, subdiv=1)
print(len(rt2.triangles), np.bincount(idx).tolist()[:5])
fig, ax = plt.subplots(figsize=(4, 3))
rt, rz = refiner.refine_field(z, triinterpolator=lin, subdiv=2)
tc = ax.tricontourf(rt, rz, levels=6)
ax.triplot(tri, "k-", lw=0.3)
fig.canvas.draw()
print(np.round(tc.levels, 3).tolist(), len(ax.lines))
fig.savefig("refined.png")
