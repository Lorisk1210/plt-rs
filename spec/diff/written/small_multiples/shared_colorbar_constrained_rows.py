# Two rows of maps with one colour bar per row in constrained layout.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:40j, 0:1:40j]
fig, axs = plt.subplots(2, 3, figsize=(9, 5), layout="constrained")
for row, scale in enumerate([1, 100]):
    for col in range(3):
        im = axs[row, col].pcolormesh(np.sin((col + 2) * x) * np.cos(3 * y) * scale, cmap=["viridis", "plasma"][row])
        axs[row, col].set_aspect("equal")
        axs[row, col].set_title(f"r{row} c{col}", fontsize=8)
    fig.colorbar(im, ax=axs[row, :], shrink=0.8, label=f"scale {scale}")
print(len(fig.axes))
fig.savefig("rows_cbar.png")
