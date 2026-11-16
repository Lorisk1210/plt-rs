# A family of lines coloured by a parameter, a colour bar standing for the parameter, and a legend for two reference lines.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.cm import ScalarMappable
from matplotlib.colors import Normalize

x = np.linspace(0, 1, 100)
ks = np.linspace(0.5, 5, 10)
norm = Normalize(ks.min(), ks.max())
cmap = plt.get_cmap("plasma")
fig, ax = plt.subplots(figsize=(7, 4.5))
for k in ks:
    ax.plot(x, x ** k, color=cmap(norm(k)))
r1, = ax.plot(x, x, "k--", label="k = 1")
r2, = ax.plot(x, np.sqrt(x), "k:", label="k = 0.5")
cb = fig.colorbar(ScalarMappable(norm=norm, cmap=cmap), ax=ax, label="exponent k")
ax.legend(handles=[r1, r2], loc="upper left")
print(len(ax.lines), round(cb.vmin, 3), round(cb.vmax, 3))
fig.savefig("family.png", dpi=50)
