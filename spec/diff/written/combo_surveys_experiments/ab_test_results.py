# A/B test: conversion bars with 95% CI, the difference distribution by bootstrap, and a cumulative conversion timeline.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(70)
n = 2000
a = rng.random(n) < 0.10
b = rng.random(n) < 0.12
rates = [a.mean(), b.mean()]
ci = [1.96 * np.sqrt(r * (1 - r) / n) for r in rates]
boot = np.array([rng.choice(b, n).mean() - rng.choice(a, n).mean() for _ in range(1000)])
fig, axs = plt.subplots(1, 3, figsize=(12, 3.8))
bars = axs[0].bar(["A", "B"], rates, yerr=ci, capsize=6, color=["0.6", "C0"])
axs[0].bar_label(bars, labels=[f"{r:.1%}" for r in rates], padding=8)
axs[0].set_ylim(0, 0.16)
axs[1].hist(boot, bins=30, color="C0", alpha=0.7)
lo, hi = np.percentile(boot, [2.5, 97.5])
axs[1].axvspan(lo, hi, color="gold", alpha=0.3)
axs[1].axvline(0, color="k")
axs[2].plot(np.cumsum(a) / np.arange(1, n + 1), label="A")
axs[2].plot(np.cumsum(b) / np.arange(1, n + 1), label="B")
axs[2].legend()
fig.tight_layout()
print([round(float(r), 4) for r in rates], round(float(lo), 4), round(float(hi), 4))
fig.savefig("ab.png", dpi=50)
