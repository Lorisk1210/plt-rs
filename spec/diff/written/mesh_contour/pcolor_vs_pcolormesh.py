# Old pcolor() on a small grid with masked data next to pcolormesh, and an edge grid.
import numpy as np
import matplotlib.pyplot as plt

z = np.ma.masked_less(np.arange(30).reshape(5, 6) % 7, 1)
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3.5))
p = a.pcolor(z, edgecolors="w", linewidths=1, cmap="Oranges")
a.set_title("pcolor")
m = b.pcolormesh(z, cmap="Oranges")
b.set_title("pcolormesh")
fig.colorbar(p, ax=a)
print(type(p).__name__, type(m).__name__)
fig.savefig("pcolor.png")
fig.savefig("pcolor_vs_pcolormesh_small.png", dpi=50)
print(len(fig.axes))
