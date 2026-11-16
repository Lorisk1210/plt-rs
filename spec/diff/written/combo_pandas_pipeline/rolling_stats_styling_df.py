# Rolling statistics added as columns, plotted with pandas style dict per column and a shaded event window.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(116)
df = pd.DataFrame({"price": 100 + rng.normal(0, 1, 200).cumsum()}, index=pd.bdate_range("2024-01-01", periods=200))
df["ma20"] = df["price"].rolling(20).mean()
df["upper"] = df["ma20"] + 2 * df["price"].rolling(20).std()
df["lower"] = df["ma20"] - 2 * df["price"].rolling(20).std()
ax = df.plot(style={"price": "k-", "ma20": "b--", "upper": "g:", "lower": "g:"}, figsize=(9, 4), lw=1)
ax.axvspan(df.index[120], df.index[140], color="orange", alpha=0.2)
ax.fill_between(df.index, df["lower"], df["upper"], color="g", alpha=0.05)
print([(l.get_label(), l.get_linestyle(), l.get_color()) for l in ax.get_lines()])
plt.savefig("bands.png", dpi=50)
