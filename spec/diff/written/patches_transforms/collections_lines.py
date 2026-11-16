# Thousands of line segments with LineCollection coloured by a value, plus a PatchCollection.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection, PatchCollection
from matplotlib.patches import Circle

x = np.linspace(0, 4 * np.pi, 500)
y = np.sin(x)
points = np.array([x, y]).T.reshape(-1, 1, 2)
segments = np.concatenate([points[:-1], points[1:]], axis=1)
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 3.5))
lc = LineCollection(segments, cmap="plasma", linewidths=2)
lc.set_array(np.cos(x[:-1]))
a.add_collection(lc)
a.autoscale()
fig.colorbar(lc, ax=a)
rng = np.random.default_rng(2)
circles = [Circle((rng.uniform(0, 10), rng.uniform(0, 10)), rng.uniform(0.2, 1)) for _ in range(30)]
pc = PatchCollection(circles, cmap="Greens", alpha=0.6, edgecolor="k")
pc.set_array(np.arange(30))
b.add_collection(pc)
b.set_xlim(0, 10)
b.set_ylim(0, 10)
b.set_aspect("equal")
print(len(lc.get_segments()), len(pc.get_paths()))
fig.savefig("collections.png")
