# Donut chart of a budget with a centre total and the legend outside.
import matplotlib.pyplot as plt

budget = {"rent": 1200, "food": 450, "transport": 180, "savings": 600, "fun": 270}
fig, ax = plt.subplots(figsize=(6, 4))
wedges, _ = ax.pie(budget.values(), wedgeprops=dict(width=0.4, edgecolor="w"), startangle=-40,
                   colors=plt.cm.Set2.colors)
ax.text(0, 0, f"{sum(budget.values())} €", ha="center", va="center", fontsize=16, fontweight="bold")
ax.legend(wedges, [f"{k} ({v} €)" for k, v in budget.items()], loc="center left", bbox_to_anchor=(1, 0.5))
ax.set_aspect("equal")
fig.tight_layout()
print(len(wedges), wedges[0].width)
fig.savefig("budget.png")
fig.savefig("donut_center_label_80dpi.png", dpi=80)
print(fig.get_size_inches(), len(fig.axes))
plt.close(fig)
