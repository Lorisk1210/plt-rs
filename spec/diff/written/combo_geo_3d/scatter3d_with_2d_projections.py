# 3-D scatter of clusters plus three 2-D projections (xy, xz, yz) in a 2x2 grid with shared colours.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(60)
pts = np.vstack([rng.normal(c, 0.5, (60, 3)) for c in [(0, 0, 0), (2, 2, 0), (0, 2, 2)]])
lab = np.repeat([0, 1, 2], 60)
fig = plt.figure(figsize=(9, 8))
ax = fig.add_subplot(2, 2, 1, projection="3d")
sc = ax.scatter(*pts.T, c=lab, cmap="tab10", vmin=0, vmax=9, s=8)
for k, (i, j, name) in enumerate([(0, 1, "xy"), (0, 2, "xz"), (1, 2, "yz")], start=2):
    a = fig.add_subplot(2, 2, k)
    a.scatter(pts[:, i], pts[:, j], c=lab, cmap="tab10", vmin=0, vmax=9, s=6)
    a.set_title(name)
    a.set_aspect("equal")
fig.tight_layout()
print(sc.get_array()[:3].tolist(), np.round(pts.mean(axis=0), 4).tolist())
fig.savefig("projections3d.png", dpi=50)
