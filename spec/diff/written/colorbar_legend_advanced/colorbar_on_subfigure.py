# Colour bars attached to subfigures (each subfigure has its own bar).
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(9, 3.5), layout="constrained")
sfs = fig.subfigures(1, 2)
for k, sf in enumerate(sfs):
    axs = sf.subplots(1, 2)
    for ax in axs:
        im = ax.imshow(np.random.default_rng(k).random((8, 8)) * (k + 1), vmin=0, vmax=2, cmap=["Blues", "Reds"][k])
    sf.colorbar(im, ax=axs, shrink=0.8)
    sf.suptitle(f"group {k}")
print([len(sf.axes) for sf in sfs])
fig.savefig("subfig_bars.png")
