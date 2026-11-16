# Photo-like heat map exported as JPEG with quality settings and as a palette PNG via Pillow options.
import numpy as np
import matplotlib.pyplot as plt
from PIL import Image

y, x = np.mgrid[-3:3:200j, -3:3:200j]
z = np.exp(-(x ** 2 + y ** 2) / 2) + 0.3 * np.sin(3 * x) * np.cos(2 * y)

fig, ax = plt.subplots(figsize=(4, 4))
im = ax.imshow(z, cmap="magma", extent=(-3, 3, -3, 3))
fig.colorbar(im, ax=ax, shrink=0.8)
ax.set_title("Field")

fig.savefig("field_q95.jpg", pil_kwargs={"quality": 95})
fig.savefig("field_q30.jpg", pil_kwargs={"quality": 30, "optimize": True})
fig.savefig("field.tiff", pil_kwargs={"compression": "tiff_lzw"})
for name in ["field_q95.jpg", "field_q30.jpg", "field.tiff"]:
    with Image.open(name) as im2:
        print(name, im2.format, im2.size, im2.mode)
