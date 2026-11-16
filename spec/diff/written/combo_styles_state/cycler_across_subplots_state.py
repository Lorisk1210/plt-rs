# Custom prop cycle set globally, then per-axes; plot in several subplots and read back colour/linestyle sequences.
import numpy as np
import matplotlib as mpl
import matplotlib.pyplot as plt
from cycler import cycler

mpl.rcParams["axes.prop_cycle"] = cycler(color=["#1b9e77", "#d95f02", "#7570b3"]) + cycler(linestyle=["-", "--", ":"])
fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
x = np.linspace(0, 1, 20)
for k in range(4):
    axs[0].plot(x, x * k)
axs[1].set_prop_cycle(cycler(color=["k", "gray"]) * cycler(marker=["o", "s"]))
for k in range(4):
    axs[1].plot(x, x * k, ms=3)
for k in range(2):
    axs[2].bar(np.arange(3) + k * 0.4, [1, 2, 3], 0.4)
    axs[2].fill_between(x * 3, k, k + 0.5, alpha=0.3)
for ax in axs[:2]:
    print([(l.get_color(), l.get_linestyle(), l.get_marker()) for l in ax.lines])
print([p.get_facecolor()[:3] for p in axs[2].patches][::3], [np.round(c.get_facecolor()[0][:3], 3).tolist() for c in axs[2].collections])
fig.savefig("cycles.png", dpi=50)
