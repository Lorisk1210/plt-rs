# Clip an image and a line to a circular patch with set_clip_path.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Circle

y, x = np.mgrid[0:1:200j, 0:1:200j]
img = np.sin(10 * x) + np.cos(10 * y)
fig, ax = plt.subplots(figsize=(4.5, 4.5))
im = ax.imshow(img, extent=(0, 1, 0, 1), cmap="Spectral")
clip = Circle((0.5, 0.5), 0.4, transform=ax.transData)
im.set_clip_path(clip)
line, = ax.plot([0, 1], [0, 1], lw=6, color="k")
line.set_clip_path(Circle((0.5, 0.5), 0.3, transform=ax.transData))
ax.set_title("clipped")
print(im.get_clip_path() is not None)
fig.savefig("clipped.png")
