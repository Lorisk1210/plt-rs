# Heat map with pcolormesh and category labels centred on cells, gridlines between cells.
import numpy as np
import matplotlib.pyplot as plt

rows = ["north", "south", "east", "west"]
cols = ["Q1", "Q2", "Q3", "Q4", "Q5", "Q6"]
vals = np.arange(24).reshape(4, 6) % 7 + np.arange(4)[:, None]
fig, ax = plt.subplots(figsize=(6, 3.5))
mesh = ax.pcolormesh(vals, cmap="Oranges", edgecolors="k", linewidth=0.5)
ax.set_xticks(np.arange(len(cols)) + 0.5, cols)
ax.set_yticks(np.arange(len(rows)) + 0.5, rows)
ax.xaxis.tick_top()
ax.invert_yaxis()
fig.colorbar(mesh, ax=ax)
print(vals.max(), mesh.get_array().size)
fig.savefig("regions.png")
