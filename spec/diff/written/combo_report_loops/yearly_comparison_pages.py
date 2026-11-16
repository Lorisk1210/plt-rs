# Loop over years saving a page with monthly bars vs last year (hatched), and a YoY line on a secondary axis.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(16)
values = {year: 100 + 10 * np.sin(np.arange(12) / 2) + (year - 2020) * 5 + rng.normal(0, 5, 12) for year in range(2020, 2025)}
months = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"]
x = np.arange(12)
yoy_all = {}
for year in range(2021, 2025):
    cur, prev = values[year], values[year - 1]
    fig, ax = plt.subplots(figsize=(7, 3.5))
    ax.bar(x - 0.2, prev, 0.4, color="white", edgecolor="gray", hatch="//", label=str(year - 1))
    ax.bar(x + 0.2, cur, 0.4, color="C0", label=str(year))
    sec = ax.twinx()
    yoy = (cur / prev - 1) * 100
    sec.plot(x, yoy, "k.-")
    sec.axhline(0, color="k", lw=0.5)
    ax.set_xticks(x, months)
    ax.legend(loc="upper left")
    fig.savefig(f"year_{year}.png", dpi=40)
    yoy_all[year] = round(float(yoy.mean()), 2)
    plt.close(fig)
print(yoy_all)
