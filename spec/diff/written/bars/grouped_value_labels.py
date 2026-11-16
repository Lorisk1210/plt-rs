# Monthly revenue per region as grouped bars with value labels, legend below the plot.
import numpy as np
import matplotlib.pyplot as plt

months = ["Jan", "Feb", "Mar", "Apr"]
regions = {"North": [20, 34, 30, 35], "South": [25, 32, 34, 20], "West": [18, 22, 27, 31]}
x = np.arange(len(months))
width = 0.25
fig, ax = plt.subplots(figsize=(7, 4.5), layout="constrained")
for i, (region, vals) in enumerate(regions.items()):
    rects = ax.bar(x + (i - 1) * width, vals, width, label=region)
    ax.bar_label(rects, padding=2, fontsize=8)
ax.set_xticks(x, months)
ax.set_ylabel("revenue (k€)")
ax.set_title("Revenue by region")
ax.set_ylim(0, 42)
ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.08), ncols=3)
fig.savefig("grouped.png")
print(len(ax.containers), [c.get_label() for c in ax.containers])
