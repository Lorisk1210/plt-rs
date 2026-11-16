# Sales dashboard from one DataFrame: trend, region bars, product pie, heat map, histogram and KPI text.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(1)
days = pd.date_range("2024-01-01", periods=180, freq="D")
df = pd.DataFrame({
    "date": np.repeat(days, 3),
    "region": np.tile(["north", "south", "west"], 180),
    "product": rng.choice(["basic", "plus", "pro"], 540, p=[0.5, 0.3, 0.2]),
    "revenue": rng.gamma(4, 50, 540),
})
fig, axd = plt.subplot_mosaic([["trend", "trend", "kpi"], ["bars", "pie", "hist"], ["heat", "heat", "heat"]],
                              figsize=(12, 10), layout="constrained")
daily = df.groupby("date")["revenue"].sum()
axd["trend"].plot(daily.index, daily, lw=0.8, color="0.6")
axd["trend"].plot(daily.rolling(14).mean(), color="C0", lw=2, label="14-day mean")
axd["trend"].legend(loc="upper left")
axd["trend"].set_title("Daily revenue")
by_region = df.groupby("region")["revenue"].sum().sort_values()
bars = axd["bars"].barh(by_region.index, by_region.values, color=["C1", "C2", "C3"])
axd["bars"].bar_label(bars, fmt="{:,.0f}", padding=3)
axd["bars"].set_xlim(0, by_region.max() * 1.25)
share = df["product"].value_counts()
axd["pie"].pie(share, labels=share.index, autopct="%1.0f%%", colors=[b.get_facecolor() for b in bars])
axd["hist"].hist(df["revenue"], bins=30, color="C4")
axd["hist"].set_xlabel("order value")
pivot = df.assign(month=df["date"].dt.month).pivot_table(index="region", columns="month", values="revenue", aggfunc="sum")
im = axd["heat"].imshow(pivot, aspect="auto", cmap="YlGnBu")
axd["heat"].set_xticks(range(pivot.shape[1]), [f"M{m}" for m in pivot.columns])
axd["heat"].set_yticks(range(pivot.shape[0]), pivot.index)
fig.colorbar(im, ax=axd["heat"], label="revenue")
axd["kpi"].axis("off")
axd["kpi"].text(0.5, 0.6, f"{df['revenue'].sum():,.0f}", ha="center", fontsize=24, fontweight="bold")
axd["kpi"].text(0.5, 0.35, "total revenue", ha="center", color="0.4")
fig.suptitle("H1 2024 sales")
print(by_region.round(1).to_dict())
print([tuple(round(c, 3) for c in w.get_facecolor()) for w in axd["pie"].patches])
fig.savefig("sales_dashboard.png", dpi=60)
