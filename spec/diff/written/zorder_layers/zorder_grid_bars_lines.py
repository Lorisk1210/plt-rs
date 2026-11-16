# Control drawing order: grid behind bars, a line above bars, a text above everything.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
ax.bar(range(6), [3, 5, 2, 6, 4, 7], color="lightblue", zorder=2)
ax.plot(range(6), [2, 4, 3, 5, 5, 6], "r-o", zorder=3)
ax.grid(True, zorder=0)
ax.set_axisbelow(True)
ax.text(2.5, 6.5, "on top", fontsize=14, zorder=10, bbox=dict(fc="w"))
ax.axhspan(2.5, 3.5, color="yellow", alpha=0.5, zorder=1)
print(sorted((a.get_zorder(), type(a).__name__) for a in ax.get_children() if hasattr(a, "get_zorder"))[:5])
fig.savefig("layers.png")
