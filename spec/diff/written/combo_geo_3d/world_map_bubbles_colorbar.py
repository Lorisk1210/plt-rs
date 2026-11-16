# Mollweide world map with bubbles sized by population, coloured by growth, a colour bar and labels for the largest.
import numpy as np
import matplotlib.pyplot as plt

cities = {"Tokyo": (139.7, 35.7, 37, 0.1), "Delhi": (77.2, 28.6, 32, 2.8), "Shanghai": (121.5, 31.2, 28, 1.9),
          "São Paulo": (-46.6, -23.5, 22, 0.8), "Mexico City": (-99.1, 19.4, 22, 0.9), "Cairo": (31.2, 30.0, 21, 2.0),
          "Lagos": (3.4, 6.5, 15, 3.5), "New York": (-74.0, 40.7, 19, 0.2), "London": (-0.1, 51.5, 9, 1.0)}
fig = plt.figure(figsize=(10, 5.5))
ax = fig.add_subplot(projection="mollweide")
lon = np.deg2rad([v[0] for v in cities.values()])
lat = np.deg2rad([v[1] for v in cities.values()])
pop = np.array([v[2] for v in cities.values()])
growth = np.array([v[3] for v in cities.values()])
sc = ax.scatter(lon, lat, s=pop * 15, c=growth, cmap="RdYlGn_r", alpha=0.7, edgecolors="k")
for (name, v), x, y in zip(cities.items(), lon, lat):
    if v[2] > 25:
        ax.annotate(name, (x, y), xytext=(5, 5), textcoords="offset points", fontsize=8)
ax.grid(True, alpha=0.4)
cb = fig.colorbar(sc, ax=ax, orientation="horizontal", shrink=0.5, pad=0.06, label="growth %/yr")
print(len(ax.texts), round(cb.vmax, 3), sc.get_sizes()[:3].tolist())
fig.savefig("megacities.png", dpi=50)
