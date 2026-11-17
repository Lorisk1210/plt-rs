# A 4000x4000 image shown at small sizes with several interpolation stages and resample settings; output pixel statistics.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

n = 4000
yy, xx = np.mgrid[0:n, 0:n]
data = np.sin(xx / 37.0) * np.cos(yy / 53.0) + ((xx // 400 + yy // 400) % 2) * 0.5
fig, axs = plt.subplots(1, 3, figsize=(6, 2), dpi=80)
specs = [dict(interpolation="nearest"), dict(interpolation="antialiased", interpolation_stage="data"),
         dict(interpolation="bilinear", interpolation_stage="rgba", resample=True)]
ims = [ax.imshow(data, cmap="gray", **kw) for ax, kw in zip(axs, specs)]
buf = io.BytesIO()
fig.savefig(buf, format="png")
buf.seek(0)
img = plt.imread(buf)
print(img.shape, [round(float(v), 4) for v in img[..., :3].mean(axis=(0, 1))])
for ax, im in zip(axs, ims):
    bb = ax.get_window_extent()
    x0, y0, x1, y1 = (int(round(v)) for v in (bb.x0, bb.y0, bb.x1, bb.y1))
    part = img[img.shape[0] - y1:img.shape[0] - y0, x0:x1, 0]
    print(im.get_interpolation(), im.get_interpolation_stage(), part.shape, round(float(part.mean()), 3), round(float(part.std()), 3))
print(ims[0].get_array().shape, ims[0].get_array().dtype, ims[0].get_extent())
for im in ims:
    im.set_clim(-0.5, 1.0)
buf2 = io.BytesIO()
fig.savefig(buf2, format="png", dpi=40)
buf2.seek(0)
small = plt.imread(buf2)
print(small.shape, round(float(small[..., 0].mean()), 4))
