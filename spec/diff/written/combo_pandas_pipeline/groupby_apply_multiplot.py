# groupby().apply to compute per-group fits, then plot each group's points and fitted line with an equation legend.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(113)
df = pd.DataFrame({"g": np.repeat(list("ABC"), 40), "x": np.tile(np.linspace(0, 10, 40), 3)})
df["y"] = df["x"] * df["g"].map({"A": 0.5, "B": 1.2, "C": -0.3}) + rng.normal(0, 1, len(df))
fits = df.groupby("g").apply(lambda d: pd.Series(np.polyfit(d["x"], d["y"], 1), index=["slope", "intercept"]), include_groups=False)
fig, ax = plt.subplots(figsize=(7, 4.5))
for g, d in df.groupby("g"):
    sc = ax.scatter(d["x"], d["y"], s=10, alpha=0.6)
    s, i = fits.loc[g]
    ax.plot([0, 10], [i, 10 * s + i], color=sc.get_facecolor()[0], label=f"{g}: y = {s:.2f}x {i:+.2f}")
ax.legend()
print(fits.round(3).to_dict())
fig.savefig("group_fits.png", dpi=50)
