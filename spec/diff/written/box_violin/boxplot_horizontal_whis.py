# Horizontal box plots with whiskers at the 5th and 95th percentiles and no outliers shown.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(17)
data = [np.random.normal(0, s, 200) for s in (0.5, 1, 1.5, 2)]
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.boxplot(data, orientation="horizontal", whis=(5, 95), showfliers=False, widths=0.5)
ax.set_yticks([1, 2, 3, 4], ["σ=0.5", "σ=1", "σ=1.5", "σ=2"])
ax.set_xlabel("value")
ax.axvline(0, color="gray", lw=0.5)
fig.savefig("spread.png")
print(ax.get_ylim())
ax.set_title("Spread by sigma")
