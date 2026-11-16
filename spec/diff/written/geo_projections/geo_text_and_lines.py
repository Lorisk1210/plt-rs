# Flight routes between airports drawn as straight lines in lon/lat on a Hammer map, with labels.
import numpy as np
import matplotlib.pyplot as plt

airports = {"FRA": (8.6, 50.0), "JFK": (-73.8, 40.6), "SIN": (103.9, 1.4), "GRU": (-46.5, -23.4), "NRT": (140.4, 35.8)}
routes = [("FRA", "JFK"), ("FRA", "SIN"), ("SIN", "NRT"), ("JFK", "GRU")]
fig = plt.figure(figsize=(8, 4.5))
ax = fig.add_subplot(projection="hammer")
for a, b in routes:
    (x1, y1), (x2, y2) = airports[a], airports[b]
    ax.plot(np.deg2rad([x1, x2]), np.deg2rad([y1, y2]), color="tab:blue", lw=1.5)
for code, (x, y) in airports.items():
    ax.plot(np.deg2rad(x), np.deg2rad(y), "ko", ms=4)
    ax.annotate(code, (np.deg2rad(x), np.deg2rad(y)), xytext=(3, 3), textcoords="offset points", fontsize=8)
ax.grid(True, ls=":")
fig.savefig("routes.png")
print(len(ax.texts))
