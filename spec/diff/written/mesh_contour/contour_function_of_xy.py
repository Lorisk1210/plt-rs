# Plot a cost function's level sets with log-spaced levels and mark its minimum (Rosenbrock).
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm

x = np.linspace(-2, 2, 200)
y = np.linspace(-1, 3, 200)
X, Y = np.meshgrid(x, y)
Z = (1 - X) ** 2 + 100 * (Y - X ** 2) ** 2
fig, ax = plt.subplots(figsize=(5.5, 5))
cs = ax.contour(X, Y, Z, levels=np.logspace(-1, 3.5, 12), norm=LogNorm(), cmap="viridis")
ax.plot(1, 1, "r*", ms=15)
ax.clabel(cs, fmt="%.1f", fontsize=7)
ax.set_title("Rosenbrock")
print(np.round(cs.levels[:4], 3))
fig.savefig("rosenbrock.png")
