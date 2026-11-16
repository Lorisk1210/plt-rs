# Histogram with rug, mean/median lines and a shaded interquartile range.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(8)
x = rng.gamma(3, 1.5, 300)
q1, med, q3 = np.percentile(x, [25, 50, 75])
fig, ax = plt.subplots(figsize=(6.5, 4))
ax.hist(x, bins=30, color="lightsteelblue", edgecolor="w")
ax.axvspan(q1, q3, color="orange", alpha=0.15, label="IQR")
ax.axvline(x.mean(), color="k", ls="--", label=f"mean {x.mean():.2f}")
ax.axvline(med, color="r", label=f"median {med:.2f}")
ax.plot(x, np.zeros_like(x) - 0.5, "|", color="0.3", ms=8)
ax.legend()
print(round(float(q1), 4), round(float(q3), 4))
fig.savefig("rug.png")
