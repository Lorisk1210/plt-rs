# Horizontal violins with quartile lines and a custom bandwidth, plus points overlaid.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(27)
groups = [rng.normal(m, 1, 80) for m in (3, 5, 4)]
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.violinplot(groups, orientation="horizontal", quantiles=[[0.25, 0.75]] * 3, bw_method=0.3, showmeans=True)
for i, g in enumerate(groups, start=1):
    jitter = rng.uniform(-0.08, 0.08, g.size)
    ax.scatter(g, i + jitter, s=4, color="k", alpha=0.4)
ax.set_yticks([1, 2, 3], ["control", "drug A", "drug B"])
ax.set_xlabel("effect")
fig.savefig("violins_h.png")
print([round(float(np.median(g)), 4) for g in groups])
