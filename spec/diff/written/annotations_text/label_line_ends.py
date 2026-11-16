# Label each line at its right end instead of a legend (common in data journalism).
import numpy as np
import matplotlib.pyplot as plt

years = np.arange(2000, 2025)
series = {"Norway": 1.2, "Spain": 0.8, "Chile": 1.6, "Japan": 0.4}
fig, ax = plt.subplots(figsize=(7, 4))
for i, (name, rate) in enumerate(series.items()):
    y = 100 * (1 + rate / 100) ** (years - 2000)
    ax.plot(years, y, color=f"C{i}")
    ax.text(years[-1] + 0.3, y[-1], name, color=f"C{i}", va="center")
ax.spines[["top", "right"]].set_visible(False)
ax.set_xlim(2000, 2028)
ax.set_title("Index (2000 = 100)", loc="left")
fig.savefig("labelled_lines.png")
print(len(ax.texts))
