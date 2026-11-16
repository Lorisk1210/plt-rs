# Change an image after creation: data, extent, origin-like flip via extent, interpolation, alpha and norm; read back.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm

fig, ax = plt.subplots(figsize=(5, 4))
im = ax.imshow(np.arange(12).reshape(3, 4))
im.set_data(np.arange(1, 21).reshape(4, 5).astype(float))
im.set_extent((0, 10, 0, 8))
im.set_interpolation("bilinear")
im.set_alpha(0.8)
im.set_norm(LogNorm(1, 20))
fig.canvas.draw()
print(im.get_array().shape, im.get_extent(), im.get_interpolation(), im.get_alpha(), type(im.norm).__name__, im.get_clim())
print([round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
im.set_extent((10, 0, 8, 0))
fig.canvas.draw()
print([round(v, 3) for v in ax.get_xlim()], ax.xaxis_inverted(), ax.yaxis_inverted())
fig.savefig("image_mutations.png", dpi=50)
