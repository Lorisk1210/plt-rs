# Counts on the left axis and percentage of total on a twin axis with matching limits.
import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter

labels = ["A", "B", "C", "D", "E"]
counts = [45, 30, 15, 7, 3]
total = sum(counts)
fig, ax = plt.subplots(figsize=(5.5, 3.5))
ax.bar(labels, counts, color="C0")
ax.set_ylabel("count")
pct = ax.twinx()
pct.set_ylim(0, ax.get_ylim()[1] / total)
pct.yaxis.set_major_formatter(PercentFormatter(1.0))
cum = [sum(counts[: i + 1]) / total for i in range(len(counts))]
pct.plot(labels, cum, "ko-")
print([round(c, 3) for c in cum], round(pct.get_ylim()[1], 4))
fig.savefig("pareto.png")
