# Take the counts and bin edges returned by hist, reuse them for a bar chart in another axes and a percentile text.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(190)
data = rng.gamma(2, 2, 500)
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4))
counts, edges, patches = a.hist(data, bins=12, color="C0")
widths = np.diff(edges)
b.bar(edges[:-1], counts / counts.sum(), width=widths, align="edge", color=[p.get_facecolor() for p in patches], edgecolor="k")
cum = np.cumsum(counts) / counts.sum()
k = np.searchsorted(cum, 0.9)
b.axvline(edges[k + 1], color="r")
b.text(edges[k + 1], 0.2, f"90% below {edges[k + 1]:.2f}", rotation=90, va="bottom")
print(counts.astype(int).tolist(), round(float(edges[k + 1]), 4), type(counts).__name__, patches[0].get_width() == widths[0])
fig.savefig("hist_reuse.png", dpi=50)
