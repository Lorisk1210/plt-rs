# 50k-point scatter saved as PDF with rasterized=True and small markers, plus PNG.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
x = rng.normal(0, 1, 50000)
y = x * 0.3 + rng.normal(0, 1, 50000)
fig, ax = plt.subplots(figsize=(5, 5))
ax.scatter(x, y, s=1, alpha=0.2, color="k", rasterized=True, linewidths=0)
ax.set_xlim(-4, 4)
ax.set_ylim(-4, 4)
ax.set_aspect("equal")
ax.set_title("50k points")
fig.savefig("cloud.pdf", dpi=150)
fig.savefig("cloud.png", dpi=80)
print(ax.collections[0].get_rasterized(), len(ax.collections[0].get_offsets()))
