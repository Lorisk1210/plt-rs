# Show an RGB float image, a uint8 RGBA image with transparency, and a clipped float warning case.
import numpy as np
import matplotlib.pyplot as plt

h, w = 40, 60
rgb = np.zeros((h, w, 3))
rgb[..., 0] = np.linspace(0, 1, w)
rgb[..., 1] = np.linspace(0, 1, h)[:, None]
rgb[..., 2] = 0.5
rgba = (np.dstack([rgb, np.linspace(0, 1, w)[None, :].repeat(h, 0)]) * 255).astype(np.uint8)
fig, axs = plt.subplots(1, 3, figsize=(10, 3))
axs[0].imshow(rgb)
axs[0].set_title("RGB float")
axs[1].set_facecolor("0.5")
axs[1].imshow(rgba)
axs[1].set_title("RGBA uint8")
axs[2].imshow(rgb * 1.5)
axs[2].set_title("out of range")
print(rgb.dtype, rgba.dtype, rgba.shape)
fig.savefig("colors.png")
