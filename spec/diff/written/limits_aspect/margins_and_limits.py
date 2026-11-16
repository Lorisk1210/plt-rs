# Control the padding around data: margins(), set_xlim with None, autoscale(tight=True).
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 50)
y = np.exp(-x / 3)
fig, axs = plt.subplots(1, 4, figsize=(12, 3), sharey=False)
for ax in axs:
    ax.plot(x, y)
axs[0].set_title("default")
axs[1].margins(x=0, y=0.2)
axs[1].set_title("margins(x=0, y=.2)")
axs[2].set_xlim(2, None)
axs[2].set_ylim(bottom=0)
axs[2].set_title("partial limits")
axs[3].autoscale(enable=True, axis="both", tight=True)
axs[3].set_title("tight")
for ax in axs:
    print(np.round(ax.get_xlim(), 3), np.round(ax.get_ylim(), 3))
fig.tight_layout()
fig.savefig("limits.png")
