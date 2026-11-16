# Filled contours with a colour bar, and the contour lines added onto the colour bar.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-2, 2, 120)
X, Y = np.meshgrid(x, x)
Z = (1 - X / 2 + X ** 5 + Y ** 3) * np.exp(-X ** 2 - Y ** 2)
fig, ax = plt.subplots(figsize=(5.5, 4.5))
cf = ax.contourf(X, Y, Z, levels=12, cmap="RdYlBu_r")
cl = ax.contour(X, Y, Z, levels=cf.levels[::2], colors="k", linewidths=0.6)
cb = fig.colorbar(cf, ax=ax)
cb.add_lines(cl)
cb.set_label("height")
print(len(cf.levels), np.round(cf.levels[:3], 3))
fig.savefig("contour_bar.png")
