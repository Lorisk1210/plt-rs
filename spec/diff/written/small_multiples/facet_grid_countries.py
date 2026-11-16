# Small multiples of a metric per country in a 3x4 grid with shared axes and a highlighted reference.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(1)
countries = ["AT", "BE", "CZ", "DK", "EE", "FI", "FR", "DE", "GR", "HU", "IE", "IT"]
years = np.arange(2000, 2024)
data = {c: 50 + np.cumsum(rng.normal(0.5, 1.5, years.size)) for c in countries}
avg = np.mean(list(data.values()), axis=0)
fig, axs = plt.subplots(3, 4, figsize=(10, 6), sharex=True, sharey=True)
for ax, c in zip(axs.flat, countries):
    for other in countries:
        ax.plot(years, data[other], color="0.85", lw=0.6)
    ax.plot(years, data[c], color="C3", lw=1.8)
    ax.plot(years, avg, color="k", lw=0.8, ls="--")
    ax.set_title(c, fontsize=9, loc="left")
fig.supylabel("index")
fig.tight_layout()
print(round(float(avg[-1]), 4))
fig.savefig("countries.png")
