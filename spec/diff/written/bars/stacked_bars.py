# Stacked bars of energy sources per country with a running bottom and totals on top.
import numpy as np
import matplotlib.pyplot as plt

countries = ["DE", "FR", "PL", "SE"]
sources = {"coal": [25, 1, 70, 0], "nuclear": [5, 68, 0, 30], "wind": [27, 9, 12, 20], "hydro": [4, 11, 2, 45]}
bottom = np.zeros(len(countries))
fig, ax = plt.subplots(figsize=(6, 4))
for name, vals in sources.items():
    bars = ax.bar(countries, vals, bottom=bottom, label=name)
    ax.bar_label(bars, label_type="center", fmt=lambda v: f"{v:.0f}" if v >= 5 else "", fontsize=8)
    bottom += vals
for i, total in enumerate(bottom):
    ax.text(i, total + 1, f"{total:.0f}%", ha="center")
ax.legend(title="source", bbox_to_anchor=(1.02, 1), loc="upper left")
ax.set_ylim(0, 115)
fig.tight_layout()
print(bottom.tolist())
fig.savefig("energy_stack.png")
