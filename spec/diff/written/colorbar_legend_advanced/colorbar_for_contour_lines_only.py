# Colour bar for a line-only contour set (no fill) and for a scatter with a categorical-like norm.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-3, 3, 100)
X, Y = np.meshgrid(x, x)
Z = np.sin(X) * np.cos(Y)
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
cs = a.contour(X, Y, Z, levels=7, cmap="plasma")
fig.colorbar(cs, ax=a)
sc = b.scatter(X[::10, ::10], Y[::10, ::10], c=np.round(Z[::10, ::10] * 2), cmap=plt.get_cmap("tab10", 5))
fig.colorbar(sc, ax=b, ticks=[-2, -1, 0, 1, 2])
print(len(cs.levels))
fig.savefig("line_bars.png")
