# 3-D bar chart of sales per product and quarter with bar3d and shading.
import numpy as np
import matplotlib.pyplot as plt

sales = np.array([[5, 7, 8, 6], [3, 4, 6, 9], [8, 6, 5, 4]])
fig = plt.figure(figsize=(7, 5))
ax = fig.add_subplot(projection="3d")
xpos, ypos = np.meshgrid(np.arange(4), np.arange(3))
xpos, ypos = xpos.ravel(), ypos.ravel()
dz = sales.ravel()
colors = plt.cm.tab10(ypos)
ax.bar3d(xpos, ypos, np.zeros_like(dz), 0.6, 0.6, dz, color=colors, shade=True)
ax.set_xticks(np.arange(4) + 0.3, ["Q1", "Q2", "Q3", "Q4"])
ax.set_yticks(np.arange(3) + 0.3, ["A", "B", "C"])
ax.set_zlabel("units")
print(dz.sum())
fig.savefig("bars3d.png")
