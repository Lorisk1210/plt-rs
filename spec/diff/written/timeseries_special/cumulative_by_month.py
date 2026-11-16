# Cumulative sum resetting each month, drawn as a step chart with month boundaries.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(7)
idx = pd.date_range("2024-01-01", "2024-04-30", freq="D")
sales = pd.Series(rng.poisson(20, len(idx)), index=idx)
mtd = sales.groupby(sales.index.to_period("M")).cumsum()
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.step(mtd.index, mtd, where="post")
for m in pd.date_range("2024-02-01", "2024-04-01", freq="MS"):
    ax.axvline(m, color="gray", ls=":")
ax.set_ylabel("month to date")
print(mtd.groupby(mtd.index.month).max().to_dict())
fig.savefig("mtd.png")
