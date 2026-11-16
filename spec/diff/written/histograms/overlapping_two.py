# Two overlapping histograms with transparency and shared bin edges for a before/after comparison.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(31)
before = rng.normal(140, 15, 400)
after = rng.normal(128, 13, 400)
bins = np.linspace(80, 200, 31)
fig, ax = plt.subplots(figsize=(6, 4))
ax.hist(before, bins=bins, alpha=0.5, label="before", color="C3")
ax.hist(after, bins=bins, alpha=0.5, label="after", color="C0")
ax.axvline(before.mean(), color="C3", ls="--")
ax.axvline(after.mean(), color="C0", ls="--")
ax.legend()
ax.set_xlabel("systolic pressure")
print(round(before.mean() - after.mean(), 3))
fig.savefig("before_after.png")
