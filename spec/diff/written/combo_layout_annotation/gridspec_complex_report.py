# Complex GridSpec report: header text row, big chart, side stats, a footer table, with hspace/wspace tuned.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(122)
fig = plt.figure(figsize=(11, 8))
gs = fig.add_gridspec(4, 3, height_ratios=[0.4, 3, 1, 1], width_ratios=[3, 1, 1], hspace=0.5, wspace=0.3)
header = fig.add_subplot(gs[0, :])
header.axis("off")
header.text(0, 0.5, "Monthly KPI report", fontsize=18, fontweight="bold", va="center")
header.text(1, 0.5, "June 2024", fontsize=12, ha="right", va="center")
main = fig.add_subplot(gs[1, :])
data = rng.normal(0, 1, (30, 3)).cumsum(axis=0)
main.plot(data)
main.legend(["north", "south", "west"], ncols=3)
for k, name in enumerate(["min", "max"]):
    ax = fig.add_subplot(gs[2, 1 + k])
    ax.bar(["n", "s", "w"], getattr(data, name)(axis=0), color=["C0", "C1", "C2"])
    ax.set_title(name)
dist = fig.add_subplot(gs[2, 0])
dist.hist(data.ravel(), bins=20)
foot = fig.add_subplot(gs[3, :])
foot.axis("off")
foot.table(cellText=np.round(data[-3:], 2), colLabels=["north", "south", "west"], rowLabels=["d28", "d29", "d30"], loc="center")
print(np.round(data[-1], 3).tolist())
fig.savefig("report.png", dpi=50)
