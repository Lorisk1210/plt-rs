# Images through colormaps with set_bad/set_under/set_over, masked data and NaN, resampled LUTs and the colorbar extensions they imply.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

data = np.linspace(-0.5, 1.5, 20).reshape(4, 5)
data[1, 2] = np.nan
masked = np.ma.masked_where(data > 1.2, data)
cmap = plt.get_cmap("viridis").resampled(8).with_extremes(bad="magenta", under="black", over="white")
fig, (a1, a2) = plt.subplots(1, 2, figsize=(6, 3), dpi=40)
im1 = a1.imshow(data, cmap=cmap, vmin=0, vmax=1, interpolation="nearest")
im2 = a2.imshow(masked, cmap=cmap, vmin=0, vmax=1, interpolation="nearest")
cb = fig.colorbar(im1, ax=a1, extend="both")
fig.canvas.draw()
rgba = im1.to_rgba(data)
print(np.round(rgba[0, 0], 3).tolist(), np.round(rgba[1, 2], 3).tolist(), np.round(rgba[-1, -1], 3).tolist())
print(np.round(im2.to_rgba(masked)[-1, -1], 3).tolist(), cmap.N, cmap.get_under().tolist(), cmap.get_over().tolist())
print(cb.extend, len(cb._extend_patches), cmap._rgba_bad)
lut = cmap(np.arange(cmap.N))
print(np.round(lut[[0, -1]], 3).tolist())
print(cmap(-1.0) == cmap.get_under().tolist() or np.allclose(cmap(-1.0), cmap.get_under()), np.allclose(cmap(np.nan), cmap.get_bad()))
rev = cmap.reversed()
print(rev.name, np.allclose(rev(0.0), cmap(1.0)), np.allclose(rev.get_under(), cmap.get_under()))
copy = cmap.copy()
copy.set_bad("cyan", alpha=0.5)
print(copy.get_bad().tolist(), cmap.get_bad().tolist())
fig.savefig("bad_values.png")
print(im1.get_cmap().name, im1.cmap is cmap, im1.get_cmap() is cmap)
print(np.round(im2.make_image(fig.canvas.get_renderer(), unsampled=True)[0][-1, -1] / 255, 3).tolist())
