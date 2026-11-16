# Threshold segmentation: image, mask overlay with contour, labelled region sizes bar chart and an intensity histogram with the threshold.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(80)
y, x = np.mgrid[0:1:150j, 0:1:200j]
img = sum(np.exp(-((x - cx) ** 2 + (y - cy) ** 2) / r) for cx, cy, r in [(0.2, 0.3, 0.004), (0.6, 0.6, 0.01), (0.8, 0.2, 0.002)])
img = img + rng.normal(0, 0.05, img.shape)
thr = 0.4
mask = img > thr
labels = np.zeros_like(mask, dtype=int)
labels[mask & (x < 0.4)] = 1; labels[mask & (x >= 0.4) & (y > 0.4)] = 2; labels[mask & (x >= 0.7) & (y <= 0.4)] = 3
fig, axs = plt.subplots(2, 2, figsize=(10, 8))
axs[0, 0].imshow(img, cmap="gray")
axs[0, 1].imshow(img, cmap="gray")
axs[0, 1].imshow(np.ma.masked_where(labels == 0, labels), cmap="Set1", alpha=0.5, vmin=0, vmax=9)
axs[0, 1].contour(mask, levels=[0.5], colors="yellow", linewidths=0.8)
sizes = [int((labels == k).sum()) for k in (1, 2, 3)]
axs[1, 0].bar(["1", "2", "3"], sizes, color=[plt.cm.Set1(k / 9) for k in (1, 2, 3)])
axs[1, 1].hist(img.ravel(), bins=60, log=True)
axs[1, 1].axvline(thr, color="r")
for ax in axs[0]:
    ax.axis("off")
fig.tight_layout()
print(sizes, int(mask.sum()))
fig.savefig("segmentation.png", dpi=50)
