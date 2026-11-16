# Pivot a long time series to wide, plot small multiples with pandas subplots, then highlight the max column.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(114)
long = pd.DataFrame({"date": np.tile(pd.date_range("2024-01-01", periods=30, freq="D"), 4),
                     "sensor": np.repeat(["s1", "s2", "s3", "s4"], 30), "value": rng.normal(0, 1, 120).cumsum()})
wide = long.pivot(index="date", columns="sensor", values="value")
axes = wide.plot(subplots=True, layout=(2, 2), figsize=(9, 6), sharex=True, legend=False)
best = wide.iloc[-1].idxmax()
for ax, col in zip(axes.flat, wide.columns):
    ax.set_title(col, color="red" if col == best else "k")
    if col == best:
        ax.set_facecolor("#fff3f3")
plt.tight_layout()
print(best, wide.iloc[-1].round(3).to_dict(), axes.shape)
plt.savefig("sensors.png", dpi=50)
