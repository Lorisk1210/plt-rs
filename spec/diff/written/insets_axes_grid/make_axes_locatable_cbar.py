# Colour bar exactly as tall as the image with make_axes_locatable and append_axes.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.axes_grid1 import make_axes_locatable

img = np.add.outer(np.linspace(0, 3, 40), np.sin(np.linspace(0, 6, 60)))
fig, ax = plt.subplots(figsize=(6, 3.5))
im = ax.imshow(img, cmap="viridis")
divider = make_axes_locatable(ax)
cax = divider.append_axes("right", size="5%", pad=0.08)
fig.colorbar(im, cax=cax)
top = divider.append_axes("top", size="25%", pad=0.1, sharex=ax)
top.plot(img.mean(axis=0))
top.tick_params(labelbottom=False)
print(len(fig.axes))
fig.savefig("locatable.png")
