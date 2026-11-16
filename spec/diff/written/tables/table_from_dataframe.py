# Render a pandas DataFrame summary as a matplotlib table next to a chart.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(2)
df = pd.DataFrame({"x": rng.normal(10, 2, 50), "y": rng.normal(5, 1, 50)})
summary = df.describe().loc[["mean", "std", "min", "max"]].round(2)
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5), gridspec_kw={"width_ratios": [2, 1]})
a.scatter(df["x"], df["y"], s=10)
a.set_xlabel("x")
a.set_ylabel("y")
b.axis("off")
t = b.table(cellText=summary.values, rowLabels=summary.index, colLabels=summary.columns, loc="center")
t.set_fontsize(9)
print(summary.to_string())
fig.savefig("summary.png")
