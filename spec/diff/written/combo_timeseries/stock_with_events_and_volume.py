# Price chart with earnings events shaded, moving averages, a volume panel and a drawdown panel sharing dates.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

rng = np.random.default_rng(50)
idx = pd.bdate_range("2023-07-01", periods=250)
price = pd.Series(100 * np.exp(np.cumsum(rng.normal(0.0004, 0.015, 250))), index=idx)
vol = pd.Series(rng.lognormal(13, 0.4, 250), index=idx)
events = pd.to_datetime(["2023-08-02", "2023-11-01", "2024-02-01", "2024-05-02"])
fig, (a, b, c) = plt.subplots(3, 1, sharex=True, figsize=(10, 8), gridspec_kw={"height_ratios": [3, 1, 1]})
a.plot(price.index, price, color="k", lw=1)
for w, col in [(20, "C0"), (50, "C1")]:
    a.plot(price.rolling(w).mean(), color=col, label=f"MA{w}")
for e in events:
    a.axvspan(e - pd.Timedelta(days=1), e + pd.Timedelta(days=1), color="gold", alpha=0.4)
a.legend(loc="upper left")
b.bar(vol.index, vol / 1e6, width=1, color=np.where(price.diff() >= 0, "seagreen", "firebrick"))
b.set_ylabel("vol (M)")
dd = price / price.cummax() - 1
c.fill_between(dd.index, dd, 0, color="red", alpha=0.4)
c.yaxis.set_major_formatter(plt.matplotlib.ticker.PercentFormatter(1))
loc = mdates.AutoDateLocator()
c.xaxis.set_major_locator(loc)
c.xaxis.set_major_formatter(mdates.ConciseDateFormatter(loc))
fig.subplots_adjust(hspace=0.05)
fig.canvas.draw()
print([t.get_text() for t in c.get_xticklabels()], round(float(dd.min()), 4))
fig.savefig("stock.png", dpi=50)
