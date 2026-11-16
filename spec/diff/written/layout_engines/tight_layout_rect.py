# tight_layout with a rect that leaves room for a figure legend and a suptitle at the top.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 100)
fig, axs = plt.subplots(1, 3, figsize=(10, 3.5))
for i, ax in enumerate(axs):
    ax.plot(x, np.sin(x + i), label="sin")
    ax.plot(x, np.cos(x + i), label="cos")
    ax.set_title(f"shift {i}")
handles, labels = axs[0].get_legend_handles_labels()
fig.legend(handles, labels, loc="upper center", ncol=2, bbox_to_anchor=(0.5, 0.92))
fig.suptitle("Phase shifts", y=0.99)
fig.tight_layout(rect=[0, 0, 1, 0.85])
print(np.round(axs[0].get_position().y1, 3))
fig.savefig("phases.png")
