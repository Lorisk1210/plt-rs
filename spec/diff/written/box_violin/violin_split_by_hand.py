# Half violins (split) by clipping each body's vertices to one side, as SO answers show.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(9)
left = [rng.normal(m, 1, 150) for m in (0, 1, 2)]
right = [rng.normal(m + 0.5, 1.3, 150) for m in (0, 1, 2)]
fig, ax = plt.subplots(figsize=(6, 4))
for data, side, color in [(left, -1, "C0"), (right, 1, "C1")]:
    parts = ax.violinplot(data, showextrema=False)
    for body in parts["bodies"]:
        verts = body.get_paths()[0].vertices
        center = np.mean(verts[:, 0])
        if side < 0:
            verts[:, 0] = np.clip(verts[:, 0], -np.inf, center)
        else:
            verts[:, 0] = np.clip(verts[:, 0], center, np.inf)
        body.set_color(color)
ax.set_xticks([1, 2, 3], ["low", "mid", "high"])
fig.savefig("split.png")
print(len(ax.collections))
