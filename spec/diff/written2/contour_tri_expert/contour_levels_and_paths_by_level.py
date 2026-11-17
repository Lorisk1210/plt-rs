# ContourSet internals: levels chosen by locator/n, get_paths per level, allsegs/allkinds, filled layers and extend.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import ticker

x = np.linspace(-2, 2, 60)
X, Y = np.meshgrid(x, x)
Z = (X ** 2 - Y ** 2) * np.exp(-(X ** 2 + Y ** 2) / 2)
fig, axs = plt.subplots(1, 3, figsize=(8, 3))
cs = axs[0].contour(X, Y, Z, levels=7)
print(np.round(cs.levels, 4).tolist(), len(cs.get_paths()), cs.filled)
print([len(p.vertices) for p in cs.get_paths()])
print([len(s) for s in cs.allsegs], [len(k) for k in cs.allkinds][:3])
cf = axs[1].contourf(X, Y, Z, levels=[-0.3, -0.1, 0, 0.1, 0.3], extend="both")
print(np.round(cf.levels, 3).tolist(), np.round(cf.layers, 3).tolist(), len(cf.get_paths()), cf.extend)
print(np.round(cf._levels, 3).tolist())
cl = axs[2].contour(X, Y, Z, locator=ticker.MaxNLocator(4, symmetric=True))
print(np.round(cl.levels, 4).tolist())
cl2 = axs[2].contour(X, Y, np.abs(Z) + 1e-3, locator=ticker.LogLocator(), colors="k")
print(np.round(cl2.levels, 6).tolist(), cl2.logscale)
fig.canvas.draw()
print([round(v, 3) for v in cs.get_datalim(axs[0].transData).bounds], cs.zmin is not None and round(cs.zmin, 4))
c1 = cs.find_nearest_contour(0.5, 0.5, pixel=False)
print(c1[0], c1[1], [round(v, 3) for v in c1[2:4]], round(c1[4], 5))
fig.savefig("levels.png")
cf.set_cmap("plasma")
fig.canvas.draw()
print(np.round(cf.get_facecolor()[:2], 3).tolist(), cf.cmap.name)
cs.remove()
print(len(axs[0].collections))
