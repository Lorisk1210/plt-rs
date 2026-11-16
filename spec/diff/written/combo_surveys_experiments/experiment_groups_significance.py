# Group comparison: box + jittered points per group, significance brackets with stars, and a means table.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(71)
groups = {"ctrl": rng.normal(10, 2, 20), "low": rng.normal(11, 2, 20), "mid": rng.normal(13, 2, 20), "high": rng.normal(15, 2, 20)}
fig, (ax, tab) = plt.subplots(1, 2, figsize=(10, 4.5), gridspec_kw={"width_ratios": [2.5, 1]})
ax.boxplot(groups.values(), tick_labels=groups.keys(), showfliers=False, widths=0.5)
for i, v in enumerate(groups.values(), start=1):
    ax.scatter(np.full(v.size, i) + rng.uniform(-0.12, 0.12, v.size), v, s=10, alpha=0.6, zorder=3)
top = max(v.max() for v in groups.values())
for k, (j, stars) in enumerate([(2, "ns"), (3, "**"), (4, "***")]):
    y = top + 1 + k * 1.2
    ax.plot([1, 1, j, j], [y, y + 0.3, y + 0.3, y], "k", lw=1)
    ax.text((1 + j) / 2, y + 0.35, stars, ha="center")
ax.set_ylim(top - 12, top + 5)
tab.axis("off")
tab.table(cellText=[[k, f"{v.mean():.2f}", f"{v.std(ddof=1):.2f}"] for k, v in groups.items()], colLabels=["group", "mean", "sd"], loc="center")
print({k: round(float(v.mean()), 3) for k, v in groups.items()})
fig.savefig("groups.png", dpi=50)
