# Continuous glucose: 14 days overlaid by time of day with percentile bands (5-95, 25-75) and the median, target range shaded.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(157)
minutes = np.arange(0, 24 * 60, 15)
meals = np.exp(-((minutes - 480) / 50) ** 2) + np.exp(-((minutes - 780) / 60) ** 2) + np.exp(-((minutes - 1140) / 60) ** 2)
days = np.array([110 + 35 * meals + rng.normal(0, 12, minutes.size) for _ in range(14)])
hours = minutes / 60
fig, ax = plt.subplots(figsize=(9, 4.5))
for d in days:
    ax.plot(hours, d, color="0.8", lw=0.5)
p5, p25, p50, p75, p95 = np.percentile(days, [5, 25, 50, 75, 95], axis=0)
ax.fill_between(hours, p5, p95, color="C0", alpha=0.15, label="5-95%")
ax.fill_between(hours, p25, p75, color="C0", alpha=0.35, label="25-75%")
ax.plot(hours, p50, color="C0", lw=2, label="median")
ax.axhspan(70, 180, color="green", alpha=0.05)
ax.set_xticks(range(0, 25, 3))
ax.legend(loc="upper left")
print(round(float(np.mean((days >= 70) & (days <= 180))), 4), round(float(p50.max()), 3))
fig.savefig("glucose.png", dpi=50)
