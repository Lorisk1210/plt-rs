# Remove tick marks and labels in different ways users try (empty lists, NullLocator, tick_params).
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import NullLocator, NullFormatter

img = np.arange(64).reshape(8, 8)
fig, axs = plt.subplots(1, 4, figsize=(10, 2.8))
for ax in axs:
    ax.imshow(img, cmap="gray")
axs[0].set_xticks([])
axs[0].set_yticks([])
axs[1].xaxis.set_major_locator(NullLocator())
axs[1].yaxis.set_major_formatter(NullFormatter())
axs[2].tick_params(left=False, bottom=False, labelleft=False, labelbottom=False)
axs[3].axis("off")
for i, ax in enumerate(axs):
    ax.set_title(f"way {i + 1}")
print(len(axs[1].get_yticks()), axs[3].axison)
fig.savefig("no_ticks.png")
