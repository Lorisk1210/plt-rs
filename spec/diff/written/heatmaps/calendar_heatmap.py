# GitHub-style calendar heat map of daily activity: weeks as columns, weekdays as rows.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

days = pd.date_range("2024-01-01", "2024-12-31", freq="D")
rng = np.random.default_rng(3)
counts = pd.Series(rng.poisson(2, len(days)) * (rng.random(len(days)) > 0.3), index=days)
week = ((days - days[0]).days + days[0].weekday()) // 7
grid = np.full((7, week.max() + 1), np.nan)
grid[days.weekday, week] = counts.values
fig, ax = plt.subplots(figsize=(11, 2.4))
ax.pcolormesh(grid, cmap="Greens", edgecolors="white", linewidth=1.5)
ax.set_yticks(np.arange(7) + 0.5, ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"], fontsize=7)
month_starts = [((pd.Timestamp(2024, m, 1) - days[0]).days + days[0].weekday()) // 7 for m in range(1, 13)]
ax.set_xticks(np.array(month_starts) + 0.5, ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"], fontsize=7)
ax.invert_yaxis()
ax.set_aspect("equal")
ax.tick_params(length=0)
print(int(counts.sum()), grid.shape)
fig.savefig("calendar.png", bbox_inches="tight")
