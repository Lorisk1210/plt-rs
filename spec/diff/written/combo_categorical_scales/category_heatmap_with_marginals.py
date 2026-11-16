# Category x category heat map (strings on both axes via imshow ticks) with marginal bar totals sharing axes.
import numpy as np
import matplotlib.pyplot as plt

rows = ["mon", "tue", "wed", "thu", "fri"]
cols = ["breakfast", "lunch", "dinner", "snack"]
data = np.array([[3, 8, 12, 2], [4, 9, 11, 3], [2, 7, 13, 4], [5, 10, 9, 2], [6, 12, 15, 6]])
fig = plt.figure(figsize=(8, 6))
gs = fig.add_gridspec(2, 2, width_ratios=[4, 1], height_ratios=[1, 4], hspace=0.05, wspace=0.05)
ax = fig.add_subplot(gs[1, 0])
ax.imshow(data, cmap="Oranges", aspect="auto")
ax.set_xticks(range(4), cols)
ax.set_yticks(range(5), rows)
top = fig.add_subplot(gs[0, 0], sharex=ax)
top.bar(range(4), data.sum(axis=0), color="C1")
top.tick_params(labelbottom=False)
right = fig.add_subplot(gs[1, 1], sharey=ax)
right.barh(range(5), data.sum(axis=1), color="C1")
right.tick_params(labelleft=False)
print(data.sum(axis=0).tolist(), data.sum(axis=1).tolist(), [round(v, 2) for v in top.get_xlim()])
fig.savefig("meals.png", dpi=50)
