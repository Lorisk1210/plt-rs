# Hill-shaded terrain with LightSource.shade and blend modes.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LightSource

y, x = np.mgrid[-3:3:200j, -3:3:200j]
z = np.sin(x * 1.5) * np.cos(y) + 0.5 * np.exp(-(x ** 2 + y ** 2))
ls = LightSource(azdeg=315, altdeg=45)
fig, axs = plt.subplots(1, 3, figsize=(10, 3.5))
axs[0].imshow(ls.hillshade(z, vert_exag=2), cmap="gray")
axs[0].set_title("hillshade")
axs[1].imshow(ls.shade(z, cmap=plt.cm.gist_earth, blend_mode="overlay", vert_exag=2))
axs[1].set_title("overlay")
axs[2].imshow(ls.shade(z, cmap=plt.cm.gist_earth, blend_mode="soft"))
axs[2].set_title("soft")
for ax in axs:
    ax.axis("off")
print(round(float(ls.hillshade(z).mean()), 4))
fig.savefig("terrain.png")
