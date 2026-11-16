# Colour bar label on the left, rotated, ticks on the left, a title above, and a second bar with label on the right.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:30j, 0:1:30j]
fig, axs = plt.subplots(1, 2, figsize=(10, 4))
im1 = axs[0].imshow(x * y)
cb1 = fig.colorbar(im1, ax=axs[0], location="left")
cb1.set_label("left label", rotation=90, labelpad=5)
cb1.ax.set_title("title", fontsize=8)
im2 = axs[1].imshow(x - y, cmap="coolwarm")
cb2 = fig.colorbar(im2, ax=axs[1])
cb2.set_label("right label (rotated)", rotation=270, labelpad=15)
cb2.ax.yaxis.set_label_position("right")
fig.canvas.draw()
print(cb1.ax.yaxis.get_label_position(), cb1.ax.yaxis.get_ticks_position(), cb2.ax.yaxis.label.get_rotation())
fig.savefig("cbar_labels.png", dpi=50)
