# Gaps in a line from NaN values and from a masked array, plus a continuous version.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(30, dtype=float)
y = np.sin(x / 4)
y_nan = y.copy()
y_nan[8:12] = np.nan
y_mask = np.ma.masked_where((x > 18) & (x < 23), y)
fig, ax = plt.subplots(figsize=(7, 3.5))
ax.plot(x, y_nan + 2, "o-", label="NaN gap")
ax.plot(x, y_mask, "s-", label="masked gap")
valid = ~np.isnan(y_nan)
ax.plot(x[valid], y_nan[valid] - 2, "^-", label="NaN dropped")
ax.legend(loc="lower left")
print(int(np.isnan(y_nan).sum()), int(y_mask.mask.sum()))
fig.savefig("gaps.png")
