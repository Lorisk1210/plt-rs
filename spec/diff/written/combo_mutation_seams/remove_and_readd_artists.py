# Remove artists of each kind, re-add new ones, and check the axes' artist lists and the legend after each step.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
l1, = ax.plot([0, 1], label="first")
l2, = ax.plot([1, 0], label="second")
bars = ax.bar([0.2, 0.8], [0.5, 0.7], width=0.1, label="bars")
sc = ax.scatter([0.5], [0.5], label="point")
ax.legend()
print(len(ax.lines), len(ax.patches), len(ax.collections), len(ax.containers))
l1.remove()
bars.remove()
print(len(ax.lines), len(ax.patches), len(ax.containers))
ax.plot([0, 1], [0.5, 0.5], label="third")
leg = ax.legend()
print([t.get_text() for t in leg.get_texts()])
sc.remove()
ax.legend()
print([t.get_text() for t in ax.get_legend().get_texts()], [l.get_label() for l in ax.lines])
fig.savefig("readd.png", dpi=50)
