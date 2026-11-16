# One line per group from a long DataFrame using groupby in a loop over ax.plot.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
rows = []
for store in ["A", "B", "C"]:
    for week in range(1, 13):
        rows.append({"store": store, "week": week, "sales": 100 + 10 * week * (1 + "ABC".index(store) / 3) + rng.normal(0, 8)})
df = pd.DataFrame(rows)
fig, ax = plt.subplots(figsize=(7, 4))
for store, g in df.groupby("store"):
    ax.plot(g["week"], g["sales"], marker="o", ms=3, label=f"store {store}")
ax.legend()
ax.set_xlabel("week")
ax.set_ylabel("sales")
print(df.groupby("store")["sales"].sum().round(2).to_dict())
fig.savefig("stores.png")
