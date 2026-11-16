# Forecast with a confidence band over dates using fill_between and a vertical "today" line.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

hist = pd.date_range("2024-01-01", periods=60, freq="D")
fut = pd.date_range(hist[-1] + pd.Timedelta(days=1), periods=30, freq="D")
y_hist = 100 + np.arange(60) * 0.5 + 5 * np.sin(np.arange(60) / 4)
y_fut = y_hist[-1] + np.arange(1, 31) * 0.5
spread = np.linspace(1, 12, 30)
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(hist, y_hist, color="k", label="observed")
ax.plot(fut, y_fut, color="C0", ls="--", label="forecast")
ax.fill_between(fut, y_fut - spread, y_fut + spread, color="C0", alpha=0.2, label="90% band")
ax.axvline(pd.Timestamp("2024-02-29"), color="red", lw=1)
ax.legend(loc="upper left")
ax.set_xlim(hist[0], fut[-1])
print(ax.get_xlim())
fig.savefig("forecast.png")
