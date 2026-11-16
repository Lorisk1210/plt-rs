# Satellite-like image with geographic extent and GPS track plotted in the same coordinates.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:150j, 0:1:200j]
terrain = np.sin(6 * x) * np.cos(4 * y) + x
fig, ax = plt.subplots(figsize=(6, 4.5))
ax.imshow(terrain, extent=(8.40, 8.60, 49.00, 49.15), origin="lower", cmap="gist_earth", aspect="auto")
t = np.linspace(0, 1, 100)
ax.plot(8.42 + 0.15 * t, 49.02 + 0.1 * t ** 2, "r-", lw=2, label="track")
ax.plot(8.42, 49.02, "go", label="start")
ax.plot(8.57, 49.12, "rs", label="end")
ax.legend(loc="upper left")
ax.set_xlabel("lon")
ax.set_ylabel("lat")
print([round(v, 3) for v in ax.get_xlim()])
fig.savefig("track.png")
