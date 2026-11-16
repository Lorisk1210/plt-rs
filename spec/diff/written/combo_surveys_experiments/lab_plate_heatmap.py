# 96-well plate: circles coloured by readout with row/column labels, outliers ringed, and a row-mean bar panel.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Circle

rng = np.random.default_rng(75)
rows, cols = "ABCDEFGH", range(1, 13)
vals = rng.normal(1.0, 0.15, (8, 12)) + np.linspace(0, 0.4, 12)
vals[2, 5] = 2.2; vals[6, 9] = 0.2
z = (vals - vals.mean()) / vals.std()
norm = plt.Normalize(vals.min(), vals.max())
fig, (ax, bar) = plt.subplots(1, 2, figsize=(11, 4.5), gridspec_kw={"width_ratios": [3, 1]})
for r in range(8):
    for c in range(12):
        ax.add_patch(Circle((c, r), 0.4, color=plt.cm.viridis(norm(vals[r, c]))))
        if abs(z[r, c]) > 3:
            ax.add_patch(Circle((c, r), 0.47, fill=False, ec="red", lw=2))
ax.set_xlim(-0.6, 11.6)
ax.set_ylim(7.6, -0.6)
ax.set_xticks(range(12), cols)
ax.set_yticks(range(8), list(rows))
ax.set_aspect("equal")
fig.colorbar(plt.cm.ScalarMappable(norm=norm, cmap="viridis"), ax=ax, shrink=0.8)
bar.barh(list(rows), vals.mean(axis=1))
bar.invert_yaxis()
print(int((np.abs(z) > 3).sum()), round(float(vals.mean()), 4))
fig.savefig("plate.png", dpi=50)
