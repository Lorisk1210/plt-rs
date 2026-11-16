# Two image rows sharing one colour bar each, plus a figure legend for line overlays, all under constrained layout.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:40j, 0:1:40j]
fig, axs = plt.subplots(2, 3, figsize=(12, 7), layout="constrained")
for r in range(2):
    for c in range(3):
        im = axs[r, c].imshow(np.sin((c + 2) * x + r) * y * (r + 1), cmap=["viridis", "magma"][r], vmin=-2, vmax=2)
        axs[r, c].plot([5, 35], [5, 35], color="w", label="diagonal")
        axs[r, c].plot([5, 35], [35, 5], color="c", label="anti-diagonal")
    fig.colorbar(im, ax=axs[r, :], location="right", shrink=0.9, label=f"row {r}")
h, l = axs[0, 0].get_legend_handles_labels()
fig.legend(h, l, loc="outside lower center", ncols=2)
fig.savefig("shared.png", dpi=50)
fig.canvas.draw()
print([round(v, 3) for v in axs[0, 2].get_position().bounds], len(fig.axes))
