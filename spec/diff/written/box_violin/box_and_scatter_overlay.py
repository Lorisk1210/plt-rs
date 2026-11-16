# Box plot with all raw points overlaid and the n per group written under each box.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(44)
groups = {"A": rng.normal(5, 1, 12), "B": rng.normal(6, 1.5, 30), "C": rng.normal(4.5, 0.5, 7)}
fig, ax = plt.subplots(figsize=(5, 4))
ax.boxplot(list(groups.values()), showfliers=False, medianprops=dict(color="k"))
for i, vals in enumerate(groups.values(), start=1):
    ax.scatter(np.full(vals.size, i) + rng.normal(0, 0.04, vals.size), vals, s=10, alpha=0.7, zorder=3)
    ax.text(i, ax.get_ylim()[0], f"n={vals.size}", ha="center", va="bottom", fontsize=8)
ax.set_xticks([1, 2, 3], list(groups))
fig.savefig("box_points.png")
print([len(v) for v in groups.values()])
