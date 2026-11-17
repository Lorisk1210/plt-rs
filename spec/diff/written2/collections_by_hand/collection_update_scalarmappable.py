# The ScalarMappable side of collections: update_scalarmappable, colorizer sharing between two collections, set_norm and callbacks.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
sc1 = ax.scatter([0, 1, 2], [0, 1, 2], c=[1, 2, 3], cmap="viridis")
sc2 = ax.scatter([0, 1, 2], [1, 2, 3], c=[10, 20, 30])
print(sc1.get_clim(), sc2.get_clim(), sc1.cmap.name, sc2.cmap.name)
log = []
sc1.callbacks.connect("changed", lambda m: log.append(("changed", type(m).__name__)))
sc1.set_clim(0, 5)
sc1.set_cmap("magma")
print(log)
sc2.colorizer = sc1.colorizer
fig.canvas.draw()
print(sc2.get_clim(), sc2.cmap.name)
sc1.set_clim(0, 50)
fig.canvas.draw()
print(sc2.get_clim(), np.round(sc2.get_facecolor()[0], 3).tolist())
sc1.set_array(np.array([5.0, 25, 45]))
sc1.update_scalarmappable()
print(np.round(sc1.get_facecolor()[:, 0], 3).tolist())
sc1.set_norm(matplotlib.colors.LogNorm(1, 100))
fig.canvas.draw()
print(type(sc1.norm).__name__, np.round(sc1.get_facecolor()[:, 0], 3).tolist(), type(sc2.norm).__name__)
cb = fig.colorbar(sc1, ax=ax)
print(type(cb.norm).__name__, [round(t, 2) for t in cb.get_ticks()][:3])
print(sc1.to_rgba(np.array([1.0, 10.0])).round(3).tolist())
sc1.set_array(np.ma.masked_array([1.0, 2, 3], [0, 1, 0]))
fig.canvas.draw()
print(np.round(sc1.get_facecolor()[1], 3).tolist(), sc1.cmap.get_bad().tolist())
fig.savefig("mappable.png")
