# Rolling mean, rolling band and exponentially weighted mean of a noisy price series.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(17)
s = pd.Series(100 + rng.normal(0, 1, 250).cumsum(), index=pd.bdate_range("2024-01-01", periods=250))
mean = s.rolling(20).mean()
std = s.rolling(20).std()
fig, ax = plt.subplots(figsize=(8, 4))
ax.plot(s.index, s, color="0.6", lw=0.8, label="price")
ax.plot(mean.index, mean, label="20-day mean")
ax.fill_between(mean.index, mean - 2 * std, mean + 2 * std, alpha=0.2, label="±2 sd")
ax.plot(s.ewm(span=20).mean(), ls="--", label="EWM 20")
ax.legend(loc="upper left")
print(round(float(s.iloc[-1]), 4))
fig.savefig("bollinger.png")
