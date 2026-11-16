# Image with a physical extent and a numeric aspect so pixels are 2x taller than wide.
import numpy as np
import matplotlib.pyplot as plt

data = np.add.outer(np.linspace(0, 1, 20), np.linspace(0, 2, 40))
fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
axs[0].imshow(data, extent=(0, 40, 0, 20))
axs[0].set_title("equal (default)")
axs[1].imshow(data, extent=(0, 40, 0, 20), aspect=2)
axs[1].set_title("aspect=2")
axs[2].imshow(data, extent=(0, 40, 0, 20), aspect="auto")
axs[2].set_title("auto")
for ax in axs:
    print(ax.get_aspect(), [round(v, 3) for v in ax.get_position().bounds])
fig.savefig("aspects.png")
