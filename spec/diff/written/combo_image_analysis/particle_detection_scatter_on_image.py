# Particle detection: local maxima found with numpy on a blurred image, plotted as circles, plus size/brightness scatter.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(81)
y, x = np.mgrid[0:100, 0:120]
centers = rng.uniform([10, 10], [90, 110], (12, 2))
img = sum(rng.uniform(0.5, 1) * np.exp(-((x - cx) ** 2 + (y - cy) ** 2) / rng.uniform(5, 20)) for cy, cx in centers)
img += rng.normal(0, 0.02, img.shape)
padded = np.pad(img, 1, constant_values=-np.inf)
neigh = np.max([padded[1 + dy:101 + dy, 1 + dx:121 + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1) if dy or dx], axis=0)
peaks = np.argwhere((img > neigh) & (img > 0.3))
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 4.5))
a.imshow(img, cmap="gray")
a.scatter(peaks[:, 1], peaks[:, 0], s=80, facecolors="none", edgecolors="red")
bright = img[peaks[:, 0], peaks[:, 1]]
b.scatter(peaks[:, 1], bright, c=bright, cmap="hot", s=40, edgecolors="k")
b.set(xlabel="x position", ylabel="peak brightness")
print(len(peaks), round(float(bright.max()), 4))
fig.savefig("particles.png", dpi=50)
