# A table-like figure: one row per metric with name, sparkline and last value text.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
metrics = {"CPU %": 40, "Memory GB": 12, "Requests/s": 900, "Errors/min": 3, "Latency ms": 120}
fig, axs = plt.subplots(len(metrics), 3, figsize=(6, 4), gridspec_kw={"width_ratios": [1.2, 3, 1]})
for row, (name, base) in enumerate(metrics.items()):
    series = base * (1 + 0.1 * np.cumsum(rng.normal(0, 0.3, 50)) / 5)
    axs[row, 0].text(0, 0.5, name, va="center")
    axs[row, 1].plot(series, lw=1)
    axs[row, 1].fill_between(range(50), series.min(), series, alpha=0.2)
    axs[row, 2].text(0, 0.5, f"{series[-1]:.1f}", va="center", fontweight="bold")
    for ax in axs[row]:
        ax.axis("off")
print(axs.shape)
fig.savefig("sparklines.png")
