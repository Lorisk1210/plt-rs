# Equity curve and its drawdown below zero filled in red, max drawdown annotated.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(6)
returns = rng.normal(0.0005, 0.01, 500)
equity = np.cumprod(1 + returns)
peak = np.maximum.accumulate(equity)
dd = equity / peak - 1
fig, (a, b) = plt.subplots(2, 1, figsize=(8, 5), sharex=True)
a.plot(equity, color="navy")
a.set_ylabel("equity")
b.fill_between(np.arange(500), dd, 0, color="red", alpha=0.4)
i = dd.argmin()
b.annotate(f"max DD {dd[i]:.1%}", xy=(i, dd[i]), xytext=(i + 30, dd[i] / 2), arrowprops=dict(arrowstyle="->"))
b.yaxis.set_major_formatter(plt.matplotlib.ticker.PercentFormatter(1.0))
print(round(float(dd.min()), 5))
fig.savefig("drawdown.png")
