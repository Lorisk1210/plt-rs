# Filled contours with explicit levels, extend='both' and hatched regions over a threshold.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 5, 100)
y = np.linspace(0, 4, 80)
X, Y = np.meshgrid(x, y)
Z = np.sin(X) * np.cos(Y) * 3
fig, ax = plt.subplots(figsize=(6, 4))
cf = ax.contourf(X, Y, Z, levels=[-2, -1, 0, 1, 2], cmap="PRGn", extend="both")
ax.contourf(X, Y, Z, levels=[2, 10], colors="none", hatches=["//"])
fig.colorbar(cf, ax=ax)
ax.set_title("levels with extensions")
print(cf.levels, cf.extend)
fig.savefig("filled.png")
