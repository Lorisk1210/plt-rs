# Read subplots' positions after layout, then add new axes aligned to them (a header strip and a side panel).
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 2, figsize=(8, 6))
for ax in axs.flat:
    ax.plot(np.random.default_rng(192).normal(size=20).cumsum())
fig.tight_layout(rect=(0, 0, 0.85, 0.9))
top = axs[0, 0].get_position()
right = axs[0, 1].get_position()
header = fig.add_axes([top.x0, 0.92, right.x1 - top.x0, 0.05])
header.axis("off")
header.text(0.5, 0.5, "aligned header", ha="center", va="center", fontsize=12)
low = axs[1, 1].get_position()
side = fig.add_axes([0.88, low.y0, 0.08, right.y1 - low.y0])
side.barh(range(4), [3, 1, 2, 4])
print([round(v, 3) for v in top.bounds], [round(v, 3) for v in side.get_position().bounds])
fig.savefig("aligned.png", dpi=50)
