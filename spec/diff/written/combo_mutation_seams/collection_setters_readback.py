# Change a scatter after creation (offsets, sizes, colours, array/cmap/clim, alpha, edge) via setters and read all back.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
sc = ax.scatter([1, 2, 3], [1, 2, 3], c=[1, 2, 3])
sc.set_offsets(np.array([[0, 0], [1, 1], [2, 0], [3, 1]]))
sc.set_sizes([10, 40, 90, 160])
sc.set_array(np.array([0.1, 0.4, 0.6, 0.9]))
sc.set_cmap("plasma")
sc.set_clim(0, 1)
sc.set_alpha(0.6)
sc.set_edgecolor("k")
sc.set_linewidth(2)
fig.canvas.draw()
print(sc.get_offsets().tolist(), sc.get_sizes().tolist(), sc.get_array().tolist())
print(sc.get_cmap().name, sc.get_clim(), sc.get_alpha(), np.round(sc.get_edgecolor(), 3).tolist(), sc.get_linewidth())
print(np.round(sc.get_facecolor(), 3).tolist())
ax.set_xlim(-1, 4); ax.set_ylim(-1, 2)
fig.savefig("collection_setters.png", dpi=50)
