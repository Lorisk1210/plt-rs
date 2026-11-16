# Wrapper functions passing **kwargs through to matplotlib (scatter and hist) with overridable defaults.
import numpy as np
import matplotlib.pyplot as plt


def nice_scatter(ax, x, y, **kwargs):
    opts = dict(s=20, alpha=0.7, edgecolors="white", linewidths=0.5)
    opts.update(kwargs)
    return ax.scatter(x, y, **opts)


def nice_hist(ax, data, **kwargs):
    opts = dict(bins=20, color="slategray", edgecolor="white")
    opts.update(kwargs)
    return ax.hist(data, **opts)


rng = np.random.default_rng(19)
fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
s1 = nice_scatter(axs[0], rng.random(50), rng.random(50))
s2 = nice_scatter(axs[0], rng.random(50), rng.random(50), c="tab:red", s=50, marker="^")
n, bins, patches = nice_hist(axs[1], rng.normal(size=300))
n2, _, _ = nice_hist(axs[2], rng.normal(size=300), bins=np.linspace(-3, 3, 13), histtype="step", color="k", density=True)
print(s1.get_sizes()[0], s2.get_sizes()[0], s1.get_alpha(), s2.get_edgecolor()[0].round(3).tolist())
print(len(patches), round(float(n2.sum() * 0.5), 4))
fig.savefig("passthrough.png", dpi=50)
