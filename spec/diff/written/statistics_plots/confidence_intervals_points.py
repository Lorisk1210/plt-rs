# Point estimates with 95% intervals from bootstrap percentiles (numpy only).
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(6)
groups = {g: rng.lognormal(m, 0.5, 40) for g, m in [("A", 1.0), ("B", 1.2), ("C", 0.8), ("D", 1.4)]}
fig, ax = plt.subplots(figsize=(5.5, 3.5))
for i, (g, v) in enumerate(groups.items()):
    boots = np.array([np.median(rng.choice(v, v.size)) for _ in range(500)])
    lo, hi = np.percentile(boots, [2.5, 97.5])
    med = np.median(v)
    ax.errorbar(i, med, yerr=[[med - lo], [hi - med]], fmt="o", capsize=5, color="k")
    print(g, round(float(med), 4), round(float(lo), 4), round(float(hi), 4))
ax.set_xticks(range(4), list(groups))
ax.set_ylabel("median (95% bootstrap CI)")
fig.savefig("bootstrap.png")
