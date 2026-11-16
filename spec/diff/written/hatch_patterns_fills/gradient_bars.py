# Bars filled with a vertical colour gradient by clipping an image to each bar.
import numpy as np
import matplotlib.pyplot as plt

values = [3, 6, 4, 8, 5]
fig, ax = plt.subplots(figsize=(6, 4))
bars = ax.bar(range(5), values, color="none", edgecolor="k")
grad = np.linspace(0, 1, 256).reshape(-1, 1)
for bar in bars:
    x, y = bar.get_xy()
    w, h = bar.get_width(), bar.get_height()
    im = ax.imshow(grad, extent=[x, x + w, y, y + h], origin="lower", aspect="auto", cmap="viridis", zorder=0)
    im.set_clip_path(bar)
ax.set_xlim(-0.6, 4.6)
ax.set_ylim(0, 9)
print(len(ax.images))
fig.savefig("gradient_bars.png")
