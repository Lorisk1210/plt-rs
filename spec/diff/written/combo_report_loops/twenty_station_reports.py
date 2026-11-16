# Report loop: one figure per weather station (20 stations), each with temps, rain bars on a twin axis and a stats box.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(10)
days = pd.date_range("2024-01-01", periods=60, freq="D")
summary = []
for k in range(20):
    temp = 5 + k * 0.3 + 6 * np.sin(np.arange(60) / 9) + rng.normal(0, 1.5, 60)
    rain = np.where(rng.random(60) > 0.7, rng.gamma(2, 3, 60), 0)
    fig, ax = plt.subplots(figsize=(6, 3.2))
    ax.plot(days, temp, color="tab:red")
    ax2 = ax.twinx()
    ax2.bar(days, rain, color="tab:blue", alpha=0.4, width=0.8)
    ax.set_title(f"Station {k:02d}")
    ax.text(0.01, 0.95, f"mean {temp.mean():.1f}°C\nrain {rain.sum():.0f} mm", transform=ax.transAxes,
            va="top", fontsize=8, bbox=dict(fc="w", alpha=0.8))
    fig.autofmt_xdate()
    fig.savefig(f"station_{k:02d}.png", dpi=40)
    summary.append((k, round(temp.mean(), 2), round(ax2.get_ylim()[1], 2)))
    plt.close(fig)
print(summary[:3], summary[-1])
print(plt.get_fignums())
