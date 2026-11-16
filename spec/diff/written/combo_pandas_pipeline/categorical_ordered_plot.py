# Ordered pandas Categorical drives axis order in bars and boxplots; unused categories appear as zero bars.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(119)
sizes = pd.Categorical(rng.choice(["S", "M", "L"], 200), categories=["XS", "S", "M", "L", "XL"], ordered=True)
df = pd.DataFrame({"size": sizes, "price": rng.normal(20, 4, 200)})
counts = df["size"].value_counts(sort=False)
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
a.bar(counts.index.astype(str), counts.values)
groups = [df.loc[df["size"] == c, "price"].values for c in df["size"].cat.categories]
b.boxplot([g if len(g) else [np.nan] for g in groups], tick_labels=list(df["size"].cat.categories))
print(counts.to_dict(), [len(g) for g in groups])
fig.savefig("ordered.png", dpi=50)
