# Grouped box plots: two treatments per time point placed with positions and a legend from boxes.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(6)
times = ["day 1", "day 7", "day 14"]
ctrl = [rng.normal(10, 2, 30) for _ in times]
treat = [rng.normal(10 + 2 * k, 2, 30) for k in range(3)]
fig, ax = plt.subplots(figsize=(6.5, 4))
pos = np.arange(len(times)) * 3
b1 = ax.boxplot(ctrl, positions=pos - 0.5, widths=0.8, patch_artist=True, boxprops=dict(facecolor="lightblue"))
b2 = ax.boxplot(treat, positions=pos + 0.5, widths=0.8, patch_artist=True, boxprops=dict(facecolor="salmon"))
ax.set_xticks(pos, times)
ax.legend([b1["boxes"][0], b2["boxes"][0]], ["control", "treatment"], loc="upper left")
ax.set_xlim(-1.5, pos[-1] + 1.5)
fig.savefig("grouped_boxes.png")
print(ax.get_xticks())
