# Topographic contour lines with inline labels at chosen levels and a negative dashed style.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-3, 3, 150)
X, Y = np.meshgrid(x, x)
Z = np.exp(-X ** 2 - Y ** 2) - np.exp(-(X - 1) ** 2 - (Y - 1) ** 2)
fig, ax = plt.subplots(figsize=(5.5, 5))
cs = ax.contour(X, Y, Z, levels=np.arange(-1.0, 1.01, 0.2), colors="k")
labels = ax.clabel(cs, inline=True, fontsize=8, fmt="%.1f")
ax.set_aspect("equal")
ax.set_title("negative contours dashed")
print(len(cs.levels), len(labels), cs.get_linestyles() if hasattr(cs, "get_linestyles") else None)
fig.savefig("contours.png")
