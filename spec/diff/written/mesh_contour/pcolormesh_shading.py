# pcolormesh with cell edges vs centres: shading 'flat', 'nearest' and 'gouraud'.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(0, 6)
y = np.arange(0, 4)
z = np.arange(20).reshape(4, 5).astype(float)
xe = np.arange(0, 6) - 0.5
ye = np.arange(0, 5) - 0.5
fig, axs = plt.subplots(1, 3, figsize=(11, 3))
axs[0].pcolormesh(xe, ye, z, shading="flat", edgecolors="k", linewidth=0.5)
axs[0].set_title("flat (edges)")
axs[1].pcolormesh(x[:5], y, z, shading="nearest")
axs[1].set_title("nearest (centres)")
axs[2].pcolormesh(x[:5], y, z, shading="gouraud", cmap="magma")
axs[2].set_title("gouraud")
for ax in axs:
    print([round(v, 2) for v in ax.get_xlim()])
fig.savefig("shading.png")
