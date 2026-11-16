# Car sharing: trips per hour heat by weekday, trip distance vs duration hexbin, and fleet utilisation gauge-like bars.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(236)
heat = rng.poisson(5, (7, 24)) + (np.exp(-((np.arange(24) - 8) / 2) ** 2) + np.exp(-((np.arange(24) - 18) / 2) ** 2)) * 20
dist = rng.lognormal(1.5, 0.6, 3000); dur = dist * rng.uniform(2, 5, 3000) + rng.normal(0, 3, 3000)
fig, axs = plt.subplots(1, 3, figsize=(15, 4.5))
im = axs[0].imshow(heat, aspect="auto", cmap="YlOrRd")
axs[0].set_yticks(range(7), list("MTWTFSS")); fig.colorbar(im, ax=axs[0])
hb = axs[1].hexbin(dist, dur, gridsize=30, bins="log", cmap="viridis", extent=(0, 20, 0, 80))
fig.colorbar(hb, ax=axs[1])
util = rng.uniform(0.3, 0.95, 6)
axs[2].barh([f"zone {i}" for i in range(6)], util, color=plt.cm.RdYlGn(util))
axs[2].set_xlim(0, 1)
axs[2].xaxis.set_major_formatter(plt.matplotlib.ticker.PercentFormatter(1))
fig.tight_layout()
print(int(heat.sum()), hb.get_array().shape, np.round(util, 3).tolist())
fig.savefig("car_sharing.png", dpi=50)
