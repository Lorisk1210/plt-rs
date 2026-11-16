# Horizontal colour bar under a map-like image, with ticks every 5 units.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[-5:5:120j, -8:8:200j]
elevation = 30 * np.exp(-((x - 2) ** 2 + y ** 2) / 8) + 12 * np.exp(-((x + 4) ** 2 + (y - 2) ** 2) / 4)
fig, ax = plt.subplots(figsize=(7, 4.5))
im = ax.imshow(elevation, extent=(-8, 8, -5, 5), cmap="terrain", origin="lower")
cb = fig.colorbar(im, ax=ax, orientation="horizontal", pad=0.12, aspect=40)
cb.set_ticks(np.arange(0, 35, 5))
cb.set_label("elevation [m]")
ax.set_title("Two hills")
print(cb.get_ticks())
fig.savefig("hills.png")
fig.savefig("hills_small.png", dpi=60)
