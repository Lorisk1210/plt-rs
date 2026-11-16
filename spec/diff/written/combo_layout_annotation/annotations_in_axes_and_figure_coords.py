# Annotations connecting two subplots (ConnectionPatch), a figure-level arrow and texts in various coordinate systems.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import ConnectionPatch, FancyArrowPatch

x = np.linspace(0, 10, 100)
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4))
a.plot(x, np.sin(x))
b.plot(x, np.cos(x))
con = ConnectionPatch(xyA=(np.pi / 2, 1), coordsA=a.transData, xyB=(0, 1), coordsB=b.transData, arrowstyle="->", color="r")
fig.add_artist(con)
fig.add_artist(FancyArrowPatch((0.45, 0.05), (0.55, 0.05), transform=fig.transFigure, arrowstyle="<->", mutation_scale=15))
fig.text(0.5, 0.08, "compare", ha="center")
a.annotate("peak", xy=(np.pi / 2, 1), xytext=(0.5, 0.5), textcoords="axes fraction", arrowprops=dict(arrowstyle="->"))
b.annotate("start", xy=(0, 1), xytext=(-40, -30), textcoords="offset points", arrowprops=dict(arrowstyle="-"))
print(len(fig.artists) if hasattr(fig, "artists") else None, len(fig.patches), len(fig.texts))
fig.savefig("connections.png", dpi=50)
