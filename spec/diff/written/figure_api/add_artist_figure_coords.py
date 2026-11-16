# Figure-level artists in figure coordinates: a banner rectangle, a logo-like circle and a footer line.
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle
from matplotlib.lines import Line2D

fig, ax = plt.subplots(figsize=(6, 4))
fig.subplots_adjust(top=0.8)
ax.plot([1, 3, 2])
fig.add_artist(Rectangle((0, 0.88), 1, 0.12, transform=fig.transFigure, color="#2b4c7e"))
fig.text(0.02, 0.94, "Quarterly report", color="white", fontsize=14, va="center")
fig.add_artist(Circle((0.95, 0.94), 0.03, transform=fig.transFigure, color="gold"))
fig.add_artist(Line2D([0.05, 0.95], [0.03, 0.03], transform=fig.transFigure, color="0.5"))
print(len(fig.patches), len(fig.lines), len(fig.texts))
fig.savefig("banner.png")
