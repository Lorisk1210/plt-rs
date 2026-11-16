# Mean and standard deviation per category from a pandas groupby, plotted as errorbars.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(9)
df = pd.DataFrame({"diet": rng.choice(["vegan", "veggie", "omni"], 150), "chol": rng.normal(190, 25, 150)})
df.loc[df.diet == "vegan", "chol"] -= 25
stats = df.groupby("diet")["chol"].agg(["mean", "std", "count"]).reindex(["vegan", "veggie", "omni"])
fig, ax = plt.subplots(figsize=(5, 4))
ax.errorbar(stats.index, stats["mean"], yerr=stats["std"], fmt="o", ms=8, capsize=8, color="darkred")
ax.set_ylabel("cholesterol [mg/dl]")
for i, (name, row) in enumerate(stats.iterrows()):
    ax.text(i, row["mean"] + row["std"] + 3, f"n={row['count']}", ha="center")
print(stats.round(3).to_string())
fig.savefig("diet.png")
