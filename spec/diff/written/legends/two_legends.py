# Two legends in one axes: one for line styles (models) and one for colours (datasets), add_artist idiom.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 5, 40)
fig, ax = plt.subplots(figsize=(6, 4))
styles = {"linear": "-", "quadratic": "--"}
colors = {"train": "C0", "test": "C3"}
for model, ls in styles.items():
    for data, c in colors.items():
        y = x if model == "linear" else x ** 2 / 5
        ax.plot(x, y + (0.3 if data == "test" else 0), ls=ls, color=c)
style_handles = [plt.Line2D([], [], color="k", ls=ls) for ls in styles.values()]
color_handles = [plt.Line2D([], [], color=c, lw=4) for c in colors.values()]
first = ax.legend(style_handles, styles.keys(), loc="upper left", title="model")
ax.add_artist(first)
ax.legend(color_handles, colors.keys(), loc="lower right", title="data")
print(len(ax.artists), ax.get_legend().get_title().get_text())
fig.savefig("two_legends.png")
