# Before/after comparison: left half of one image and right half of another with a divider.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:200j, 0:1:300j]
before = np.sin(20 * x) * np.cos(15 * y)
after = np.convolve(before.ravel(), np.ones(9) / 9, mode="same").reshape(before.shape)
combo = before.copy()
combo[:, 150:] = after[:, 150:]
fig, ax = plt.subplots(figsize=(6, 4))
ax.imshow(combo, cmap="gray")
ax.axvline(149.5, color="yellow", lw=2)
ax.text(10, 15, "before", color="yellow")
ax.text(160, 15, "after (smoothed)", color="yellow")
ax.axis("off")
print(round(float(np.abs(before - after).mean()), 4))
fig.savefig("before_after.png")
