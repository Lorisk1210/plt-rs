# Shade time ranges spanning the full height with a blended transform (x data, y axes).
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.transforms as mtransforms

x = np.arange(100)
y = np.cumsum(np.sin(x / 5))
fig, ax = plt.subplots(figsize=(7, 3.5))
ax.plot(x, y)
trans = mtransforms.blended_transform_factory(ax.transData, ax.transAxes)
ax.fill_between(x, 0, 1, where=y > 2, facecolor="green", alpha=0.3, transform=trans)
ax.text(10, 0.9, "label at 90% height", transform=trans)
ax.axhline(2, ls="--", color="k")
print(trans.transform((10, 0.9)).round(3))
fig.savefig("blended.png")
