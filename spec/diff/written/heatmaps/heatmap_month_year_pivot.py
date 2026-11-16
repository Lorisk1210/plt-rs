# Month x year heat map of temperatures from a long pandas table pivoted by year and month.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

idx = pd.date_range("2015-01-01", "2024-12-01", freq="MS")
temp = 10 + 9 * np.sin((idx.month - 4) / 12 * 2 * np.pi) + (idx.year - 2015) * 0.08
df = pd.DataFrame({"year": idx.year, "month": idx.month, "temp": temp})
pivot = df.pivot(index="year", columns="month", values="temp")
fig, ax = plt.subplots(figsize=(7, 4.5))
im = ax.imshow(pivot, cmap="coolwarm", aspect="auto")
ax.set_xticks(range(12), ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"])
ax.set_yticks(range(len(pivot.index)), pivot.index)
fig.colorbar(im, label="°C")
print(pivot.shape, round(float(pivot.values.mean()), 4))
fig.savefig("month_year.png")
