# Read scatter offsets after a transform and connect nearest neighbours with a LineCollection.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection

rng = np.random.default_rng(22)
fig, ax = plt.subplots(figsize=(5, 5))
sc = ax.scatter(rng.random(25), rng.random(25), s=30, zorder=3)
pts = sc.get_offsets()
d = np.hypot(*(pts[:, None, :] - pts[None, :, :]).transpose(2, 0, 1))
np.fill_diagonal(d, np.inf)
nearest = d.argmin(axis=1)
segs = [[pts[i], pts[j]] for i, j in enumerate(nearest)]
lc = LineCollection(segs, colors="0.6", linewidths=1)
ax.add_collection(lc)
disp = sc.get_offset_transform().transform(pts[:2])
print(type(pts).__name__, pts.shape, nearest[:5].tolist(), np.round(disp, 2).tolist())
print(len(lc.get_segments()), lc.get_segments()[0].shape)
fig.savefig("neighbours.png", dpi=50)
