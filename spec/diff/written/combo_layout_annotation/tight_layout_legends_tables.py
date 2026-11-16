# tight_layout with a table below one axes, a legend outside another and a colour bar, then bbox_inches tight save.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(13, 4))
axs[0].bar(range(4), [3, 5, 2, 4])
axs[0].table(cellText=[["3", "5", "2", "4"]], rowLabels=["n"], colLabels=list("abcd"), loc="bottom", bbox=[0, -0.35, 1, 0.2])
axs[0].set_xticks([])
for k in range(4):
    axs[1].plot(np.arange(5), np.arange(5) ** (k / 2), label=f"p={k / 2}")
axs[1].legend(loc="upper left", bbox_to_anchor=(1.01, 1))
im = axs[2].imshow(np.arange(16).reshape(4, 4))
fig.colorbar(im, ax=axs[2])
fig.tight_layout()
fig.savefig("tight_all.png", dpi=50, bbox_inches="tight")
print([round(v, 3) for v in axs[1].get_position().bounds])
