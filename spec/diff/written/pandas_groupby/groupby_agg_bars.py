# Mean and count per category with groupby().agg, bars with counts written on top.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(7)
df = pd.DataFrame({"genre": rng.choice(["drama", "comedy", "action", "horror", "docu"], 300),
                   "rating": rng.normal(6.5, 1.2, 300).clip(1, 10)})
agg = df.groupby("genre")["rating"].agg(["mean", "count"]).sort_values("mean")
fig, ax = plt.subplots(figsize=(6, 3.5))
bars = ax.barh(agg.index, agg["mean"], color="mediumseagreen")
ax.bar_label(bars, labels=[f"n={n}" for n in agg["count"]], padding=3, fontsize=8)
ax.set_xlim(0, 8.5)
ax.set_xlabel("mean rating")
print(agg.round(3).to_dict())
fig.savefig("genres.png")
