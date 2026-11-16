# App reviews: rating distribution stacked by version, average rating over time with error band, and keyword bars.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(206)
df = pd.DataFrame({"version": rng.choice(["1.0", "1.1", "2.0"], 600, p=[0.3, 0.3, 0.4]),
                   "day": rng.integers(0, 120, 600)})
df["rating"] = np.clip(np.round(rng.normal(df["version"].map({"1.0": 3.2, "1.1": 3.8, "2.0": 4.3}), 0.9)), 1, 5).astype(int)
fig, axs = plt.subplots(1, 3, figsize=(15, 4.5))
ct = pd.crosstab(df["rating"], df["version"])
ct.plot(kind="bar", stacked=True, ax=axs[0], rot=0)
daily = df.groupby(df["day"] // 7)["rating"].agg(["mean", "std", "count"])
se = daily["std"] / np.sqrt(daily["count"])
axs[1].plot(daily.index, daily["mean"])
axs[1].fill_between(daily.index, daily["mean"] - se, daily["mean"] + se, alpha=0.3)
axs[1].set(xlabel="week", ylabel="avg rating")
words = {"crash": 40, "slow": 33, "love": 28, "update": 25, "battery": 18}
axs[2].bar(list(words), list(words.values()), color=["C3", "C3", "C2", "C0", "C3"])
fig.tight_layout()
print(ct.to_dict(), daily["mean"].round(3).tolist()[:3])
fig.savefig("reviews.png", dpi=50)
