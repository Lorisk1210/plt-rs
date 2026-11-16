# Diverging bars: monthly balance coloured green when positive and red when negative.
import numpy as np
import matplotlib.pyplot as plt

balance = np.array([120, -40, 60, -80, 150, 30, -20, 90, -10, 70, 110, -60])
months = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"]
fig, ax = plt.subplots(figsize=(7, 3.5))
colors = np.where(balance >= 0, "seagreen", "indianred")
bars = ax.bar(range(12), balance, color=colors)
ax.axhline(0, color="black", lw=0.8)
ax.set_xticks(range(12), months)
ax.bar_label(bars, fmt="%+d", fontsize=7, padding=2)
ax.set_title(f"Net {balance.sum():+d}")
fig.savefig("balance.png")
print(ax.get_ylim())
