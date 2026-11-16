# Histograms per category with a common bin edges array and a common y limit, titled with n.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(4)
cats = {name: rng.normal(mu, sd, n) for name, mu, sd, n in
        [("alpha", 0, 1, 300), ("beta", 1, 0.5, 120), ("gamma", -1, 1.5, 200), ("delta", 2, 1, 80)]}
bins = np.linspace(-6, 6, 37)
fig, axs = plt.subplots(2, 2, figsize=(8, 5), sharex=True, sharey=True)
for ax, (name, v) in zip(axs.flat, cats.items()):
    ax.hist(v, bins=bins, color="teal", alpha=0.8)
    ax.set_title(f"{name} (n={v.size})", fontsize=9)
for ax in axs[1]:
    ax.set_xlabel("value")
print(axs[0, 0].get_ylim())
fig.savefig("per_category.png")
