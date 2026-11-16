# One scatter per group with its own linear fit and R^2 in the corner, shared colour scale.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(6)
fig, axs = plt.subplots(1, 4, figsize=(12, 3), sharey=True)
for k, ax in enumerate(axs):
    x = rng.uniform(0, 10, 50)
    y = (k + 1) * 0.4 * x + rng.normal(0, 1 + k, 50)
    p = np.polyfit(x, y, 1)
    r2 = np.corrcoef(x, y)[0, 1] ** 2
    ax.scatter(x, y, c=y, cmap="viridis", vmin=-5, vmax=25, s=10)
    ax.plot([0, 10], np.polyval(p, [0, 10]), "r")
    ax.text(0.05, 0.9, f"$R^2$={r2:.2f}", transform=ax.transAxes)
    ax.set_title(f"group {k + 1}")
print(axs[0].get_ylim())
fig.savefig("fits.png")
