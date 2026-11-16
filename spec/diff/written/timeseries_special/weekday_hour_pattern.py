# Average value per hour for each weekday from an hourly series, one line per day.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
idx = pd.date_range("2024-03-04", periods=24 * 28, freq="h")
load = pd.Series(50 + 20 * np.sin((idx.hour - 7) / 24 * 2 * np.pi) + (idx.weekday >= 5) * -10 + rng.normal(0, 3, len(idx)), index=idx)
profile = load.groupby([load.index.weekday, load.index.hour]).mean().unstack(0)
fig, ax = plt.subplots(figsize=(7, 4))
for day, name in enumerate(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]):
    ax.plot(profile.index, profile[day], label=name, lw=2 if day >= 5 else 1)
ax.set_xticks(range(0, 24, 3))
ax.set_xlabel("hour of day")
ax.legend(ncols=4, fontsize=8)
print(profile.shape)
fig.savefig("weekday_profile.png")
