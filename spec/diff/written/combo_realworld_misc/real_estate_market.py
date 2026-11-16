# Real estate: price per m² by district boxplots, a price vs size scatter with trend per district, and monthly listings bars.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(230)
df = pd.DataFrame({"district": rng.choice(["centre", "north", "east", "harbour"], 400), "size": rng.uniform(30, 160, 400)})
base = df["district"].map({"centre": 9000, "north": 5500, "east": 4500, "harbour": 7000})
df["price"] = df["size"] * base * rng.normal(1, 0.12, 400) / 1000
fig = plt.figure(figsize=(12, 8))
a = fig.add_subplot(2, 2, 1)
names = sorted(df["district"].unique())
a.boxplot([df.loc[df.district == d, "price"] / df.loc[df.district == d, "size"] * 1000 for d in names], tick_labels=names)
a.set_ylabel("€/m²")
b = fig.add_subplot(2, 2, 2)
for d in names:
    g = df[df.district == d]
    sc = b.scatter(g["size"], g["price"], s=6, alpha=0.5, label=d)
    k, c0 = np.polyfit(g["size"], g["price"], 1)
    b.plot([30, 160], [30 * k + c0, 160 * k + c0], color=sc.get_facecolor()[0])
b.legend(fontsize=7)
c = fig.add_subplot(2, 1, 2)
c.bar(pd.period_range("2024-01", periods=12, freq="M").strftime("%b"), rng.integers(20, 60, 12))
fig.tight_layout()
print(df.groupby("district")["price"].median().round(1).to_dict())
fig.savefig("real_estate.png", dpi=50)
