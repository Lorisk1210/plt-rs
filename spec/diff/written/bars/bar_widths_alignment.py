# Histogram-like bars with explicit edges: align='edge', variable widths, and tick_label.
import numpy as np
import matplotlib.pyplot as plt

edges = np.array([0, 5, 10, 20, 40, 80])
counts = np.array([12, 18, 25, 9, 3])
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.bar(edges[:-1], counts / np.diff(edges), width=np.diff(edges), align="edge",
       edgecolor="k", color="burlywood")
ax.set_xlabel("income bracket (k)")
ax.set_ylabel("people per k")
fig.savefig("brackets.png")
fig2, ax2 = plt.subplots(figsize=(4, 3))
ax2.bar([1, 2, 3], [4, 5, 6], tick_label=["one", "two", "three"], width=0.5, align="center")
print([t.get_text() for t in ax2.get_xticklabels()])
fig2.savefig("tick_label.png")
