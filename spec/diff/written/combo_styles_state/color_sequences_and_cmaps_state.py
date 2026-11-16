# Register a colour sequence and a colormap, use both by name across subplots, and list them back.
import numpy as np
import matplotlib as mpl
import matplotlib.pyplot as plt
from matplotlib.colors import LinearSegmentedColormap

mpl.color_sequences.register("brand5", ["#003f5c", "#58508d", "#bc5090", "#ff6361", "#ffa600"])
mpl.colormaps.register(LinearSegmentedColormap.from_list("brand_grad", ["#003f5c", "#ffa600"]))
fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
axs[0].set_prop_cycle(color=mpl.color_sequences["brand5"])
for k in range(5):
    axs[0].plot([0, 1], [k, k + 1])
axs[1].imshow(np.linspace(0, 1, 100)[None, :].repeat(10, 0), cmap="brand_grad", aspect="auto")
axs[2].scatter(range(10), range(10), c=range(10), cmap="brand_grad")
print([l.get_color() for l in axs[0].lines], "brand5" in mpl.color_sequences, "brand_grad" in mpl.colormaps)
print(np.round(mpl.colormaps["brand_grad"](0.5), 4).tolist())
fig.savefig("brand.png", dpi=50)
