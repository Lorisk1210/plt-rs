# Two subfigures side by side, each with its own suptitle, colour and layout, for a paper figure.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(9, 4), layout="constrained")
left, right = fig.subfigures(1, 2, wspace=0.05, width_ratios=[1.4, 1])
left.set_facecolor("0.95")
axs = left.subplots(2, 2, sharex=True)
for i, ax in enumerate(axs.flat):
    ax.plot(np.linspace(0, 1, 30) ** (i + 1))
left.suptitle("(a) growth curves")
ax_r = right.subplots()
im = ax_r.imshow(np.arange(25).reshape(5, 5), cmap="viridis")
right.colorbar(im, ax=ax_r, shrink=0.7)
right.suptitle("(b) matrix")
fig.suptitle("Figure 3")
print(type(left).__name__, len(left.axes), len(right.axes), len(fig.axes))
fig.savefig("subfigures.png")
