# Bump chart of team rankings over seasons with inverted y and labels on both ends.
import matplotlib.pyplot as plt

seasons = [2019, 2020, 2021, 2022, 2023]
ranks = {"Lions": [1, 2, 4, 3, 1], "Tigers": [2, 1, 1, 2, 3], "Bears": [3, 4, 2, 1, 2], "Wolves": [4, 3, 3, 4, 4]}
fig, ax = plt.subplots(figsize=(7, 4))
for team, r in ranks.items():
    ax.plot(seasons, r, "o-", lw=3, ms=10)
    ax.text(seasons[0] - 0.15, r[0], team, ha="right", va="center")
    ax.text(seasons[-1] + 0.15, r[-1], team, ha="left", va="center")
ax.invert_yaxis()
ax.set_yticks([1, 2, 3, 4])
ax.set_xticks(seasons)
ax.set_xlim(seasons[0] - 1, seasons[-1] + 1)
ax.spines[["top", "right", "left"]].set_visible(False)
print(ax.get_ylim())
fig.savefig("bump.png")
