# DataFrame.plot() of several columns over a date index with title, ylabel and grid.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

idx = pd.date_range("2023-01-01", periods=24, freq="MS")
df = pd.DataFrame({"product A": 100 + np.arange(24) * 3, "product B": 80 + 15 * np.sin(np.arange(24) / 3),
                   "product C": 60 + np.arange(24) ** 1.2}, index=idx)
ax = df.plot(figsize=(8, 4), title="Monthly units", ylabel="units", grid=True, lw=2)
ax.legend(loc="upper left")
print(type(ax).__name__, [l.get_label() for l in ax.get_lines()])
plt.savefig("units.png")
current = plt.gcf()
print(len(current.axes), current.get_size_inches())
current.savefig("df_plot_line_small.png", dpi=50)
