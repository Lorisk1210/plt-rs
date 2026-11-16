# Climate figure: warming stripes, an anomaly bar chart coloured by sign, and a smoothed trend with uncertainty band.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import TwoSlopeNorm

rng = np.random.default_rng(32)
years = np.arange(1900, 2024)
anom = 0.008 * (years - 1900) - 0.3 + 0.0001 * (years - 1960).clip(0) ** 2 + rng.normal(0, 0.12, years.size)
fig, axs = plt.subplots(3, 1, figsize=(9, 8), gridspec_kw={"height_ratios": [1, 2, 2]})
norm = TwoSlopeNorm(vmin=anom.min(), vcenter=0, vmax=anom.max())
axs[0].imshow(anom[None, :], aspect="auto", cmap="RdBu_r", norm=norm, extent=(years[0], years[-1] + 1, 0, 1))
axs[0].set_yticks([])
axs[1].bar(years, anom, color=np.where(anom > 0, "firebrick", "steelblue"), width=1)
axs[1].axhline(0, color="k", lw=0.5)
smooth = np.convolve(anom, np.ones(11) / 11, mode="same")
axs[2].plot(years, anom, ".", color="0.6", ms=3)
axs[2].plot(years[5:-5], smooth[5:-5], color="k", lw=2)
axs[2].fill_between(years[5:-5], smooth[5:-5] - 0.1, smooth[5:-5] + 0.1, color="k", alpha=0.15)
axs[2].set_ylabel("°C vs 1951-1980")
fig.tight_layout()
print(round(float(anom[-10:].mean()), 4), round(norm.vmin, 4), round(norm.vmax, 4))
fig.savefig("climate.png", dpi=50)
