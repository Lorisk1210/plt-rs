# Read violin bodies' path extents (bounding boxes, not mutating) to place labels above each violin.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(250)
data = [rng.normal(m, s, 100) for m, s in [(0, 1), (1, 0.5), (2, 1.5)]]
fig, ax = plt.subplots(figsize=(6, 4))
parts = ax.violinplot(data, showmedians=True)
tops = []
for k, body in enumerate(parts["bodies"]):
    bb = body.get_paths()[0].get_extents()
    ax.text((bb.x0 + bb.x1) / 2, bb.y1 + 0.1, f"n={len(data[k])}", ha="center")
    tops.append(round(float(bb.y1), 4))
ax.set_ylim(top=max(tops) + 0.6)
print(tops, [round(float(v), 4) for v in parts["cmedians"].get_segments()[0][:, 1]])
fig.savefig("violin_labels.png", dpi=50)
