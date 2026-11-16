# Weekly totals per category from a daily long table: groupby + resample, then lines.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(13)
dates = pd.date_range("2024-01-01", periods=90, freq="D")
df = pd.DataFrame({"date": np.tile(dates, 2), "cat": np.repeat(["food", "travel"], 90),
                   "spend": np.concatenate([rng.gamma(3, 10, 90), rng.gamma(1, 30, 90)])})
weekly = df.set_index("date").groupby("cat")["spend"].resample("W").sum().unstack(0)
fig, ax = plt.subplots(figsize=(8, 3.5))
weekly.plot(ax=ax, marker=".")
ax.set_ylabel("weekly spend")
print(weekly.shape, round(float(weekly.sum().sum()), 2))
fig.savefig("weekly.png")
