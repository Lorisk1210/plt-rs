# Bin a continuous column with pd.cut and plot counts per bin as bars with interval labels.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(15)
ages = pd.Series(rng.integers(0, 95, 600))
bins = pd.cut(ages, bins=[0, 18, 30, 45, 60, 75, 95], right=False)
counts = bins.value_counts().sort_index()
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.bar(counts.index.astype(str), counts.values, color="slateblue")
ax.set_xlabel("age band")
ax.set_ylabel("people")
plt.setp(ax.get_xticklabels(), rotation=30, ha="right")
print(counts.to_dict())
fig.tight_layout()
fig.savefig("age_bands.png")
