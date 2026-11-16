# Edge detection by finite differences: gradient magnitude, direction as a cyclic colormap, and quiver of gradients.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:100j, 0:1:100j]
img = ((x - 0.5) ** 2 + (y - 0.5) ** 2 < 0.1).astype(float) + 0.5 * (x > 0.7)
gy, gx = np.gradient(img)
mag = np.hypot(gx, gy)
ang = np.arctan2(gy, gx)
fig, axs = plt.subplots(1, 4, figsize=(14, 3.5))
axs[0].imshow(img, cmap="gray")
im1 = axs[1].imshow(mag, cmap="inferno")
fig.colorbar(im1, ax=axs[1], shrink=0.7)
im2 = axs[2].imshow(np.ma.masked_where(mag < 0.05, ang), cmap="twilight", vmin=-np.pi, vmax=np.pi)
fig.colorbar(im2, ax=axs[2], shrink=0.7, ticks=[-np.pi, 0, np.pi]).ax.set_yticklabels(["-π", "0", "π"])
s = slice(None, None, 6)
axs[3].imshow(img, cmap="gray", alpha=0.5)
axs[3].quiver(np.arange(100)[s], np.arange(100)[s], gx[s, s], -gy[s, s], mag[s, s], cmap="autumn", scale=3)
for ax in axs:
    ax.axis("off")
print(round(float(mag.max()), 4), int((mag > 0.05).sum()))
fig.savefig("edges.png", dpi=50)
