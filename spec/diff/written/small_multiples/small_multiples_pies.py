# Grid of donut charts, one per region, same colours and one shared legend.
import matplotlib.pyplot as plt

regions = {"North": [40, 35, 25], "South": [30, 30, 40], "East": [50, 20, 30], "West": [20, 50, 30],
           "Central": [33, 33, 34], "Islands": [60, 25, 15]}
labels = ["A", "B", "C"]
colors = ["#66c2a5", "#fc8d62", "#8da0cb"]
fig, axs = plt.subplots(2, 3, figsize=(8, 5.5))
for ax, (name, vals) in zip(axs.flat, regions.items()):
    ax.pie(vals, colors=colors, wedgeprops=dict(width=0.45), startangle=90)
    ax.set_title(name)
fig.legend(labels, loc="lower center", ncols=3, title="segment")
fig.subplots_adjust(bottom=0.12)
print(len(fig.axes))
fig.savefig("donuts.png")
