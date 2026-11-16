# Log colour bar with minor ticks and a large label, on pcolormesh data spanning six decades.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm

y, x = np.mgrid[1:2:60j, 1:2:60j]
z = 10 ** (6 * (x - 1) * (y - 1))
fig, ax = plt.subplots(figsize=(5, 4))
pc = ax.pcolormesh(z, norm=LogNorm(vmin=1, vmax=1e6), cmap="inferno")
cb = fig.colorbar(pc, ax=ax)
cb.set_label("concentration", fontsize=13)
cb.ax.minorticks_on()
fig.canvas.draw()
print([t.get_text() for t in cb.ax.get_yticklabels()])
fig.savefig("log_bar.png")
