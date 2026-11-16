# Colour bar with ticks on the left, a formatter, a title on top and no outline.
import numpy as np
import matplotlib.pyplot as plt

z = np.random.default_rng(1).random((10, 10)) * 2000
fig, ax = plt.subplots(figsize=(5, 4))
im = ax.imshow(z, cmap="viridis")
cb = fig.colorbar(im, ax=ax, format="{x:,.0f} m")
cb.ax.yaxis.set_ticks_position("left")
cb.ax.set_title("depth", fontsize=9)
cb.outline.set_visible(False)
cb.ax.tick_params(length=0, labelsize=8)
fig.canvas.draw()
print([t.get_text() for t in cb.ax.get_yticklabels()][:3])
fig.savefig("depth_bar.png")
