# Mean response per treatment as bars with standard-error whiskers and capsize.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(2)
groups = ["placebo", "low", "medium", "high"]
samples = [rng.normal(m, 2.5, 20) for m in (10, 12, 15, 17)]
means = [s.mean() for s in samples]
sem = [s.std(ddof=1) / np.sqrt(s.size) for s in samples]
fig, ax = plt.subplots(figsize=(5.5, 4))
bars = ax.bar(groups, means, yerr=sem, capsize=6, color="lightsteelblue", edgecolor="navy",
              error_kw=dict(elinewidth=1.2, ecolor="navy"))
ax.bar_label(bars, labels=[f"{m:.1f}" for m in means], label_type="edge", padding=8)
ax.set_ylabel("response")
ax.set_ylim(0, 22)
print(np.round(means, 3), np.round(sem, 3))
fig.savefig("treatments.png")
