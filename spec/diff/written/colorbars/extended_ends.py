# Colour bar with triangular extensions for values out of range, plus over/under colours.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-3, 3, 100)
z = np.outer(np.sin(x), np.cos(x)) * 1.6
cmap = plt.get_cmap("coolwarm").copy()
cmap.set_over("magenta")
cmap.set_under("cyan")
fig, ax = plt.subplots(figsize=(5, 4))
pcm = ax.pcolormesh(x, x, z, cmap=cmap, vmin=-1, vmax=1, shading="nearest")
cb = fig.colorbar(pcm, ax=ax, extend="both", extendfrac=0.1)
cb.set_label("clipped to [-1, 1]")
print(cb.extend, cmap.get_over(), cmap.get_under())
fig.savefig("extended.png")
