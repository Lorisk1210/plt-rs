# Category names on x via string data for scatter with jitter (numeric offset by mapping) plus boxplots at the same positions.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(130)
cats = ["alpha", "beta", "gamma"]
data = {c: rng.normal(i, 1, 40) for i, c in enumerate(cats)}
fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(cats, [d.mean() for d in data.values()], "k_", ms=30, mew=2)
mapping = ax.xaxis.units._mapping
for c, d in data.items():
    ax.scatter(mapping[c] + rng.uniform(-0.15, 0.15, d.size), d, s=8, alpha=0.5)
ax.boxplot(list(data.values()), positions=[mapping[c] for c in cats], widths=0.4, showfliers=False)
ax.set_xticks([mapping[c] for c in cats], cats)
print(dict(mapping), [t.get_text() for t in ax.get_xticklabels()])
fig.savefig("cat_jitter.png", dpi=50)
