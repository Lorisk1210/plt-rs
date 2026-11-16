# Price with moving averages on top and volume bars below sharing the date axis (height ratios 3:1).
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(4)
idx = pd.bdate_range("2024-01-01", periods=120)
price = pd.Series(50 + rng.normal(0, 0.8, 120).cumsum(), index=idx)
volume = pd.Series(rng.integers(1000, 5000, 120), index=idx)
fig, (top, bottom) = plt.subplots(2, 1, sharex=True, figsize=(9, 5), gridspec_kw={"height_ratios": [3, 1]})
top.plot(price, color="k", lw=1, label="close")
top.plot(price.rolling(10).mean(), label="MA10")
top.plot(price.rolling(30).mean(), label="MA30")
top.legend(loc="upper left")
colors = np.where(price.diff().fillna(0) >= 0, "seagreen", "firebrick")
bottom.bar(volume.index, volume, color=colors, width=1)
bottom.set_ylabel("volume")
fig.subplots_adjust(hspace=0.05)
fig.autofmt_xdate()
print(round(float(price.max()), 4), int(volume.sum()))
fig.savefig("price_volume.png")
