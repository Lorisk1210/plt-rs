# Statistics text box in the corner of a histogram using bbox props and transAxes.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(10)
data = rng.gamma(2.0, 2.0, 1000)
fig, ax = plt.subplots(figsize=(6, 4))
ax.hist(data, bins=40, color="0.6")
stats = "\n".join([f"n = {data.size}", f"mean = {data.mean():.2f}", f"median = {np.median(data):.2f}",
                   f"std = {data.std():.2f}"])
props = dict(boxstyle="round,pad=0.5", facecolor="wheat", alpha=0.8)
ax.text(0.97, 0.95, stats, transform=ax.transAxes, fontsize=10, va="top", ha="right", bbox=props)
ax.text(0.03, 0.95, "sample A", transform=ax.transAxes, va="top",
        bbox=dict(boxstyle="square", fc="white", ec="k"))
print(stats.replace("\n", "; "))
fig.savefig("stats_box.png")
