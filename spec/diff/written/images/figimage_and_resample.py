# Large image downsampled in a small axes (antialiasing) and a figimage placed in pixels.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:1000j, 0:1:1000j]
rings = np.sin(200 * np.hypot(x - 0.5, y - 0.5) ** 2)
fig = plt.figure(figsize=(6, 3), dpi=80)
ax1 = fig.add_subplot(1, 2, 1)
ax1.imshow(rings, cmap="gray", interpolation="antialiased")
ax1.set_title("antialiased")
ax2 = fig.add_subplot(1, 2, 2)
ax2.imshow(rings, cmap="gray", interpolation="nearest")
ax2.set_title("nearest")
small = np.outer(np.linspace(0, 1, 30), np.ones(30))
fig.figimage(small, xo=10, yo=10, cmap="hot", origin="upper")
fig.savefig("moire.png")
print(len(fig.images))
