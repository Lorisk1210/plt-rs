# Overlay this year's and last year's daily series aligned by day of year, with a cumulative difference panel.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(58)
idx = pd.date_range("2023-01-01", "2024-12-31", freq="D")
s = pd.Series(100 + 20 * np.sin(idx.dayofyear.to_numpy() / 365 * 2 * np.pi) + (idx.year.to_numpy() - 2023) * 5 + rng.normal(0, 6, len(idx)), index=idx)
last = s["2023"].reset_index(drop=True)
this = s["2024"].reset_index(drop=True)[:365]
fig, (a, b) = plt.subplots(2, 1, figsize=(9, 6), sharex=True)
a.plot(last.rolling(7).mean(), color="0.6", label="2023")
a.plot(this.rolling(7).mean(), color="C0", label="2024")
a.legend()
diff = (this - last[:365]).cumsum()
b.fill_between(diff.index, diff, 0, where=diff >= 0, color="green", alpha=0.3)
b.fill_between(diff.index, diff, 0, where=diff < 0, color="red", alpha=0.3)
b.set_xlabel("day of year")
print(round(float(diff.iloc[-1]), 3), len(this))
fig.savefig("yoy_overlay.png", dpi=50)
