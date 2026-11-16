# Split a synthetic RGB image into its channels and show each with a matching colormap plus histograms.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:120j, 0:1:160j]
img = np.dstack([x, y, 0.5 + 0.5 * np.sin(10 * x * y)])
fig, axs = plt.subplots(2, 4, figsize=(11, 5))
axs[0, 0].imshow(img)
axs[0, 0].set_title("RGB")
axs[1, 0].axis("off")
for k, (name, cmap) in enumerate([("red", "Reds"), ("green", "Greens"), ("blue", "Blues")], start=1):
    axs[0, k].imshow(img[..., k - 1], cmap=cmap, vmin=0, vmax=1)
    axs[0, k].set_title(name)
    axs[1, k].hist(img[..., k - 1].ravel(), bins=30, color=name)
for ax in axs[0]:
    ax.axis("off")
fig.tight_layout()
print(img.shape, [round(float(img[..., c].mean()), 4) for c in range(3)])
fig.savefig("channels.png")
