# Select scatter points by reading offsets and colours back, then summarise the selection in an inset with a histogram.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(193)
fig, ax = plt.subplots(figsize=(6, 5))
sc = ax.scatter(rng.normal(size=300), rng.normal(size=300), c=rng.random(300), cmap="viridis", s=12)
offs = sc.get_offsets()
vals = sc.get_array()
sel = (offs[:, 0] > 0.5) & (vals > 0.5)
ax.scatter(offs[sel, 0], offs[sel, 1], s=40, facecolors="none", edgecolors="r")
ins = ax.inset_axes([0.62, 0.05, 0.35, 0.3])
ins.hist(vals[sel], bins=10, color="r")
ins.set_title(f"n={sel.sum()}", fontsize=8)
print(int(sel.sum()), type(offs).__name__, type(vals).__name__, round(float(vals[sel].mean()), 4))
fig.savefig("selection.png", dpi=50)
