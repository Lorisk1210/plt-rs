# pandas.plotting.scatter_matrix with histograms on the diagonal for four variables.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from pandas.plotting import scatter_matrix

rng = np.random.default_rng(14)
base = rng.normal(0, 1, 150)
df = pd.DataFrame({"a": base, "b": base * 0.6 + rng.normal(0, 0.8, 150), "c": rng.uniform(-2, 2, 150),
                   "d": -base + rng.normal(0, 0.5, 150)})
axes = scatter_matrix(df, figsize=(7, 7), diagonal="hist", alpha=0.6, s=10)
print(axes.shape, round(df.corr().loc["a", "d"], 4))
plt.savefig("matrix.png")
