# Overlay a semi-transparent mask on a grayscale image, with a per-pixel alpha array.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:120j, 0:1:160j]
base = 0.5 + 0.5 * np.sin(20 * x) * np.sin(15 * y)
mask = ((x - 0.5) ** 2 / 0.08 + (y - 0.5) ** 2 / 0.05) < 1
alpha = np.where(mask, 0.5, 0.0)
fig, ax = plt.subplots(figsize=(6, 4.5))
ax.imshow(base, cmap="gray")
ax.imshow(mask, cmap="autumn", alpha=alpha)
ax.contour(mask, levels=[0.5], colors="red", linewidths=1)
ax.set_title("segmentation overlay")
print(int(mask.sum()))
fig.savefig("overlay.png")
