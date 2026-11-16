# Weekly totals as bars from daily data, a linear trend line fitted on week numbers, and annotations of best/worst weeks.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(53)
daily = pd.Series(rng.poisson(30, 140) + np.arange(140) // 10, index=pd.date_range("2024-01-01", periods=140, freq="D"))
weekly = daily.resample("W-SUN").sum()
x = np.arange(len(weekly))
coef = np.polyfit(x, weekly.values, 1)
fig, ax = plt.subplots(figsize=(9, 4))
bars = ax.bar(weekly.index, weekly.values, width=5, color="C0", alpha=0.8)
ax.plot(weekly.index, np.polyval(coef, x), "r--", label=f"trend {coef[0]:+.1f}/week")
for i, label in [(int(weekly.values.argmax()), "best"), (int(weekly.values.argmin()), "worst")]:
    ax.annotate(f"{label}: {weekly.values[i]}", (weekly.index[i], weekly.values[i]), xytext=(0, 12),
                textcoords="offset points", ha="center", arrowprops=dict(arrowstyle="-"))
ax.legend()
fig.autofmt_xdate()
print(len(weekly), np.round(coef, 3).tolist(), [t.get_text() for t in ax.texts])
fig.savefig("weekly.png", dpi=50)
