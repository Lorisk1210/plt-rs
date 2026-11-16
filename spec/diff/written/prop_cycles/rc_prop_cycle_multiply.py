# Global prop cycle via rcParams with a product of cyclers (colours x markers).
import numpy as np
import matplotlib.pyplot as plt
from cycler import cycler

plt.rcParams["axes.prop_cycle"] = cycler(marker=["o", "s"]) * cycler(color=["#e66101", "#5e3c99", "#1b9e77"])
fig, ax = plt.subplots(figsize=(6, 4))
for k in range(6):
    ax.plot(np.arange(5), np.arange(5) * (k + 1), ms=5, label=str(k))
ax.legend(ncols=2)
print([(l.get_marker(), l.get_color()) for l in ax.lines])
fig.savefig("product.png")
ax.set_title("markers times colours")
fig.savefig("product_small.png", dpi=50)
