# DataFrame.hist() grid and DataFrame.boxplot() of numeric columns.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(4)
df = pd.DataFrame({"height": rng.normal(170, 9, 200), "weight": rng.normal(70, 12, 200),
                   "age": rng.integers(18, 70, 200), "income": rng.lognormal(10, 0.4, 200) / 1000})
axes = df.hist(bins=20, figsize=(8, 6), color="steelblue", edgecolor="white")
plt.tight_layout()
plt.savefig("hist_grid.png")
print(axes.shape)
plt.figure()
bp = df[["height", "weight", "age"]].boxplot(grid=False, return_type="dict")
print(sorted(bp.keys()))
plt.savefig("boxes.png")
