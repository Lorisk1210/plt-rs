# Merge two DataFrames, compute derived columns, scatter_matrix coloured by a category and a correlation bar.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from pandas.plotting import scatter_matrix

rng = np.random.default_rng(111)
cities = pd.DataFrame({"city": [f"c{i}" for i in range(40)], "pop": rng.lognormal(12, 1, 40), "region": rng.choice(["N", "S"], 40)})
econ = pd.DataFrame({"city": [f"c{i}" for i in range(40)], "gdp": rng.lognormal(10, 0.5, 40), "jobs": rng.uniform(0.4, 0.8, 40)})
df = cities.merge(econ, on="city").assign(gdp_pc=lambda d: d["gdp"] / d["pop"] * 1e3, log_pop=lambda d: np.log10(d["pop"]))
colors = df["region"].map({"N": "C0", "S": "C1"})
axes = scatter_matrix(df[["log_pop", "gdp_pc", "jobs"]], c=colors, figsize=(7, 7), diagonal="hist", s=15)
plt.savefig("matrix.png", dpi=40)
fig, ax = plt.subplots(figsize=(5, 3))
corr = df[["log_pop", "gdp_pc", "jobs"]].corr()["gdp_pc"].drop("gdp_pc")
corr.plot.barh(ax=ax, color=np.where(corr > 0, "C2", "C3"))
print(axes.shape, corr.round(4).to_dict(), df.shape)
fig.savefig("corr.png", dpi=50)
