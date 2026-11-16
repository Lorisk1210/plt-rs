# Many legends: per-axes legend, a second legend via add_artist, a figure legend and a legend in its own axes.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

fig, axs = plt.subplots(1, 3, figsize=(13, 4))
x = np.linspace(0, 1, 20)
for ax in axs[:2]:
    a = ax.plot(x, x, "o-", label="data")[0]
    b = ax.plot(x, x ** 2, "s--", label="model")[0]
first = axs[0].legend(loc="upper left")
axs[0].add_artist(first)
axs[0].legend(handles=[Line2D([], [], color="k", ls=":", label="reference")], loc="lower right")
axs[1].legend(loc="center left", bbox_to_anchor=(1, 0.5), title="kind")
axs[2].axis("off")
axs[2].legend(*axs[0].get_legend_handles_labels(), loc="center", title="copied")
fig.legend(*axs[1].get_legend_handles_labels(), loc="lower center", ncols=2)
fig.subplots_adjust(bottom=0.2, wspace=0.6)
print(len(fig.legends), [type(a).__name__ for a in axs[0].artists], axs[2].get_legend().get_title().get_text())
fig.savefig("legends.png", dpi=50)
