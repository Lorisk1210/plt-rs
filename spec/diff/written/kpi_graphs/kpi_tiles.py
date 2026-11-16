# Dashboard KPI tiles: big numbers with deltas in coloured text inside framed axes.
import matplotlib.pyplot as plt

kpis = [("Users", 12840, +4.2), ("Revenue", 98.3, -1.5), ("Churn", 2.1, -0.3), ("NPS", 47, +3.0)]
fig, axs = plt.subplots(1, 4, figsize=(10, 2.2))
for ax, (name, value, delta) in zip(axs, kpis):
    ax.text(0.5, 0.75, name, ha="center", fontsize=11, color="0.4", transform=ax.transAxes)
    ax.text(0.5, 0.4, f"{value:,}", ha="center", fontsize=22, fontweight="bold", transform=ax.transAxes)
    ax.text(0.5, 0.12, f"{delta:+.1f}%", ha="center", color="green" if delta > 0 else "red", transform=ax.transAxes)
    ax.set_xticks([])
    ax.set_yticks([])
    for s in ax.spines.values():
        s.set_edgecolor("0.8")
print(len(axs))
fig.savefig("kpis.png")
