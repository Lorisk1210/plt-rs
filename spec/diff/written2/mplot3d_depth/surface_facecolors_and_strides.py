# plot_surface with rstride/cstride vs rcount/ccount, facecolors from a colormap, shade options; polygon counts and colours.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import cm

X, Y = np.meshgrid(np.linspace(-2, 2, 41), np.linspace(-2, 2, 31))
Z = np.exp(-(X ** 2 + Y ** 2))
fig = plt.figure(figsize=(9, 6), dpi=50)
variants = [dict(rstride=1, cstride=1), dict(rstride=5, cstride=10), dict(rcount=8, ccount=8), dict(rcount=100, ccount=100),
            dict(facecolors=cm.viridis(Z / Z.max()), shade=False), dict(cmap="coolwarm", antialiased=False, linewidth=0)]
surfs = []
for k, kw in enumerate(variants):
    ax = fig.add_subplot(2, 3, k + 1, projection="3d")
    s = ax.plot_surface(X, Y, Z, **kw)
    surfs.append(s)
fig.canvas.draw()
for kw, s in zip(variants, surfs):
    fc = s.get_facecolor()
    print(sorted(kw)[:2], len(s.get_paths()), len(fc), np.round(fc[0], 3).tolist())
print(surfs[-1].get_array().shape if surfs[-1].get_array() is not None else None, surfs[-1].get_cmap().name)
print(np.round(surfs[-1].get_clim(), 4).tolist() if surfs[-1].get_array() is not None else None)
ax = fig.axes[0]
wire = ax.plot_wireframe(X, Y, Z + 0.5, rcount=5, ccount=0, color="k")
print(len(wire.get_segments()), np.round(wire._segments3d[0][0], 3).tolist())
fig.savefig("surfaces.png")
tri = fig.axes[1].plot_trisurf(X.ravel()[::7], Y.ravel()[::7], Z.ravel()[::7], cmap="magma")
fig.canvas.draw()
print(len(tri.get_paths()), tri.get_array() is not None)
