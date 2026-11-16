# Create a 2x3 grid, then change course: delete two axes, add a spanning axes via GridSpec, move a legend to it.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 3, figsize=(11, 6))
for ax in axs.flat:
    ax.plot(np.arange(5), np.random.default_rng(244).random(5), label="series")
gs = axs[1, 1].get_gridspec()
for ax in axs[1, 1:]:
    ax.remove()
big = fig.add_subplot(gs[1, 1:])
big.bar(range(10), np.arange(10) % 4, color="C1")
big.legend(*axs[0, 0].get_legend_handles_labels(), loc="upper left")
fig.tight_layout()
print(len(fig.axes), big.get_subplotspec().colspan, axs[1, 0].get_subplotspec().rowspan)
fig.savefig("rearranged.png", dpi=50)
