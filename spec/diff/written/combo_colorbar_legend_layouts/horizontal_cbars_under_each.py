# Each panel with its own horizontal colour bar below it, different norms, labels, and ticks formatted differently.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm, CenteredNorm

y, x = np.mgrid[0.01:1:50j, 0.01:1:50j]
fields = [(x * y, None, "{x:.2f}"), (10 ** (4 * x), LogNorm(), None), (x - y, CenteredNorm(), "{x:+.1f}")]
fig, axs = plt.subplots(1, 3, figsize=(13, 4.5))
for ax, (f, norm, fmt) in zip(axs, fields):
    im = ax.imshow(f, norm=norm, cmap="cividis" if norm is None else "RdBu_r")
    cb = fig.colorbar(im, ax=ax, orientation="horizontal", pad=0.08, format=fmt)
    cb.set_label(type(norm).__name__ if norm else "linear")
fig.tight_layout()
fig.canvas.draw()
print([[t.get_text() for t in fig.axes[k].get_xticklabels()][:3] for k in (3, 4, 5)])
fig.savefig("hcbars.png", dpi=50)
