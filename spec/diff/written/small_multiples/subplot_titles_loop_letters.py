# Panel letters (a), (b), ... placed at the top-left of each panel in figure-friendly positions.
import string
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.transforms as mtransforms

fig, axs = plt.subplot_mosaic([["a", "b"], ["c", "c"]], figsize=(7, 5), layout="constrained")
x = np.linspace(0, 1, 50)
axs["a"].plot(x, x ** 2)
axs["b"].scatter(x, np.sin(6 * x), s=5)
axs["c"].bar(range(6), [3, 1, 4, 1, 5, 9])
for label, ax in axs.items():
    trans = mtransforms.ScaledTranslation(-20 / 72, 7 / 72, fig.dpi_scale_trans)
    ax.text(0.0, 1.0, f"({label})", transform=ax.transAxes + trans, fontsize=12, va="bottom", fontweight="bold")
print(sorted(axs), string.ascii_lowercase[:3])
fig.savefig("letters.png")
