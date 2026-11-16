# City locations on a Mollweide world projection with a lon/lat grid (no cartopy).
import numpy as np
import matplotlib.pyplot as plt

cities = {"London": (-0.1, 51.5), "Tokyo": (139.7, 35.7), "Sydney": (151.2, -33.9),
          "Lima": (-77.0, -12.0), "Cairo": (31.2, 30.0), "Anchorage": (-149.9, 61.2)}
fig = plt.figure(figsize=(8, 4.5))
ax = fig.add_subplot(projection="mollweide")
for name, (lon, lat) in cities.items():
    ax.plot(np.deg2rad(lon), np.deg2rad(lat), "o", color="crimson")
    ax.text(np.deg2rad(lon) + 0.05, np.deg2rad(lat), name, fontsize=8)
ax.grid(True)
ax.set_title("Cities")
print(ax.name, len(ax.lines))
fig.savefig("cities.png")
