# Flight routes on a Hammer map coloured by passengers, plus a bar chart of route totals using the same colours.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import Normalize

airports = {"FRA": (8.6, 50.0), "JFK": (-73.8, 40.6), "DXB": (55.4, 25.3), "SIN": (103.9, 1.4), "GRU": (-46.5, -23.4)}
routes = {("FRA", "JFK"): 900, ("FRA", "DXB"): 650, ("DXB", "SIN"): 800, ("JFK", "GRU"): 400, ("FRA", "SIN"): 500}
norm = Normalize(min(routes.values()), max(routes.values()))
cmap = plt.get_cmap("plasma")
fig = plt.figure(figsize=(12, 4.5))
ax = fig.add_subplot(1, 2, 1, projection="hammer")
for (a, b), pax in routes.items():
    (x1, y1), (x2, y2) = airports[a], airports[b]
    ax.plot(np.deg2rad([x1, x2]), np.deg2rad([y1, y2]), color=cmap(norm(pax)), lw=1 + pax / 300)
for code, (x, y) in airports.items():
    ax.plot(np.deg2rad(x), np.deg2rad(y), "ko", ms=3)
    ax.text(np.deg2rad(x), np.deg2rad(y) + 0.05, code, fontsize=7, ha="center")
ax.grid(True, alpha=0.3)
bx = fig.add_subplot(1, 2, 2)
names = [f"{a}-{b}" for a, b in routes]
bars = bx.barh(names, list(routes.values()), color=[l.get_color() for l in ax.lines[:len(routes)]])
fig.colorbar(plt.cm.ScalarMappable(norm=norm, cmap=cmap), ax=bx, label="passengers/day")
print([tuple(np.round(b.get_facecolor(), 3)) for b in bars][:2])
fig.savefig("routes.png", dpi=50)
