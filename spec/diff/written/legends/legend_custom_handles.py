# Legend with handmade handles (Line2D, Patch) for a scatter plot coloured by a category column.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Patch

rng = np.random.default_rng(3)
colors = {"setosa": "tab:red", "versicolor": "tab:green", "virginica": "tab:blue"}
fig, ax = plt.subplots(figsize=(5.5, 4))
for name, c in colors.items():
    pts = rng.normal(loc=list(colors).index(name), scale=0.4, size=(30, 2))
    ax.scatter(pts[:, 0], pts[:, 1], color=c, s=15)
ax.axhspan(-0.5, 0.5, color="gold", alpha=0.2)
handles = [Line2D([0], [0], marker="o", color="w", markerfacecolor=c, markersize=8, label=n)
           for n, c in colors.items()]
handles.append(Patch(facecolor="gold", alpha=0.2, label="reference band"))
handles.append(Line2D([0], [0], color="k", ls="--", label="threshold"))
ax.axvline(1.0, color="k", ls="--")
ax.legend(handles=handles, loc="lower right", framealpha=0.9)
print([t.get_text() for t in ax.get_legend().get_texts()])
fig.savefig("custom_handles.png")
