# Daily website visits for a quarter from a pandas Series with a DatetimeIndex, plotted with plt.plot.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

idx = pd.date_range("2024-01-01", "2024-03-31", freq="D")
visits = pd.Series(500 + 80 * np.sin(np.arange(len(idx)) * 2 * np.pi / 7) + np.arange(len(idx)) * 2, index=idx)
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(visits.index, visits.values, lw=1)
ax.plot(visits.rolling(7).mean(), lw=2, label="7-day mean")
ax.set_title("Visits Q1 2024")
ax.legend()
fig.autofmt_xdate()
fig.canvas.draw()
print([t.get_text() for t in ax.get_xticklabels()])
fig.savefig("visits.png")
