# fig.add_gridspec with sharing between subplots created from it, a common modern idiom.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(7, 5))
gs = fig.add_gridspec(3, 2, hspace=0, wspace=0)
axs = gs.subplots(sharex="col", sharey="row")
x = np.linspace(-1, 1, 100)
for i in range(3):
    for j in range(2):
        axs[i, j].plot(x, np.polynomial.legendre.legval(x, [0] * (2 * i + j) + [1]))
        axs[i, j].text(0.05, 0.85, f"P{2 * i + j}", transform=axs[i, j].transAxes)
for ax in axs.flat:
    ax.label_outer()
print(axs.shape)
fig.savefig("legendre.png")
