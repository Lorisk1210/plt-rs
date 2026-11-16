# Additive decomposition (trend by rolling mean, seasonal by monthly means) in four stacked panels.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(2)
idx = pd.date_range("2018-01-01", periods=72, freq="MS")
series = pd.Series(100 + np.arange(72) * 0.8 + 10 * np.sin(2 * np.pi * idx.month / 12) + rng.normal(0, 2, 72), index=idx)
trend = series.rolling(12, center=True).mean()
detrended = series - trend
seasonal = detrended.groupby(series.index.month).transform("mean")
resid = series - trend - seasonal
fig, axs = plt.subplots(4, 1, figsize=(8, 7), sharex=True)
for ax, (name, comp) in zip(axs, [("observed", series), ("trend", trend), ("seasonal", seasonal), ("residual", resid)]):
    ax.plot(comp.index, comp, lw=1)
    ax.set_ylabel(name)
axs[3].axhline(0, color="k", lw=0.5)
print(round(float(seasonal.max()), 4), round(float(resid.std()), 4))
fig.savefig("decomposition.png")
