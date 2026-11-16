# DataFrame.plot.scatter with c= a column, colormap and size from a column.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(6)
df = pd.DataFrame({"area": rng.uniform(30, 200, 80)})
df["price"] = df["area"] * 3.2 + rng.normal(0, 40, 80)
df["rooms"] = (df["area"] // 35 + 1).astype(int)
ax = df.plot.scatter(x="area", y="price", c="rooms", colormap="viridis", s=df["rooms"] * 12, figsize=(6, 4))
ax.set_title("Flats")
print(round(df["price"].corr(df["area"]), 4))
plt.savefig("flats.png")
