# Raw sensor data with gaps, hourly and daily resamples, a rolling band, and a min/max range on a second panel.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(51)
idx = pd.date_range("2024-04-01", periods=7 * 24 * 12, freq="5min")
raw = pd.Series(20 + 5 * np.sin(2 * np.pi * idx.hour.to_numpy() / 24) + rng.normal(0, 0.8, len(idx)), index=idx)
raw.iloc[500:700] = np.nan
hourly = raw.resample("h").mean()
daily = raw.resample("D").agg(["min", "mean", "max"])
fig, (a, b) = plt.subplots(2, 1, figsize=(10, 6), sharex=True)
a.plot(raw.index, raw, lw=0.3, color="0.7", label="5 min")
a.plot(hourly.index, hourly, color="C0", label="hourly")
roll = raw.rolling("6h").mean()
a.plot(roll.index, roll, color="C1", label="6h rolling")
a.legend(loc="upper right", ncols=3, fontsize=8)
b.fill_between(daily.index, daily["min"], daily["max"], step="post", alpha=0.3)
b.step(daily.index, daily["mean"], where="post", color="k")
fig.autofmt_xdate()
print(int(raw.isna().sum()), daily.round(2).iloc[0].tolist(), len(hourly))
fig.savefig("sensor.png", dpi=50)
