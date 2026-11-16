# Box plots of delivery times per courier with notches, means and outlier styling.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(23)
couriers = ["FastCo", "Speedy", "Budget", "Local"]
times = [rng.gamma(k, 6, 150) for k in (3, 2.5, 5, 4)]
fig, ax = plt.subplots(figsize=(6.5, 4))
bp = ax.boxplot(times, tick_labels=couriers, notch=True, showmeans=True, patch_artist=True,
                flierprops=dict(marker="x", markersize=4, markeredgecolor="gray"),
                meanprops=dict(marker="D", markerfacecolor="white", markeredgecolor="k"))
for patch, color in zip(bp["boxes"], ["#8dd3c7", "#ffffb3", "#bebada", "#fb8072"]):
    patch.set_facecolor(color)
ax.set_ylabel("hours")
print(sorted(bp.keys()), len(bp["fliers"]))
print([round(m.get_ydata()[0], 3) for m in bp["medians"]])
fig.savefig("couriers.png")
