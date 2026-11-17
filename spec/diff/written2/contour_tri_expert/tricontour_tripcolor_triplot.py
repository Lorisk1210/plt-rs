# tricontour/tricontourf/tripcolor/triplot with masks and explicit triangles; returned artists and their array shapes.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.tri as mtri

n = 12
ang = np.linspace(0, 2 * np.pi, n, endpoint=False)
rad = np.linspace(0.25, 1, 5)
A, R = np.meshgrid(ang, rad)
A[1::2] += np.pi / n
x, y = (R * np.cos(A)).ravel(), (R * np.sin(A)).ravel()
z = np.cos(3 * np.arctan2(y, x)) * np.hypot(x, y)
tri = mtri.Triangulation(x, y)
xm, ym = x[tri.triangles].mean(axis=1), y[tri.triangles].mean(axis=1)
tri.set_mask(np.hypot(xm, ym) < 0.25)
fig, axs = plt.subplots(2, 2, figsize=(6, 6), dpi=50)
lines, markers = axs[0, 0].triplot(tri, "bo-", lw=0.5, ms=2)
print(type(lines).__name__, len(lines.get_xdata()), np.isnan(lines.get_xdata()).sum(), len(markers.get_xdata()))
tp = axs[0, 1].tripcolor(tri, z, shading="flat", cmap="RdBu")
print(type(tp).__name__, tp.get_array().shape, len(tp.get_paths()))
tp2 = axs[0, 1].tripcolor(tri, facecolors=np.arange(len(tri.triangles)), alpha=0.3)
print(tp2.get_array().shape)
tg = axs[1, 0].tripcolor(tri, z, shading="gouraud")
print(type(tg).__name__, tg.get_array().shape)
tc = axs[1, 1].tricontour(tri, z, levels=5, colors="k")
tcf = axs[1, 1].tricontourf(x, y, z, levels=5, cmap="viridis")
print(np.round(tc.levels, 3).tolist(), len(tc.get_paths()), len(tcf.get_paths()))
tcf2 = axs[1, 1].tricontourf(x, y, tri.triangles, z, levels=[-1, 0, 1])
print(len(tcf2.get_paths()), np.round(tcf2.levels, 2).tolist())
fig.canvas.draw()
print([round(v, 3) for v in axs[0, 0].get_xlim()])
try:
    axs[0, 1].tripcolor(tri, z, facecolors=z)
except ValueError as e:
    print("ValueError", str(e)[:60])
fig.savefig("tri.png")
