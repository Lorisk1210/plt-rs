# Combine tiles into one mosaic image with numpy and show tile borders and indices.
import numpy as np
import matplotlib.pyplot as plt

tiles = [np.full((20, 20), k / 8.0) + np.eye(20) * 0.2 for k in range(9)]
mosaic = np.block([[tiles[r * 3 + c] for c in range(3)] for r in range(3)])
fig, ax = plt.subplots(figsize=(5, 5))
ax.imshow(mosaic, cmap="viridis")
for k in range(1, 3):
    ax.axhline(k * 20 - 0.5, color="w")
    ax.axvline(k * 20 - 0.5, color="w")
for r in range(3):
    for c in range(3):
        ax.text(c * 20 + 10, r * 20 + 10, str(r * 3 + c), ha="center", va="center", color="w", fontsize=14)
ax.axis("off")
print(mosaic.shape)
fig.savefig("mosaic.png")
