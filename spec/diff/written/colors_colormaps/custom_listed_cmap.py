# Corporate colormap from a list of hex colours (ListedColormap and LinearSegmentedColormap.from_list).
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap, LinearSegmentedColormap

brand = ["#0b1d51", "#725cad", "#8ccdeb", "#ffe3a9"]
discrete = ListedColormap(brand, name="brand")
smooth = LinearSegmentedColormap.from_list("brand_smooth", brand, N=256)
z = np.linspace(0, 1, 100).reshape(1, -1).repeat(10, 0)
fig, axs = plt.subplots(2, 1, figsize=(6, 2.5))
axs[0].imshow(z, cmap=discrete, aspect="auto")
axs[0].set_title("ListedColormap", fontsize=9)
axs[1].imshow(z, cmap=smooth, aspect="auto")
axs[1].set_title("from_list", fontsize=9)
for ax in axs:
    ax.set_axis_off()
print(discrete.N, smooth.N, np.round(smooth(0.5), 4))
fig.savefig("brand.png")
