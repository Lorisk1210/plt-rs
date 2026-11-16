# Long axis labels overlap in a 2x2 grid; tight_layout with padding fixes them before saving.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 2, figsize=(6, 5))
for ax in axs.flat:
    ax.plot(np.arange(10), np.arange(10) ** 1.5)
    ax.set_xlabel("elapsed time since start of measurement [s]")
    ax.set_ylabel("cumulative energy [kJ]")
    ax.set_title("A rather long title for a small panel")
fig.tight_layout(pad=1.2, h_pad=1.0, w_pad=0.8)
print([np.round(ax.get_position().bounds, 3).tolist() for ax in axs.flat])
fig.savefig("tight.png")
fig.suptitle("Energy over time")
fig.tight_layout()
fig.savefig("tight_suptitle.png")
