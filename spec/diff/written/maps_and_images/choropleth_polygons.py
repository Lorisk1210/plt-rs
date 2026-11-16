# Choropleth of made-up square "districts" with a PolyCollection coloured by a value.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection

rng = np.random.default_rng(3)
polys, vals = [], []
for i in range(6):
    for j in range(5):
        jitter = rng.uniform(-0.15, 0.15, (4, 2))
        polys.append(np.array([[i, j], [i + 1, j], [i + 1, j + 1], [i, j + 1]]) + jitter)
        vals.append(np.hypot(i - 2.5, j - 2) + rng.uniform(0, 1))
fig, ax = plt.subplots(figsize=(6, 5))
pc = PolyCollection(polys, array=np.array(vals), cmap="YlOrRd", edgecolors="k", linewidths=0.5)
ax.add_collection(pc)
ax.autoscale()
ax.set_aspect("equal")
fig.colorbar(pc, ax=ax, label="index")
print(len(polys), round(float(max(vals)), 4))
fig.savefig("districts.png")
