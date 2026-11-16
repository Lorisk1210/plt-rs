# Legend entries for fill_between, hist, errorbar, stem and pie wedges together.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
x = np.arange(5)
ax.fill_between(x, 0, x, alpha=0.3, label="area")
ax.hist([0.5, 1.5, 1.5, 2.5], bins=4, alpha=0.5, label="hist")
ax.errorbar(x, x + 1, yerr=0.3, fmt="o", label="errorbar")
ax.stem(x, x + 2, label="stem")
ax.legend(loc="upper left")
print([type(h).__name__ for h in ax.get_legend().legend_handles])
fig.savefig("mixed_legend.png")
