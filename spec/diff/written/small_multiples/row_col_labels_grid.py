# Row and column headers on a grid of plots (annotate on the outer axes), for parameter sweeps.
import numpy as np
import matplotlib.pyplot as plt

alphas = [0.1, 0.5, 1.0]
betas = [1, 2, 4, 8]
x = np.linspace(0, 5, 100)
fig, axs = plt.subplots(len(alphas), len(betas), figsize=(10, 6), sharex=True, sharey=True)
for i, a in enumerate(alphas):
    for j, b in enumerate(betas):
        axs[i, j].plot(x, np.exp(-a * x) * np.cos(b * x))
for ax, b in zip(axs[0], betas):
    ax.annotate(f"β = {b}", xy=(0.5, 1.05), xycoords="axes fraction", ha="center", va="bottom", fontsize=11)
for ax, a in zip(axs[:, 0], alphas):
    ax.annotate(f"α = {a}", xy=(-0.35, 0.5), xycoords="axes fraction", ha="right", va="center", rotation=90)
fig.subplots_adjust(left=0.12)
print(axs.shape)
fig.savefig("sweep.png")
