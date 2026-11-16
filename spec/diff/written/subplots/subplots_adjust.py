# Manually tune the spacing between a 2x3 grid of small multiples with subplots_adjust.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 3, figsize=(8, 4.5))
x = np.linspace(0, 2 * np.pi, 60)
for i, ax in enumerate(axs.ravel()):
    ax.plot(x, np.sin((i + 1) * x), lw=1)
    ax.set_title(f"mode {i + 1}", fontsize=9)
    ax.set_xticks([0, np.pi, 2 * np.pi], ["0", "π", "2π"])
fig.subplots_adjust(left=0.06, right=0.98, bottom=0.08, top=0.9, wspace=0.35, hspace=0.5)
p = fig.subplotpars
print(p.left, p.right, p.bottom, p.top, p.wspace, p.hspace)
print(np.round(axs[1, 2].get_position().bounds, 4))
fig.suptitle("Vibration modes")
fig.savefig("modes.png")
