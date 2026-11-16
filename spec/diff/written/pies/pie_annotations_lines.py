# Pie with labels placed outside using annotate and connector lines (gallery-style recipe by hand).
import numpy as np
import matplotlib.pyplot as plt

recipe = ["225 g flour", "90 g sugar", "1 egg", "60 g butter", "100 ml milk", "1/2 package yeast"]
data = [225, 90, 50, 60, 100, 5]
fig, ax = plt.subplots(figsize=(7, 4), subplot_kw=dict(aspect="equal"))
wedges, _ = ax.pie(data, wedgeprops=dict(width=0.5), startangle=-40)
kw = dict(arrowprops=dict(arrowstyle="-"), zorder=0, va="center")
for i, p in enumerate(wedges):
    ang = (p.theta2 - p.theta1) / 2.0 + p.theta1
    y = np.sin(np.deg2rad(ang))
    x = np.cos(np.deg2rad(ang))
    ha = {-1: "right", 1: "left"}[int(np.sign(x))]
    kw["arrowprops"].update({"connectionstyle": f"angle,angleA=0,angleB={ang}"})
    ax.annotate(recipe[i], xy=(x, y), xytext=(1.35 * np.sign(x), 1.4 * y), ha=ha, **kw)
ax.set_title("Dough ingredients")
fig.savefig("recipe.png")
print(len(ax.texts))
