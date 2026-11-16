# Diverging anomaly map centred on zero with TwoSlopeNorm and an asymmetric range.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import TwoSlopeNorm

y, x = np.mgrid[0:1:40j, 0:1:60j]
anomaly = 6 * x - 2 + np.sin(9 * y)
fig, ax = plt.subplots(figsize=(6, 3.5))
norm = TwoSlopeNorm(vmin=anomaly.min(), vcenter=0, vmax=anomaly.max())
pc = ax.pcolormesh(anomaly, cmap="RdBu_r", norm=norm)
cb = fig.colorbar(pc, ax=ax, extend="neither")
cb.set_label("anomaly")
print(round(norm.vmin, 3), norm.vcenter, round(norm.vmax, 3))
print(np.round(cb.get_ticks(), 2))
fig.savefig("anomaly.png")
