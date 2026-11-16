# Earthquake-like events (synthetic) on an Aitoff map sized by magnitude and coloured by depth.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(33)
lon = rng.uniform(-180, 180, 120)
lat = np.clip(rng.normal(0, 30, 120), -85, 85)
mag = rng.uniform(4, 8, 120)
depth = rng.uniform(0, 600, 120)
fig = plt.figure(figsize=(9, 5))
ax = fig.add_subplot(projection="aitoff")
sc = ax.scatter(np.deg2rad(lon), np.deg2rad(lat), s=(mag - 3) ** 3, c=depth, cmap="viridis_r", alpha=0.7)
cb = fig.colorbar(sc, ax=ax, orientation="horizontal", shrink=0.6, pad=0.08)
cb.set_label("depth [km]")
ax.grid(True, alpha=0.5)
print(round(float(mag.max()), 3))
fig.savefig("quakes.png")
