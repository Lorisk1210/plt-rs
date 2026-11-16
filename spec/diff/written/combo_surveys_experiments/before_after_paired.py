# Paired before/after measurements: slope lines per subject coloured by direction, difference histogram, and summary text.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(76)
before = rng.normal(140, 12, 25)
after = before - rng.normal(8, 6, 25)
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 4.5))
for x0, x1 in zip(before, after):
    a.plot([0, 1], [x0, x1], "o-", color="C0" if x1 < x0 else "C3", alpha=0.6)
a.set_xticks([0, 1], ["before", "after"])
a.set_xlim(-0.3, 1.3)
d = after - before
b.hist(d, bins=10, color="0.6")
b.axvline(d.mean(), color="k", ls="--")
b.text(0.02, 0.95, f"mean change {d.mean():.1f}\nimproved {np.mean(d < 0):.0%}", transform=b.transAxes, va="top")
fig.tight_layout()
print(round(float(d.mean()), 3), int((d < 0).sum()), len(a.lines))
fig.savefig("paired.png", dpi=50)
