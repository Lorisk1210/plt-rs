# Legend placed in its own empty axes beside a grid of plots (a legend-only panel).
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplot_mosaic([["a", "b", "leg"], ["c", "d", "leg"]], figsize=(9, 5),
                              width_ratios=[1, 1, 0.4])
x = np.linspace(0, 1, 30)
for key, k in zip("abcd", range(4)):
    for p in (1, 2, 3):
        axs[key].plot(x, x ** p * (k + 1), label=f"power {p}")
handles, labels = axs["a"].get_legend_handles_labels()
axs["leg"].legend(handles, labels, loc="center", frameon=False)
axs["leg"].axis("off")
print(len(axs["leg"].get_legend().get_texts()))
fig.savefig("legend_panel.png")
