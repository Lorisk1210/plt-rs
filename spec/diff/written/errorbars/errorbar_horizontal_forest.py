# Forest plot: odds ratios with horizontal confidence intervals on a log x axis.
import numpy as np
import matplotlib.pyplot as plt

studies = ["Smith 2010", "Lee 2012", "Gomez 2015", "Novak 2018", "Pooled"]
odds = np.array([0.8, 1.2, 0.65, 0.9, 0.86])
lo = np.array([0.55, 0.8, 0.4, 0.7, 0.74])
hi = np.array([1.15, 1.8, 1.05, 1.15, 0.99])
y = np.arange(len(studies))[::-1]
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.errorbar(odds, y, xerr=[odds - lo, hi - odds], fmt="s", color="k", capsize=3)
ax.plot(odds[-1], y[-1], "D", color="red", ms=9)
ax.axvline(1, ls="--", color="gray")
ax.set_xscale("log")
ax.set_yticks(y, studies)
ax.set_xlabel("odds ratio")
print(ax.get_xlim())
fig.savefig("forest.png")
