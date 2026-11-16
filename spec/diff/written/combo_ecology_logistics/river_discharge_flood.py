# River gauge: discharge with flood thresholds (axhspans), hydrograph of a flood event inset, and annual peaks bar.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(216)
idx = pd.date_range("2020-01-01", "2023-12-31", freq="D")
q = 50 + 30 * np.sin(idx.dayofyear.to_numpy() / 365 * 2 * np.pi) + rng.gamma(1, 10, len(idx))
q[800:815] += np.linspace(0, 300, 15) * np.exp(-np.linspace(0, 3, 15))
fig, (a, b) = plt.subplots(2, 1, figsize=(10, 7))
a.plot(idx, q, lw=0.6)
for lo, hi, c in [(150, 250, "gold"), (250, 1000, "red")]:
    a.axhspan(lo, hi, color=c, alpha=0.1)
a.set_ylim(0, q.max() * 1.1)
ins = a.inset_axes([0.6, 0.55, 0.35, 0.4])
ins.plot(idx[790:830], q[790:830], "r")
ins.tick_params(labelsize=6)
peaks = pd.Series(q, index=idx).resample("YE").max()
b.bar(peaks.index.year, peaks.values, color=np.where(peaks.values > 250, "C3", "C0"))
fig.autofmt_xdate()
print(peaks.round(2).tolist())
fig.savefig("river.png", dpi=50)
