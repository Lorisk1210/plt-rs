# Legend centered below the plot in several columns, with room made by subplots_adjust.
import numpy as np
import matplotlib.pyplot as plt

years = np.arange(2015, 2025)
fig, ax = plt.subplots(figsize=(7, 4.5))
for i, region in enumerate(["Europe", "Asia", "Africa", "Americas", "Oceania"]):
    ax.plot(years, 50 + 10 * i + np.sin(years + i) * 5, marker="o", ms=3, label=region)
ax.legend(loc="upper center", bbox_to_anchor=(0.5, -0.15), ncol=5, frameon=False, fontsize=9)
fig.subplots_adjust(bottom=0.25)
ax.set_title("Index by region")
fig.savefig("below.png")
print(ax.get_legend()._ncols)
ax.set_xlabel("year")
ax.grid(axis="y", alpha=0.4)
fig.savefig("below_tight.png", bbox_inches="tight")
