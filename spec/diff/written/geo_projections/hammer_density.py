# Sky density map on a Hammer projection using pcolormesh in radians with a colour bar.
import numpy as np
import matplotlib.pyplot as plt

lon = np.linspace(-np.pi, np.pi, 73)
lat = np.linspace(-np.pi / 2, np.pi / 2, 37)
Lon, Lat = np.meshgrid(lon, lat)
density = np.cos(Lat) ** 2 * (1 + 0.5 * np.cos(2 * Lon))
fig = plt.figure(figsize=(8, 4.5))
ax = fig.add_subplot(111, projection="hammer")
mesh = ax.pcolormesh(Lon, Lat, density[:-1, :-1], cmap="plasma", shading="flat")
fig.colorbar(mesh, orientation="horizontal", pad=0.05, shrink=0.6)
ax.grid(True, color="w", lw=0.5)
print(ax.get_xlim(), ax.get_ylim())
fig.savefig("sky.png")
