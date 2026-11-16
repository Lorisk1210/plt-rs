# Hatched uncertainty regions with fill_between (no fill colour, hatch only) for print.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 100)
y = np.sqrt(x)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(x, y, "k")
ax.fill_between(x, y - 0.3, y + 0.3, facecolor="none", hatch="////", edgecolor="0.4", lw=0)
ax.fill_between(x, y + 0.3, y + 0.6, facecolor="none", hatch="..", edgecolor="0.6", lw=0)
ax.set_title("hatched bands")
print(ax.collections[0].get_hatch(), ax.collections[1].get_hatch())
fig.savefig("hatched_band.png")
fig.savefig("hatched_band.pdf", metadata={"CreationDate": None})
