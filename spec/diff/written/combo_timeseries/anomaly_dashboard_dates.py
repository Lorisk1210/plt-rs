# Anomaly dashboard: series with anomalies, z-score panel with thresholds, and a table of the anomaly dates.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(57)
idx = pd.date_range("2024-01-01", periods=200, freq="D")
s = pd.Series(rng.normal(50, 5, 200), index=idx)
s.iloc[[30, 85, 140, 141]] += [30, -25, 28, 22]
z = (s - s.rolling(21, center=True, min_periods=5).median()) / s.std()
anom = z.abs() > 3
fig = plt.figure(figsize=(11, 7))
gs = fig.add_gridspec(2, 3)
a = fig.add_subplot(gs[0, :])
a.plot(s.index, s, lw=0.8)
a.scatter(s.index[anom], s[anom], color="red", zorder=3)
b = fig.add_subplot(gs[1, :2], sharex=a)
b.plot(z.index, z, lw=0.8)
b.axhline(3, color="r", ls="--"); b.axhline(-3, color="r", ls="--")
c = fig.add_subplot(gs[1, 2])
c.axis("off")
c.table(cellText=[[d.strftime("%Y-%m-%d"), f"{v:.1f}"] for d, v in s[anom].items()], colLabels=["date", "value"], loc="center")
fig.autofmt_xdate()
print(int(anom.sum()), [d.strftime("%m-%d") for d in s.index[anom]])
fig.savefig("anomalies.png", dpi=50)
