"""Donut chart: how a household's monthly budget is spent, with the total
in the hole."""

import pltrs.pyplot as plt

spending = {
    "housing": 1250,
    "food": 540,
    "transport": 310,
    "health": 220,
    "leisure": 260,
    "savings": 400,
    "other": 170,
}

fig, ax = plt.subplots(figsize=(6.4, 4.5), layout="constrained")
wedges, labels, percents = ax.pie(
    list(spending.values()),
    labels=list(spending),
    autopct="%1.0f%%",
    pctdistance=0.8,
    startangle=90,
    counterclock=False,
    wedgeprops=dict(width=0.4, edgecolor="white", linewidth=2),
    textprops=dict(fontsize=9),
)
for percent in percents:
    percent.set_color("white")
ax.text(0, 0, f"€{sum(spending.values()):,}\nper month", ha="center", va="center", fontsize=13)
ax.set_title("Where the money goes")

fig.savefig("donut.png")
