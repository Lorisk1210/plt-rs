# RGB image analysis: the image, its three channel histograms in matching colours, and a 2-D hue/value histogram.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors

y, x = np.mgrid[0:1:120j, 0:1:160j]
img = np.clip(np.dstack([x, 0.5 + 0.5 * np.sin(6 * y), 1 - x * y]), 0, 1)
hsv = mcolors.rgb_to_hsv(img)
fig = plt.figure(figsize=(11, 7))
ax = fig.add_subplot(2, 2, 1)
ax.imshow(img)
ax.axis("off")
ax2 = fig.add_subplot(2, 2, 2)
for k, c in enumerate("rgb"):
    ax2.hist(img[..., k].ravel(), bins=40, histtype="step", color=c, lw=1.5, label=c)
ax2.legend()
ax3 = fig.add_subplot(2, 2, 3)
h = ax3.hist2d(hsv[..., 0].ravel(), hsv[..., 2].ravel(), bins=40, cmap="magma")
ax3.set(xlabel="hue", ylabel="value")
fig.colorbar(h[3], ax=ax3)
ax4 = fig.add_subplot(2, 2, 4)
ax4.imshow(hsv[..., 0], cmap="hsv")
ax4.axis("off")
fig.tight_layout()
print(np.round(img.mean(axis=(0, 1)), 4).tolist(), round(float(hsv[..., 0].mean()), 4))
fig.savefig("channels.png", dpi=50)
