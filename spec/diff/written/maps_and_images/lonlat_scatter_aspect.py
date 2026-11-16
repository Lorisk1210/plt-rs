# Station map without cartopy: lon/lat scatter with aspect corrected by cos(latitude).
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(2)
lon = rng.uniform(5, 15, 60)
lat = rng.uniform(47, 55, 60)
temp = 15 - (lat - 47) * 0.8 + rng.normal(0, 0.5, 60)
fig, ax = plt.subplots(figsize=(5, 6))
sc = ax.scatter(lon, lat, c=temp, cmap="coolwarm", s=30, edgecolors="k", linewidths=0.3)
ax.set_aspect(1 / np.cos(np.deg2rad(51)))
ax.set_xlabel("longitude [°E]")
ax.set_ylabel("latitude [°N]")
fig.colorbar(sc, ax=ax, shrink=0.6, label="°C")
ax.grid(alpha=0.3)
print(round(ax.get_aspect(), 5))
fig.savefig("stations.png")
