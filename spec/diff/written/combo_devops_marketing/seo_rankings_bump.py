# SEO keyword rankings: bump chart over weeks (inverted y, log-ish ranks), sparklines grid per keyword and a change table.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(209)
keywords = ["python plot", "matplotlib legend", "bar chart", "heatmap", "subplots"]
ranks = np.clip(np.round(np.cumsum(rng.normal(0, 1.5, (5, 12)), axis=1) + rng.integers(2, 20, (5, 1))), 1, 30).astype(int)
fig = plt.figure(figsize=(12, 6))
a = fig.add_subplot(1, 2, 1)
for k, kw in enumerate(keywords):
    a.plot(range(1, 13), ranks[k], "o-", label=kw)
a.invert_yaxis(); a.legend(fontsize=7)
for k, kw in enumerate(keywords):
    s = fig.add_subplot(5, 2, 2 * k + 2)
    s.plot(ranks[k], color=a.lines[k].get_color())
    s.invert_yaxis(); s.set_yticks([]); s.set_xticks([])
    s.text(1.02, 0.5, f"{ranks[k, 0] - ranks[k, -1]:+d}", transform=s.transAxes, va="center")
print(ranks[:, -1].tolist(), [l.get_color() for l in a.lines][:2])
fig.savefig("seo.png", dpi=50)
