# Bars, scatter and fill_between share the axes property cycle; check which colours they take.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
ax.bar([0, 1, 2], [3, 2, 4], label="bars")
ax.scatter([0, 1, 2], [1, 2.5, 1.5], label="scatter")
ax.fill_between([0, 1, 2], [0.2, 0.4, 0.3], label="area")
ax.plot([0, 2], [4, 4], label="line")
ax.legend()
print([p.get_facecolor() for p in ax.patches][:1])
print(ax.collections[0].get_facecolor().round(4), ax.collections[1].get_facecolor().round(4))
print(ax.lines[0].get_color())
fig.savefig("shared_cycle.png")
