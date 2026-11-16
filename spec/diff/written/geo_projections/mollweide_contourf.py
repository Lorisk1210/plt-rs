# Filled contours of a global temperature-like field on Mollweide with coastline-free grid.
import numpy as np
import matplotlib.pyplot as plt

lon = np.linspace(-np.pi, np.pi, 90)
lat = np.linspace(-np.pi / 2, np.pi / 2, 45)
Lon, Lat = np.meshgrid(lon, lat)
temp = 30 * np.cos(Lat) - 10 + 5 * np.sin(3 * Lon) * np.cos(Lat)
fig = plt.figure(figsize=(8, 4.5))
ax = fig.add_subplot(projection="mollweide")
cf = ax.contourf(Lon, Lat, temp, levels=12, cmap="RdYlBu_r")
fig.colorbar(cf, ax=ax, orientation="horizontal", shrink=0.6, label="°C")
ax.grid(True, color="k", alpha=0.3)
ax.set_title("Toy temperature")
print(len(cf.levels))
fig.savefig("toy_temp.png")
