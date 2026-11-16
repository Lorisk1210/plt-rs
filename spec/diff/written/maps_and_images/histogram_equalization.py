# Contrast stretching: an image before and after histogram equalization, with both histograms.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(4)
img = np.clip(rng.normal(0.4, 0.08, (100, 100)) + np.linspace(0, 0.1, 100), 0, 1)
hist, bins = np.histogram(img, bins=256, range=(0, 1))
cdf = hist.cumsum() / hist.sum()
eq = np.interp(img.ravel(), bins[:-1], cdf).reshape(img.shape)
fig, axs = plt.subplots(2, 2, figsize=(7, 6))
axs[0, 0].imshow(img, cmap="gray", vmin=0, vmax=1)
axs[0, 1].imshow(eq, cmap="gray", vmin=0, vmax=1)
axs[1, 0].hist(img.ravel(), bins=64)
axs[1, 1].hist(eq.ravel(), bins=64)
for ax, t in zip(axs[0], ["original", "equalized"]):
    ax.set_title(t)
    ax.axis("off")
print(round(float(eq.std()), 4))
fig.savefig("equalized.png")
