# fill_between where the data has NaN gaps: the fill breaks at the gaps.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(20, dtype=float)
y = np.sin(x / 3) + 1.5
y[6:9] = np.nan
fig, ax = plt.subplots(figsize=(6, 3))
ax.fill_between(x, 0, y, alpha=0.5)
ax.plot(x, y, "k.-")
ax.set_title("gap in fill")
paths = ax.collections[0].get_paths()
print(len(paths))
fig.savefig("nan_fill.png")
