# A 3-D volume shown as voxels above a threshold, plus three orthogonal slice images with crosshairs.
import numpy as np
import matplotlib.pyplot as plt

z, y, x = np.mgrid[0:1:16j, 0:1:16j, 0:1:16j]
vol = np.exp(-((x - 0.5) ** 2 + (y - 0.4) ** 2 + (z - 0.6) ** 2) / 0.05)
fig = plt.figure(figsize=(12, 4))
a = fig.add_subplot(1, 4, 1, projection="3d")
filled = vol > 0.5
colors = plt.cm.inferno(vol)
a.voxels(filled, facecolors=colors, edgecolor="none")
ci = np.unravel_index(vol.argmax(), vol.shape)
for k, (img, name, cross) in enumerate([(vol[ci[0]], "axial", (ci[2], ci[1])), (vol[:, ci[1]], "coronal", (ci[2], ci[0])),
                                         (vol[:, :, ci[2]], "sagittal", (ci[1], ci[0]))], start=2):
    ax = fig.add_subplot(1, 4, k)
    ax.imshow(img, cmap="inferno", origin="lower")
    ax.axvline(cross[0], color="c", lw=0.8)
    ax.axhline(cross[1], color="c", lw=0.8)
    ax.set_title(name)
fig.tight_layout()
print(int(filled.sum()), [int(v) for v in ci])
fig.savefig("volume.png", dpi=50)
