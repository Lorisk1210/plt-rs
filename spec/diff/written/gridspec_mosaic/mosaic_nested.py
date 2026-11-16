# Nested mosaic: one big map-like panel and a nested 2x2 block for details, with gridspec_kw spacing.
import numpy as np
import matplotlib.pyplot as plt

inner = [["d1", "d2"], ["d3", "d4"]]
outer = [["main", inner]]
fig, axd = plt.subplot_mosaic(outer, figsize=(9, 4), gridspec_kw={"wspace": 0.3})
yy, xx = np.mgrid[0:1:60j, 0:1:60j]
axd["main"].imshow(np.sin(8 * xx) * np.cos(6 * yy), origin="lower", cmap="RdBu")
axd["main"].set_title("overview")
for i, key in enumerate(["d1", "d2", "d3", "d4"], start=1):
    axd[key].plot(np.sin(np.linspace(0, i * np.pi, 50)))
    axd[key].set_title(key, fontsize=8)
    axd[key].tick_params(labelsize=6)
print(len(axd))
fig.savefig("nested_mosaic.png")
