# Per-bar alpha from data, thick edges with joinstyle, and edge colours by sign.
import numpy as np
import matplotlib.pyplot as plt

vals = np.array([5, -3, 8, -1, 4, 6, -7])
alphas = np.abs(vals) / np.abs(vals).max()
fig, ax = plt.subplots(figsize=(6, 3.5))
bars = ax.bar(range(len(vals)), vals, color="C0", linewidth=3, joinstyle="round")
for bar, a, v in zip(bars, alphas, vals):
    bar.set_alpha(0.3 + 0.7 * a)
    bar.set_edgecolor("darkgreen" if v > 0 else "darkred")
ax.axhline(0, color="k", lw=0.5)
print([round(b.get_alpha(), 3) for b in bars])
fig.savefig("alpha_bars.png")
