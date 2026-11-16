# Discrete colour bar with extensions for a categorised field, and a legend of category patches reading the same colours.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.colors import BoundaryNorm
from matplotlib.patches import Patch

y, x = np.mgrid[0:1:60j, 0:1:60j]
z = (x + y) * 60 - 10
bounds = [0, 20, 40, 60, 80, 100]
cmap = plt.get_cmap("YlOrBr", len(bounds) - 1).with_extremes(under="lightblue", over="darkred")
norm = BoundaryNorm(bounds, cmap.N)
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 4.5))
im = a.imshow(z, cmap=cmap, norm=norm, origin="lower")
cb = fig.colorbar(im, ax=a, extend="both", ticks=bounds, spacing="proportional")
b.axis("off")
b.legend(handles=[Patch(color=cmap(norm((lo + hi) / 2)), label=f"{lo}–{hi}") for lo, hi in zip(bounds[:-1], bounds[1:])]
         + [Patch(color=cmap.get_under(), label="< 0"), Patch(color=cmap.get_over(), label="> 100")], loc="center")
print(cb.extend, cb.get_ticks().tolist(), np.round(cmap.get_over(), 3).tolist())
fig.savefig("discrete.png", dpi=50)
