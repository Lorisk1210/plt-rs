# LogNorm with non-positive vmin, vmin > vmax, and both norm and vmin given: the errors.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm, Normalize

data = np.arange(1, 17).reshape(4, 4).astype(float)
fig, ax = plt.subplots(figsize=(4, 3))
for norm, kw in [(LogNorm(vmin=0, vmax=10), {}), (Normalize(vmin=5, vmax=1), {}), (Normalize(), {"vmin": 2})]:
    try:
        im = ax.imshow(data, norm=norm, **kw)
        fig.canvas.draw()
        print(type(norm).__name__, "drawn")
    except ValueError as err:
        print(type(norm).__name__, "ValueError:", str(err)[:90])
for leftover in list(ax.images):
    leftover.remove()
ax.imshow(data, norm=LogNorm())
fig.savefig("norms.png")
