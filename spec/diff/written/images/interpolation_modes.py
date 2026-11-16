# Compare interpolation modes on a small 8x8 image upscaled in the figure.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(1)
img = rng.random((8, 8))
methods = ["none", "nearest", "bilinear", "bicubic", "spline16", "lanczos", "gaussian", "antialiased"]
fig, axs = plt.subplots(2, 4, figsize=(10, 5))
for ax, m in zip(axs.flat, methods):
    ax.imshow(img, interpolation=m, cmap="viridis")
    ax.set_title(m, fontsize=9)
    ax.set_axis_off()
print(axs[0, 2].images[0].get_interpolation())
fig.savefig("interpolation.png")
