# Two series with a rolling correlation panel coloured by sign (fill_between where) and a scatter coloured by time.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(54)
idx = pd.date_range("2022-01-01", periods=500, freq="D")
a = pd.Series(np.cumsum(rng.normal(0, 1, 500)), index=idx)
b = pd.Series(0.5 * a.values * np.sin(np.arange(500) / 80) + np.cumsum(rng.normal(0, 1, 500)), index=idx)
corr = a.diff().rolling(60).corr(b.diff())
fig = plt.figure(figsize=(11, 7))
ax1 = fig.add_subplot(2, 2, (1, 2))
ax1.plot(a, label="A")
ax1.plot(b, label="B")
ax1.legend()
ax2 = fig.add_subplot(2, 2, 3)
ax2.fill_between(corr.index, corr, 0, where=corr > 0, color="green", alpha=0.4, interpolate=True)
ax2.fill_between(corr.index, corr, 0, where=corr < 0, color="red", alpha=0.4, interpolate=True)
ax2.set_ylim(-1, 1)
ax3 = fig.add_subplot(2, 2, 4)
sc = ax3.scatter(a.diff(), b.diff(), c=np.arange(500), cmap="viridis", s=6)
fig.colorbar(sc, ax=ax3, label="day")
fig.autofmt_xdate()
fig.tight_layout()
print(round(float(corr.dropna().mean()), 4), round(float(corr.min()), 4))
fig.savefig("rolling_corr.png", dpi=50)
