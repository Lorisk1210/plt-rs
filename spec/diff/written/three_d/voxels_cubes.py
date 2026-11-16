# Voxel plot of two objects with different colours and edge colours.
import numpy as np
import matplotlib.pyplot as plt

x, y, z = np.indices((8, 8, 8))
cube = (x < 3) & (y < 3) & (z < 3)
sphere = (x - 5) ** 2 + (y - 5) ** 2 + (z - 5) ** 2 <= 5
filled = cube | sphere
colors = np.empty(filled.shape, dtype=object)
colors[cube] = "tab:red"
colors[sphere] = "tab:blue"
fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(projection="3d")
ax.voxels(filled, facecolors=colors, edgecolor="k", linewidth=0.3)
print(int(filled.sum()))
fig.savefig("voxels.png")
