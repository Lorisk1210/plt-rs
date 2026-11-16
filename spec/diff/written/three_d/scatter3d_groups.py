# 3-D scatter of three clusters with depth shading off and a legend.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(4)
fig = plt.figure(figsize=(6, 5))
ax = fig.add_subplot(111, projection="3d")
for k, (c, m) in enumerate([("C0", "o"), ("C1", "^"), ("C2", "s")]):
    pts = rng.normal(k * 2, 0.6, (50, 3))
    ax.scatter(pts[:, 0], pts[:, 1], pts[:, 2], c=c, marker=m, depthshade=False, label=f"cluster {k}")
ax.legend()
ax.set_box_aspect((1, 1, 0.8))
ax.view_init(20, 35)
print(len(ax.collections))
fig.savefig("clusters.png")
