# Colour bar in a manually placed axes (cax) to the right, same height as the image axes.
import numpy as np
import matplotlib.pyplot as plt

z = np.fromfunction(lambda i, j: np.sin(i / 5) + np.cos(j / 7), (60, 80))
fig = plt.figure(figsize=(6, 4))
ax = fig.add_axes([0.08, 0.1, 0.72, 0.8])
cax = fig.add_axes([0.83, 0.1, 0.03, 0.8])
im = ax.imshow(z, aspect="auto", cmap="twilight")
cb = fig.colorbar(im, cax=cax)
cb.ax.set_title("level", fontsize=8)
cb.outline.set_linewidth(0.5)
cb.ax.tick_params(labelsize=7)
print(cax.get_position().bounds)
fig.savefig("cax.png")
