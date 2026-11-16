# Set spine and grid preferences once through rcParams for all following plots.
import numpy as np
import matplotlib.pyplot as plt

plt.rcParams.update({"axes.spines.top": False, "axes.spines.right": False, "axes.grid": True,
                     "grid.linestyle": ":", "grid.alpha": 0.6, "axes.grid.axis": "y",
                     "xtick.direction": "in", "ytick.direction": "in"})
fig, axs = plt.subplots(1, 2, figsize=(8, 3))
axs[0].plot(np.arange(10), np.arange(10) ** 0.5)
axs[1].bar(list("abcd"), [3, 5, 2, 6])
print([ax.spines["top"].get_visible() for ax in axs])
fig.savefig("rc_spines.png")
axs[0].set_title("roots")
axs[1].set_title("bars")
fig.savefig("rc_spines_small.png", dpi=50)
