# Percent-stacked bars from pd.crosstab(normalize='index') with labels inside segments.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(10)
df = pd.DataFrame({"age": rng.choice(["18-29", "30-49", "50-64", "65+"], 500),
                   "answer": rng.choice(["yes", "no", "unsure"], 500, p=[0.5, 0.35, 0.15])})
ct = pd.crosstab(df["age"], df["answer"], normalize="index") * 100
ax = ct.plot(kind="barh", stacked=True, figsize=(7, 3.5), color=["#d73027", "#fee08b", "#1a9850"])
for c in ax.containers:
    ax.bar_label(c, fmt="%.0f%%", label_type="center", fontsize=8)
ax.legend(loc="lower right", ncols=3, fontsize=8)
ax.set_xlim(0, 100)
print(ct.round(2).iloc[0].to_dict())
plt.savefig("crosstab.png")
