# Waterfall chart of a profit bridge built from bars with running bottoms and connector lines.
import numpy as np
import matplotlib.pyplot as plt

steps = [("revenue", 120), ("COGS", -45), ("salaries", -30), ("marketing", -12), ("other", 5)]
labels = [s[0] for s in steps] + ["profit"]
values = np.array([s[1] for s in steps])
starts = np.r_[0, np.cumsum(values)[:-1]]
fig, ax = plt.subplots(figsize=(7, 4))
colors = ["steelblue"] + ["seagreen" if v > 0 else "indianred" for v in values[1:]]
ax.bar(range(len(values)), values, bottom=starts, color=colors)
ax.bar(len(values), values.sum(), color="navy")
for i in range(len(values)):
    ax.plot([i - 0.4, i + 1.4], [starts[i] + values[i]] * 2, color="gray", lw=0.8)
ax.set_xticks(range(len(labels)), labels)
for i, v in enumerate(np.r_[values, values.sum()]):
    ax.text(i, (starts[i] + v if i < len(values) else v) + 2, f"{v:+d}" if i < len(values) else str(v), ha="center")
print(values.sum())
fig.savefig("waterfall.png")
