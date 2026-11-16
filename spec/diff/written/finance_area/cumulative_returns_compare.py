# Compare cumulative returns of three assets with percent formatting and end labels.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter

rng = np.random.default_rng(14)
idx = pd.bdate_range("2023-01-02", periods=260)
rets = pd.DataFrame(rng.normal([0.0004, 0.0002, 0.0007], [0.01, 0.005, 0.02], (260, 3)), index=idx, columns=["stocks", "bonds", "crypto"])
cum = (1 + rets).cumprod() - 1
fig, ax = plt.subplots(figsize=(8, 4))
for col in cum:
    ax.plot(cum.index, cum[col], label=col)
    ax.annotate(f"{cum[col].iloc[-1]:.1%}", (cum.index[-1], cum[col].iloc[-1]), xytext=(4, 0), textcoords="offset points", va="center")
ax.yaxis.set_major_formatter(PercentFormatter(1.0))
ax.axhline(0, color="k", lw=0.6)
ax.legend()
print(cum.iloc[-1].round(4).to_dict())
fig.savefig("cumulative.png")
