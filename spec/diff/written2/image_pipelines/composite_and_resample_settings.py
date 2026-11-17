# Several overlapping images in one axes with image.composite_image on/off and alpha; PNG pixels at overlaps and the SVG image count.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "composite"
fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
a = ax.imshow(np.zeros((4, 4)), extent=(0, 2, 0, 2), cmap="Reds", vmin=-1, vmax=1, alpha=0.6)
b = ax.imshow(np.ones((4, 4)), extent=(1, 3, 1, 3), cmap="Blues", vmin=0, vmax=1, alpha=0.6)
c = ax.imshow(np.eye(4), extent=(0.5, 2.5, 0.5, 2.5), cmap="Greens", alpha=0.4)
ax.set_xlim(0, 3)
ax.set_ylim(0, 3)
for composite in (True, False):
    with plt.rc_context({"image.composite_image": composite}):
        svg = io.BytesIO()
        fig.savefig(svg, format="svg", metadata={"Date": None})
        png = io.BytesIO()
        fig.savefig(png, format="png")
        png.seek(0)
        img = plt.imread(png)
        cx, cy = ax.transData.transform((1.5, 1.5))
        pix = img[img.shape[0] - int(cy), int(cx)]
        print(composite, svg.getvalue().count(b"<image"), np.round(pix, 2).tolist())
print(ax.get_images() == [a, b, c], [im.get_alpha() for im in ax.get_images()])
b.set_zorder(-1)
png = io.BytesIO()
fig.savefig(png, format="png")
png.seek(0)
img = plt.imread(png)
cx, cy = ax.transData.transform((1.5, 1.5))
print(np.round(img[img.shape[0] - int(cy), int(cx)], 2).tolist())
fig.savefig("composite.png")
