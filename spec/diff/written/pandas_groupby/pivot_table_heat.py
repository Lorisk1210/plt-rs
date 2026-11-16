# Pivot a long table to weekday x hour and show it as a heat map with labelled axes.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
df = pd.DataFrame([(d, h, max(0, 20 + 15 * np.sin((h - 6) / 24 * 2 * np.pi) + (10 if d in ("Sat", "Sun") else 0) + rng.normal(0, 3)))
                   for d in days for h in range(24)], columns=["day", "hour", "trips"])
table = df.pivot_table(index="day", columns="hour", values="trips").reindex(days)
fig, ax = plt.subplots(figsize=(9, 3.5))
im = ax.imshow(table.values, aspect="auto", cmap="YlGnBu")
ax.set_yticks(range(7), table.index)
ax.set_xticks(range(0, 24, 3), table.columns[::3])
fig.colorbar(im, ax=ax, label="trips")
print(table.shape, round(float(table.values.max()), 3))
fig.savefig("trips.png")
