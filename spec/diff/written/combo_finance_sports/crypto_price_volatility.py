# Crypto prices: log price lines of three coins, rolling volatility heat map by month, and drawdown histogram.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(164)
idx = pd.date_range("2023-01-01", periods=540, freq="D")
prices = pd.DataFrame({c: s0 * np.exp(np.cumsum(rng.normal(0.001, v, 540))) for c, s0, v in [("BTC", 20000, 0.03), ("ETH", 1500, 0.04), ("SOL", 15, 0.06)]}, index=idx)
vol = np.log(prices).diff().rolling(30).std() * np.sqrt(365)
monthly_vol = vol.resample("MS").mean().T
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 1, 1)
prices.plot(ax=a, logy=True)
b = fig.add_subplot(2, 2, 3)
im = b.imshow(monthly_vol.values, aspect="auto", cmap="magma")
b.set_yticks(range(3), monthly_vol.index)
b.set_xticks(range(0, monthly_vol.shape[1], 3), [d.strftime("%y-%m") for d in monthly_vol.columns[::3]], rotation=45)
fig.colorbar(im, ax=b)
c = fig.add_subplot(2, 2, 4)
dd = prices / prices.cummax() - 1
c.hist([dd[col] for col in dd], bins=30, label=list(dd.columns), histtype="step")
c.legend()
fig.tight_layout()
print(prices.iloc[-1].round(2).to_dict(), dd.min().round(4).to_dict())
fig.savefig("crypto.png", dpi=50)
