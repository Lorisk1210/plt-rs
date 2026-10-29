"""Grouped bars with value labels: daily screen time by age group and
device."""

import numpy as np

import pltrs.pyplot as plt

groups = ["13–17", "18–24", "25–34", "35–49", "50–64", "65+"]
hours = {
    "phone": [4.8, 4.4, 3.6, 2.9, 2.1, 1.3],
    "computer": [2.1, 3.0, 3.4, 3.1, 2.4, 1.6],
    "television": [1.2, 1.1, 1.5, 1.9, 2.8, 3.9],
}

x = np.arange(len(groups))
width = 0.27
fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
for i, (device, values) in enumerate(hours.items()):
    bars = ax.bar(x + (i - 1) * width, values, width, label=device)
    ax.bar_label(bars, fmt="%.1f", padding=2, fontsize=7.5)
ax.set_xticks(x, groups)
ax.set_title("Daily screen time by age group")
ax.set_xlabel("age")
ax.set_ylabel("hours per day")
ax.set_ylim(0, 5.6)
ax.legend(loc="upper left", ncols=3)

fig.savefig("grouped_bars.png")
