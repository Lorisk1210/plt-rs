# Marketing campaigns: spend vs revenue scatter with ROI-coloured bubbles, channel funnel bars, and a daily spend stackplot.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(203)
camps = pd.DataFrame({"spend": rng.uniform(1, 20, 25) * 1000, "channel": rng.choice(["search", "social", "email", "tv"], 25)})
camps["revenue"] = camps["spend"] * rng.uniform(0.5, 4, 25)
camps["roi"] = camps["revenue"] / camps["spend"] - 1
fig = plt.figure(figsize=(12, 8))
a = fig.add_subplot(2, 2, 1)
sc = a.scatter(camps["spend"], camps["revenue"], c=camps["roi"], s=camps["spend"] / 50, cmap="RdYlGn", vmin=-0.5, vmax=3)
a.plot([0, 20000], [0, 20000], "k--", lw=0.8)
fig.colorbar(sc, ax=a, label="ROI")
b = fig.add_subplot(2, 2, 2)
by = camps.groupby("channel")[["spend", "revenue"]].sum()
by.plot.bar(ax=b, rot=0)
c = fig.add_subplot(2, 1, 2)
days = pd.date_range("2024-03-01", periods=30)
daily = {ch: rng.gamma(2, 100, 30) for ch in ["search", "social", "email", "tv"]}
c.stackplot(days, *daily.values(), labels=list(daily))
c.legend(loc="upper left", ncols=4)
fig.tight_layout()
print(by.round(0).to_dict(), round(float(camps["roi"].median()), 4))
fig.savefig("campaigns.png", dpi=50)
