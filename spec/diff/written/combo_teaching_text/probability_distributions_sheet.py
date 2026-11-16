# Probability cheat sheet: PDFs of four distributions with parameters in titles (mathtext), CDFs below, and a table of moments.
import numpy as np
import matplotlib.pyplot as plt
from math import gamma

x = np.linspace(0.001, 6, 400)
dists = {r"$\mathcal{N}(2, 0.7)$": np.exp(-(x - 2) ** 2 / (2 * 0.49)) / np.sqrt(2 * np.pi * 0.49),
         r"$\mathrm{Exp}(\lambda=1)$": np.exp(-x),
         r"$\Gamma(k=2, \theta=1)$": x * np.exp(-x) / gamma(2),
         r"$\mathrm{LogN}(0, 0.5)$": np.exp(-np.log(x) ** 2 / 0.5) / (x * np.sqrt(0.5 * np.pi))}
fig, axs = plt.subplots(2, 4, figsize=(14, 6), sharex=True)
rows = []
for k, (name, pdf) in enumerate(dists.items()):
    axs[0, k].plot(x, pdf)
    axs[0, k].set_title(name)
    cdf = np.cumsum(pdf) * (x[1] - x[0])
    axs[1, k].plot(x, cdf, color="C1")
    axs[1, k].set_ylim(0, 1.05)
    rows.append([name, f"{np.sum(x * pdf) * (x[1] - x[0]):.2f}"])
fig.tight_layout()
print([r[1] for r in rows], [ax.get_title() for ax in axs[0]][:2])
fig.savefig("distributions.png", dpi=50)
