# Colour bar following its mappable through set_clim, set_cmap, set_norm and set_array; read ticks after each change.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm, BoundaryNorm

fig, ax = plt.subplots(figsize=(5, 4))
pc = ax.pcolormesh(np.arange(1, 13).reshape(3, 4).astype(float))
cb = fig.colorbar(pc)
log = []
def snap(tag):
    fig.canvas.draw()
    log.append((tag, [t.get_text() for t in cb.ax.get_yticklabels()][:4], round(cb.vmin, 3), round(cb.vmax, 3)))
snap("start")
pc.set_clim(0, 20); snap("clim")
pc.set_cmap("magma"); snap("cmap")
pc.set_norm(LogNorm(1, 12)); snap("log")
pc.set_norm(BoundaryNorm([1, 4, 8, 12], 256)); snap("boundary")
pc.set_array(np.arange(12).reshape(3, 4) * 2.0); snap("array")
for entry in log:
    print(entry)
fig.savefig("colorbar_follow.png", dpi=50)
