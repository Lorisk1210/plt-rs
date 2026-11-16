# Fleet report: fuel efficiency scatter by vehicle type with fitted lines, monthly fuel cost stacked bars, and outlier table.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(217)
df = pd.DataFrame({"type": rng.choice(["van", "truck", "car"], 120), "km": rng.uniform(500, 5000, 120)})
df["litres"] = df["km"] * df["type"].map({"van": 0.09, "truck": 0.25, "car": 0.06}) * rng.normal(1, 0.1, 120)
df.loc[[5, 50], "litres"] *= 1.8
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 2, 1)
for kind, g in df.groupby("type"):
    sc = a.scatter(g["km"], g["litres"], s=10, label=kind)
    k, b0 = np.polyfit(g["km"], g["litres"], 1)
    a.plot([500, 5000], [500 * k + b0, 5000 * k + b0], color=sc.get_facecolor()[0])
a.legend()
b = fig.add_subplot(2, 2, 2)
monthly = pd.DataFrame(rng.gamma(5, 300, (6, 3)), columns=["van", "truck", "car"], index=["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
monthly.plot(kind="bar", stacked=True, ax=b, rot=0)
c = fig.add_subplot(2, 1, 2); c.axis("off")
ratio = df["litres"] / df["km"] / df["type"].map(df.groupby("type").apply(lambda g: (g["litres"] / g["km"]).median(), include_groups=False))
out = df[ratio > 1.4].round(1)
c.table(cellText=out.values, colLabels=out.columns, loc="center")
fig.tight_layout()
print(len(out), monthly.sum().round(1).to_dict())
fig.savefig("fleet.png", dpi=50)
