# Normal Q-Q plot computed with numpy only (sorted sample vs theoretical quantiles), with reference line.
import numpy as np
import matplotlib.pyplot as plt
from statistics import NormalDist

rng = np.random.default_rng(1)
sample = np.sort(rng.standard_t(5, 150))
n = sample.size
probs = (np.arange(1, n + 1) - 0.5) / n
theory = np.array([NormalDist().inv_cdf(p) for p in probs])
fig, ax = plt.subplots(figsize=(4.5, 4.5))
ax.plot(theory, sample, "o", ms=3)
slope, intercept = np.polyfit(theory, sample, 1)
ax.plot(theory, slope * theory + intercept, "r-")
ax.set_xlabel("theoretical quantiles")
ax.set_ylabel("sample quantiles")
print(round(slope, 4), round(intercept, 4))
fig.savefig("qq.png")
