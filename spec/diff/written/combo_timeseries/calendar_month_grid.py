# A month calendar figure: each day a cell with the date, coloured by a value, weekends hatched, with a colour bar.
import calendar
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
from matplotlib.colors import Normalize

rng = np.random.default_rng(55)
year, month = 2024, 7
weeks = calendar.Calendar(firstweekday=0).monthdayscalendar(year, month)
values = {d: rng.uniform(0, 10) for w in weeks for d in w if d}
norm = Normalize(0, 10)
cmap = plt.get_cmap("YlOrRd")
fig, ax = plt.subplots(figsize=(7, 5))
for r, week in enumerate(weeks):
    for c, day in enumerate(week):
        if not day:
            continue
        ax.add_patch(Rectangle((c, -r), 1, 1, fc=cmap(norm(values[day])), ec="w", hatch="//" if c >= 5 else None))
        ax.text(c + 0.1, -r + 0.8, str(day), fontsize=9)
ax.set_xlim(0, 7)
ax.set_ylim(-len(weeks) + 1, 1)
ax.set_xticks(np.arange(7) + 0.5, list(calendar.day_abbr))
ax.xaxis.tick_top()
ax.set_yticks([])
fig.colorbar(plt.cm.ScalarMappable(norm=norm, cmap=cmap), ax=ax, label="value")
ax.set_title(calendar.month_name[month] + f" {year}", pad=25)
print(len(weeks), round(sum(values.values()), 3))
fig.savefig("calendar.png", dpi=50)
