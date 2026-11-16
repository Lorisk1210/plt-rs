# One small pie per year in a row of subplots, same colours for the same categories.
import matplotlib.pyplot as plt

years = {2021: [50, 30, 20], 2022: [45, 35, 20], 2023: [38, 40, 22], 2024: [30, 45, 25]}
cats = ["print", "online", "tv"]
colors = ["#e41a1c", "#377eb8", "#4daf4a"]
fig, axs = plt.subplots(1, 4, figsize=(10, 3))
for ax, (year, vals) in zip(axs, years.items()):
    ax.pie(vals, colors=colors, autopct="%d%%", textprops=dict(color="w", fontsize=8))
    ax.set_title(str(year))
fig.legend(cats, loc="lower center", ncols=3)
fig.subplots_adjust(bottom=0.15)
print(len(fig.axes))
fig.savefig("ad_spend.png")
