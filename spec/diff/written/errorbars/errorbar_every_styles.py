# Dense data with errorevery to thin the bars and styled caps (capthick) and barsabove.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 60)
y = np.cos(x)
fig, ax = plt.subplots(figsize=(7, 3.5))
ax.errorbar(x, y, yerr=0.2, errorevery=5, capsize=3, capthick=2, barsabove=True, lw=1, label="every 5th")
ax.errorbar(x, y + 1.5, yerr=0.15 + 0.05 * x / 10, errorevery=(2, 6), fmt="-", color="C2", alpha=0.7, label="offset")
ax.legend(loc="lower left")
fig.savefig("thinned.png")
print(len(ax.containers))
ax.set_title("Thinned error bars")
ax.set_xlabel("x")
fig.savefig("thinned_titled.png", dpi=60)
