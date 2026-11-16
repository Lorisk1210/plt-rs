# Constrained layout stress: suptitle, supxlabel, colour bars on two panels, a legend outside, rotated ticks, long titles.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 3, figsize=(12, 7), layout="constrained")
y, x = np.mgrid[0:1:30j, 0:1:30j]
for k, ax in enumerate(axs.flat):
    if k in (1, 4):
        im = ax.imshow(np.sin((k + 2) * x) * y, cmap="viridis")
        fig.colorbar(im, ax=ax, label=f"field {k}")
    else:
        for j in range(3):
            ax.plot(np.arange(8), np.arange(8) * (j + 1), label=f"series {j}")
        ax.set_xticks(range(8), [f"category {i}" for i in range(8)], rotation=45, ha="right")
    ax.set_title(f"A long title for panel number {k}", fontsize=9)
handles, labels = axs[0, 0].get_legend_handles_labels()
fig.legend(handles, labels, loc="outside right center")
fig.suptitle("Constrained layout with everything")
fig.supxlabel("shared x label")
fig.savefig("constrained_all.png", dpi=50)
fig.canvas.draw()
print([[round(v, 3) for v in ax.get_position().bounds] for ax in axs.flat][:2])
