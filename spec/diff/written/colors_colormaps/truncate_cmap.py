# Truncate a colormap to its middle part (drop the too-light end) and use it for bars.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap


def truncate(cmap, lo=0.2, hi=0.9, n=100):
    return LinearSegmentedColormap.from_list(f"trunc({cmap.name},{lo:.2f},{hi:.2f})", cmap(np.linspace(lo, hi, n)))


blues = plt.get_cmap("Blues")
short = truncate(blues, 0.3, 1.0)
values = np.array([2, 5, 3, 8, 6, 9])
fig, ax = plt.subplots(figsize=(5.5, 3.5))
ax.bar(range(6), values, color=short(values / values.max()))
print(short.name, np.round(short(0.0), 3))
fig.savefig("truncated.png")
