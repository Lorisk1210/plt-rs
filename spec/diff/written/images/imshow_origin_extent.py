# Show a 2-D array with origin='lower' and a physical extent, compared with the default.
import numpy as np
import matplotlib.pyplot as plt

z = np.arange(20).reshape(4, 5)
fig, axs = plt.subplots(1, 3, figsize=(10, 3))
axs[0].imshow(z)
axs[0].set_title("default (upper)")
axs[1].imshow(z, origin="lower")
axs[1].set_title("origin=lower")
im = axs[2].imshow(z, origin="lower", extent=[0, 50, 0, 2], aspect="auto")
axs[2].set_title("extent, auto aspect")
for ax in axs:
    print([round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
print(im.get_extent())
fig.savefig("origins.png")
