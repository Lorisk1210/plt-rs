# Draw a small network: nodes on a circle with scatter, edges with LineCollection, labels.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection

nodes = ["A", "B", "C", "D", "E", "F"]
edges = [(0, 1), (0, 2), (1, 3), (2, 3), (3, 4), (4, 5), (5, 0), (1, 4)]
ang = np.linspace(0, 2 * np.pi, len(nodes), endpoint=False)
pos = np.c_[np.cos(ang), np.sin(ang)]
degree = np.bincount(np.array(edges).ravel(), minlength=len(nodes))
fig, ax = plt.subplots(figsize=(5, 5))
ax.add_collection(LineCollection([pos[[i, j]] for i, j in edges], colors="0.6", linewidths=2, zorder=1))
ax.scatter(pos[:, 0], pos[:, 1], s=300 * degree, c=degree, cmap="Oranges", edgecolors="k", zorder=2)
for name, (x, y) in zip(nodes, pos):
    ax.text(x, y, name, ha="center", va="center", fontweight="bold")
ax.set_aspect("equal")
ax.axis("off")
ax.margins(0.2)
print(degree.tolist())
fig.savefig("network.png")
