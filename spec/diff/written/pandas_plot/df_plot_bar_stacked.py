# Stacked and grouped bar charts from a DataFrame (kind='bar', stacked=True, barh).
import pandas as pd
import matplotlib.pyplot as plt

df = pd.DataFrame({"2022": [12, 7, 5], "2023": [15, 9, 4], "2024": [18, 12, 6]}, index=["web", "app", "store"])
fig, axs = plt.subplots(1, 3, figsize=(12, 3.5))
df.plot(kind="bar", ax=axs[0], rot=0, title="grouped")
df.T.plot(kind="bar", stacked=True, ax=axs[1], rot=0, title="stacked")
df.plot.barh(ax=axs[2], title="horizontal", color=["#1b9e77", "#d95f02", "#7570b3"])
fig.tight_layout()
print([len(ax.patches) for ax in axs])
fig.savefig("bars.png")
current = plt.gcf()
print(len(current.axes), current.get_size_inches())
current.savefig("df_plot_bar_stacked_small.png", dpi=50)
