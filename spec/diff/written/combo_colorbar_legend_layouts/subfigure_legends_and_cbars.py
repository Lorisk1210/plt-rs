# Subfigures each with their own legend (subfig.legend) and colour bar (subfig.colorbar), and a figure suptitle.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(12, 5), layout="constrained")
sfs = fig.subfigures(1, 2, width_ratios=[1.2, 1])
axs = sfs[0].subplots(1, 2, sharey=True)
for k, ax in enumerate(axs):
    ax.plot(np.arange(10), np.arange(10) * (k + 1), label="linear")
    ax.plot(np.arange(10), np.arange(10) ** 1.2, label="power")
sfs[0].legend(*axs[0].get_legend_handles_labels(), loc="outside lower center", ncols=2)
ax2 = sfs[1].subplots()
im = ax2.imshow(np.random.default_rng(221).random((8, 8)))
sfs[1].colorbar(im, ax=ax2, label="value")
fig.suptitle("subfigure legends and colour bars")
print(len(sfs[0].legends), len(fig.legends), len(fig.axes))
fig.savefig("subfig_legend.png", dpi=50)
