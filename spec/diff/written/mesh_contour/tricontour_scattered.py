# Contour irregularly spaced station data with tricontourf and plot the stations.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.tri as tri

rng = np.random.default_rng(7)
x = rng.uniform(-2, 2, 150)
y = rng.uniform(-2, 2, 150)
z = x * np.exp(-x ** 2 - y ** 2)
triang = tri.Triangulation(x, y)
fig, ax = plt.subplots(figsize=(5.5, 4.5))
tcf = ax.tricontourf(triang, z, levels=10, cmap="RdBu_r")
ax.tricontour(triang, z, levels=10, colors="k", linewidths=0.4)
ax.plot(x, y, "k.", ms=2)
fig.colorbar(tcf, ax=ax)
print(len(triang.triangles) > 0, len(tcf.levels))
fig.savefig("stations.png")
