# Several semi-transparent fill_between layers stacked with explicit zorder for a fan chart.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(24)
center = 100 + 2 * x
fig, ax = plt.subplots(figsize=(7, 4))
for k, (width, alpha) in enumerate([(30, 0.15), (20, 0.25), (10, 0.4)]):
    spread = width * np.sqrt(x / 23)
    ax.fill_between(x, center - spread, center + spread, color="C0", alpha=alpha, lw=0, zorder=k + 1)
ax.plot(x, center, color="C0", lw=2, zorder=5)
ax.set_title("fan chart")
print([round(c.get_alpha(), 2) for c in ax.collections])
fig.savefig("fan.png")
