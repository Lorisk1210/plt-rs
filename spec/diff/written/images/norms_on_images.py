# The same image under Normalize, LogNorm, PowerNorm and CenteredNorm.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib import colors

y, x = np.mgrid[-2:2:100j, -2:2:100j]
z = np.exp(-(x ** 2 + y ** 2)) * 1000 + 1
fig, axs = plt.subplots(1, 4, figsize=(12, 3))
norms = [colors.Normalize(), colors.LogNorm(), colors.PowerNorm(gamma=0.3), colors.CenteredNorm(vcenter=500)]
for ax, norm in zip(axs, norms):
    im = ax.imshow(z, norm=norm, cmap="cubehelix")
    fig.colorbar(im, ax=ax, shrink=0.7)
    ax.set_title(type(norm).__name__, fontsize=9)
    ax.axis("off")
print([(round(n.vmin, 3), round(n.vmax, 3)) for n in norms])
fig.savefig("norms.png")
