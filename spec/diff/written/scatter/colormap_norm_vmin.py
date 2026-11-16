# Scatter coloured with vmin/vmax clipping and a reversed colormap, plus plot() with markers only.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 200)
y = np.sin(x) + 0.1 * x
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
sc = a.scatter(x, y, c=y, cmap="RdYlBu_r", vmin=-0.5, vmax=1.5, s=12)
fig.colorbar(sc, ax=a, extend="both")
a.set_title("scatter")
b.plot(x, y, "o", ms=3, mfc="none", mec="purple")
b.set_title("plot with markers")
print(sc.get_clim(), sc.cmap.name)
fig.savefig("clipped_colors.png")
