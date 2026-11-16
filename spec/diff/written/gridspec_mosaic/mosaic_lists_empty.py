# Mosaic from a nested list with an empty cell ('.') and per-key subplot keywords.
import numpy as np
import matplotlib.pyplot as plt

fig, axd = plt.subplot_mosaic(
    [["hist", "."], ["scatter", "histy"]],
    width_ratios=[3, 1], height_ratios=[1, 3],
    per_subplot_kw={"scatter": {"facecolor": "#fafafa"}},
    figsize=(5, 5),
)
np.random.seed(9)
x, y = np.random.randn(2, 300)
axd["scatter"].scatter(x, y, s=4)
axd["hist"].hist(x, bins=20)
axd["histy"].hist(y, bins=20, orientation="horizontal")
axd["hist"].sharex(axd["scatter"])
print(sorted(axd.keys()))
print(axd["scatter"].get_facecolor())
fig.savefig("mosaic_empty.png")
