# Strip (jitter) plot with mean and 95% CI bars drawn by hand, as an alternative to seaborn.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(30)
labels = ["wt", "ko1", "ko2"]
groups = [rng.normal(m, 0.6, 25) for m in (1.0, 1.6, 0.7)]
fig, ax = plt.subplots(figsize=(4.5, 4))
for i, g in enumerate(groups):
    xs = i + rng.uniform(-0.15, 0.15, g.size)
    ax.plot(xs, g, "o", ms=4, alpha=0.6, color=f"C{i}")
    ci = 1.96 * g.std(ddof=1) / np.sqrt(g.size)
    ax.errorbar(i + 0.3, g.mean(), yerr=ci, fmt="s", color="k", capsize=4)
ax.set_xticks(range(3), labels)
ax.set_ylabel("expression")
ax.set_xlim(-0.5, 2.7)
print([round(float(g.mean()), 4) for g in groups])
fig.savefig("strip.png")
