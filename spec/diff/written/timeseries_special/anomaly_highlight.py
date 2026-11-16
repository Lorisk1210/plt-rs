# Highlight anomalies (|z| > 2.5) in a time series with red markers and shaded windows.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
idx = pd.date_range("2024-01-01", periods=300, freq="D")
s = pd.Series(rng.normal(0, 1, 300), index=idx)
s.iloc[[50, 120, 121, 250]] += [6, -5, -4, 7]
z = (s - s.rolling(30, min_periods=5).mean()) / s.rolling(30, min_periods=5).std()
anom = z.abs() > 2.5
fig, ax = plt.subplots(figsize=(9, 3.5))
ax.plot(s.index, s, lw=0.8)
ax.scatter(s.index[anom], s[anom], color="red", zorder=3)
for t in s.index[anom]:
    ax.axvspan(t - pd.Timedelta(days=2), t + pd.Timedelta(days=2), color="red", alpha=0.1)
print(int(anom.sum()))
fig.savefig("anomalies.png")
