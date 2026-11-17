# Save a PNG and an EPS of a small chart, then load the PNG with imread into another figure; compare sizes and EPS bounds.
import io
import re
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.image as mimage

small, ax = plt.subplots(figsize=(2, 1.5), dpi=100)
ax.bar(["a", "b", "c"], [3, 1, 2], color=["C0", "C1", "C2"])
small.savefig("small.png")
small.savefig("small.eps")
img = mimage.imread("small.png")
print(img.shape, img.dtype, round(float(img[..., 3].min()), 3))
big, (a1, a2) = plt.subplots(1, 2, figsize=(5, 2.5), dpi=80)
a1.imshow(img)
a2.imshow(img[::2, ::2], interpolation="nearest")
a2.set_title("downsampled")
big.savefig("big.png")
print(mimage.imread("big.png").shape)
with open("small.eps", "rb") as f:
    eps = f.read()
print(re.search(rb"%%BoundingBox: ([-\d ]+)", eps).group(1), eps.count(b"\n") > 50)
thumb = mimage.thumbnail("small.png", "thumb.png", scale=0.5)
print(type(thumb).__name__, mimage.imread("thumb.png").shape)
buf = io.BytesIO()
mimage.imsave(buf, img[..., :3], format="png")
buf.seek(0)
print(mimage.imread(buf).shape)
mimage.imsave("gray.png", img[..., 0], cmap="gray", vmin=0, vmax=1)
print(mimage.imread("gray.png").shape, round(float(mimage.imread("gray.png")[..., 0].mean()), 4))
