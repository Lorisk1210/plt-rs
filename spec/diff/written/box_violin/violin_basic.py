# Violin plots of three bimodal-ish distributions with medians and extrema shown.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
a = np.concatenate([rng.normal(0, 1, 200), rng.normal(4, 0.8, 100)])
b = rng.normal(2, 1.5, 300)
c = rng.uniform(-1, 5, 300)
fig, ax = plt.subplots(figsize=(6, 4))
parts = ax.violinplot([a, b, c], showmedians=True, showextrema=True, widths=0.8)
for body in parts["bodies"]:
    body.set_facecolor("mediumpurple")
    body.set_edgecolor("black")
    body.set_alpha(0.6)
parts["cmedians"].set_color("black")
ax.set_xticks([1, 2, 3], ["mixture", "normal", "uniform"])
print(sorted(parts.keys()))
fig.savefig("violins.png")
