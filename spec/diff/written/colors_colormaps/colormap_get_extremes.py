# Inspect colormaps: N, under/over/bad, call with bytes, alpha, and an array of values.
import numpy as np
import matplotlib as mpl

cmap = mpl.colormaps["viridis"]
print(cmap.N, cmap.name, cmap.is_gray())
print(np.round(cmap(0.0), 4), np.round(cmap(1.0), 4))
print(cmap([0.0, 0.5, 1.0], bytes=True))
print(np.round(cmap(0.25, alpha=0.5), 4))
print(np.round(cmap.get_bad(), 3), np.round(cmap.get_under(), 4), np.round(cmap.get_over(), 4))
custom = cmap.with_extremes(under="k", over="w", bad="r")
print(custom(np.array([-1.0, 2.0, np.nan])))
print(np.round(mpl.colormaps["gray"](np.linspace(0, 1, 3)), 3))
seq = mpl.color_sequences["tab10"]
print(len(seq), np.round(seq[0], 4))
import matplotlib.pyplot as plt
fig, ax = plt.subplots(figsize=(4, 1))
ax.imshow(np.linspace(-0.2, 1.2, 50)[None, :], cmap=custom, vmin=0, vmax=1, aspect="auto")
fig.savefig("extremes.png")
