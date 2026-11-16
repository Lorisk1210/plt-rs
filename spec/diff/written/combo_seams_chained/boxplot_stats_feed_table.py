# Read box plot artists (median/whisker/flier positions) back to fill a summary table next to the plot.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(191)
groups = {"A": rng.normal(5, 1, 50), "B": rng.lognormal(1.5, 0.4, 50), "C": rng.uniform(2, 8, 50)}
fig, (a, t) = plt.subplots(1, 2, figsize=(11, 4), gridspec_kw={"width_ratios": [1.5, 1]})
bp = a.boxplot(list(groups.values()), tick_labels=list(groups))
rows = []
for k, name in enumerate(groups):
    med = bp["medians"][k].get_ydata()[0]
    lo, hi = bp["whiskers"][2 * k].get_ydata()[1], bp["whiskers"][2 * k + 1].get_ydata()[1]
    n_fl = len(bp["fliers"][k].get_ydata())
    rows.append([name, f"{med:.2f}", f"{lo:.2f}", f"{hi:.2f}", str(n_fl)])
t.axis("off")
t.table(cellText=rows, colLabels=["group", "median", "low", "high", "fliers"], loc="center")
print(rows)
fig.savefig("box_table.png", dpi=50)
