# Read an image's array and extent back to compute a row profile in data units and a histogram with the image's norm limits.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:50j, 0:1:80j]
fig, axs = plt.subplots(1, 3, figsize=(13, 3.5))
im = axs[0].imshow(np.sin(8 * x) * y, extent=(0, 8, 0, 5), cmap="RdBu")
arr = im.get_array()
x0, x1, y0, y1 = im.get_extent()
row = arr.shape[0] // 2
axs[1].plot(np.linspace(x0, x1, arr.shape[1]), arr[row])
axs[2].hist(np.asarray(arr).ravel(), bins=30, range=im.get_clim())
print(arr.shape, [x0, x1, y0, y1], [round(float(v), 4) for v in im.get_clim()])
fig.savefig("image_profile.png", dpi=50)
