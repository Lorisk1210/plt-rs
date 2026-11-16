# Resample tick data to OHLC bars with pandas, draw them as high-low lines and open/close ticks plus volume.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(118)
ticks = pd.Series(100 + rng.normal(0, 0.2, 5000).cumsum(), index=pd.date_range("2024-03-01 09:00", periods=5000, freq="5s"))
ohlc = ticks.resample("15min").ohlc()
vol = ticks.resample("15min").count()
fig, (a, b) = plt.subplots(2, 1, figsize=(9, 5), sharex=True, gridspec_kw={"height_ratios": [3, 1]})
x = np.arange(len(ohlc))
a.vlines(x, ohlc["low"], ohlc["high"], color="k")
a.hlines(ohlc["open"], x - 0.3, x, color="k")
a.hlines(ohlc["close"], x, x + 0.3, color="k")
b.bar(x, vol, color="0.6")
b.set_xticks(x[::4], [t.strftime("%H:%M") for t in ohlc.index[::4]])
print(len(ohlc), ohlc.round(3).iloc[0].tolist())
fig.savefig("ohlc.png", dpi=50)
