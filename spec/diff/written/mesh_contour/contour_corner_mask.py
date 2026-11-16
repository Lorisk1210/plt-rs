# Contour of a field with masked values (a hole) and the corner_mask option.
import numpy as np
import matplotlib.pyplot as plt

x, y = np.meshgrid(np.arange(7), np.arange(10))
z = np.sin(0.5 * x) * np.cos(0.52 * y)
mask = np.zeros_like(z, dtype=bool)
mask[2, 3:5] = True
mask[3:5, 4] = True
mask[7, 2] = True
z = np.ma.array(z, mask=mask)
fig, axs = plt.subplots(1, 2, figsize=(8, 4))
for ax, corner in zip(axs, [False, True]):
    cs = ax.contourf(x, y, z, corner_mask=corner)
    ax.contour(cs, colors="k")
    ax.set_title(f"corner_mask={corner}")
    ax.plot(np.ma.array(x, mask=~mask), y, "ro")
fig.savefig("corner_mask.png")
print(int(mask.sum()))
