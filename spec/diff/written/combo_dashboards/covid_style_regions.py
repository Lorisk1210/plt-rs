# Regional epidemic-style dashboard: log cases per region, 7-day growth bars, a ranking table and a map-like grid.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(9)
days = pd.date_range("2024-01-01", periods=60, freq="D")
regions = ["R1", "R2", "R3", "R4", "R5"]
rates = rng.uniform(0.03, 0.12, 5)
cases = pd.DataFrame({r: np.round(10 * np.exp(rate * np.arange(60)) * rng.uniform(0.8, 1.2, 60)) for r, rate in zip(regions, rates)}, index=days)
fig, axs = plt.subplots(2, 2, figsize=(11, 8))
cases.plot(ax=axs[0, 0], logy=True, legend=False)
axs[0, 0].set_title("cases (log)")
growth = (cases.iloc[-1] / cases.iloc[-8] - 1) * 100
bars = axs[0, 1].barh(growth.index, growth.values, color=[l.get_color() for l in axs[0, 0].get_lines()])
axs[0, 1].bar_label(bars, fmt="%.0f%%")
axs[0, 1].set_title("7-day growth")
rank = cases.iloc[-1].sort_values(ascending=False).astype(int)
axs[1, 0].axis("off")
axs[1, 0].table(cellText=[[r, f"{v:,}"] for r, v in rank.items()], colLabels=["region", "cases"], loc="center")
grid = np.array([[growth[r] for r in regions[:3]], [growth[r] for r in regions[2:]]])
im = axs[1, 1].imshow(grid, cmap="Reds")
fig.colorbar(im, ax=axs[1, 1], label="growth %")
fig.tight_layout()
print(rank.to_dict())
print([round(v, 1) for v in growth.values])
fig.savefig("regions.png", dpi=60)
