# Forecast evaluation: actual vs forecast with interval band, error bars by horizon, and an error distribution boxplot.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(47)
idx = pd.date_range("2024-01-01", periods=120, freq="D")
actual = 100 + 10 * np.sin(np.arange(120) / 7) + np.cumsum(rng.normal(0, 1, 120))
forecast = actual + rng.normal(0, 3, 120)
fig = plt.figure(figsize=(11, 7))
ax1 = fig.add_subplot(2, 1, 1)
ax1.plot(idx, actual, "k", label="actual")
ax1.plot(idx, forecast, "C0", label="forecast")
ax1.fill_between(idx, forecast - 6, forecast + 6, alpha=0.2, label="90% PI")
ax1.legend(loc="upper left", ncols=3)
errors = forecast - actual
horizons = np.arange(1, 8)
mae = [np.abs(errors[h::7]).mean() for h in range(7)]
ax2 = fig.add_subplot(2, 2, 3)
ax2.bar(horizons, mae, yerr=[np.abs(errors[h::7]).std() / 4 for h in range(7)], capsize=3)
ax2.set_xlabel("horizon (days)")
ax3 = fig.add_subplot(2, 2, 4)
ax3.boxplot([errors[h::7] for h in range(7)], tick_labels=[str(h) for h in horizons])
ax3.axhline(0, color="k", lw=0.5)
fig.tight_layout()
print([round(float(m), 3) for m in mae], round(float(np.sqrt(np.mean(errors ** 2))), 4))
fig.savefig("forecast_eval.png", dpi=50)
