# Candlestick chart drawn by hand with vlines and bars from an OHLC DataFrame.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(2)
days = pd.bdate_range("2024-03-01", periods=30)
close = 100 + rng.normal(0, 1.5, 30).cumsum()
open_ = np.r_[100, close[:-1]] + rng.normal(0, 0.5, 30)
high = np.maximum(open_, close) + rng.uniform(0.2, 1.5, 30)
low = np.minimum(open_, close) - rng.uniform(0.2, 1.5, 30)
df = pd.DataFrame({"open": open_, "high": high, "low": low, "close": close}, index=days)
up = df.close >= df.open
x = np.arange(len(df))
fig, ax = plt.subplots(figsize=(9, 4))
ax.vlines(x, df.low, df.high, color="k", lw=0.8)
ax.bar(x[up], (df.close - df.open)[up], bottom=df.open[up], color="seagreen", width=0.6)
ax.bar(x[~up], (df.close - df.open)[~up], bottom=df.open[~up], color="firebrick", width=0.6)
ax.set_xticks(x[::5], [d.strftime("%d %b") for d in df.index[::5]])
ax.set_ylabel("price")
print(int(up.sum()), round(float(df.close.iloc[-1]), 4))
fig.savefig("candles.png")
