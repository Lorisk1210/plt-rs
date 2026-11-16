# Update an existing image's data and colour limits (set_data, set_clim, autoscale) between saves.
import numpy as np
import matplotlib.pyplot as plt

frames = [np.outer(np.sin(np.linspace(0, k, 30)), np.cos(np.linspace(0, k, 30))) for k in (2, 4, 8)]
fig, ax = plt.subplots(figsize=(4, 3.5))
im = ax.imshow(frames[0], cmap="coolwarm")
cb = fig.colorbar(im)
fig.savefig("frame0.png")
im.set_data(frames[1] * 3)
im.autoscale()
fig.savefig("frame1.png")
im.set_data(frames[2])
im.set_clim(-0.5, 0.5)
im.set_cmap("PuOr")
fig.savefig("frame2.png")
print(im.get_clim(), im.get_array().shape, cb.vmin)
