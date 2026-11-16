# Gallery of 16 generated "digit-like" images in a 4x4 grid with titles and no axes.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(2)
fig, axs = plt.subplots(4, 4, figsize=(6, 6))
for k, ax in enumerate(axs.flat):
    img = np.zeros((16, 16))
    yy, xx = np.mgrid[0:16, 0:16]
    img += np.exp(-((xx - 8 - 4 * np.cos(k)) ** 2 + (yy - 8 - 4 * np.sin(k)) ** 2) / 8)
    img += rng.random((16, 16)) * 0.2
    ax.imshow(img, cmap="gray_r")
    ax.set_title(f"#{k} p={img.max():.2f}", fontsize=7)
    ax.axis("off")
fig.subplots_adjust(wspace=0.1, hspace=0.3)
print(axs.size)
fig.savefig("gallery.png")
