# Grid of square images with 'compressed' layout so there is no white gap between them.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 3, figsize=(7, 4), layout="compressed")
for k, ax in enumerate(axs.flat):
    img = np.add.outer(np.arange(16), np.arange(16) * (k + 1)) % 7
    im = ax.imshow(img, cmap="cividis")
    ax.set_axis_off()
    ax.set_title(f"pattern {k}", fontsize=9)
fig.colorbar(im, ax=axs, shrink=0.8)
print(fig.get_layout_engine().__class__.__name__)
fig.savefig("patterns.png")
fig.suptitle("Modular patterns")
fig.savefig("patterns_titled.png", dpi=80)
