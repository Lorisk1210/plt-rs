# Positional-argument call styles of the old APIs: plot with many x/y/fmt groups, fill with several polygons, subplot codes, axis lists.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

x = np.arange(5)
fig = plt.figure(figsize=(6, 4))
ax = fig.add_subplot(221)
lines = ax.plot(x, x, "ro", x, x ** 2, "g^-", x, x ** 0.5)
print(len(lines), [(l.get_color(), l.get_marker(), l.get_linestyle()) for l in lines])
ax2 = fig.add_subplot(2, 2, 2)
polys = ax2.fill([0, 1, 1], [0, 0, 1], "r", [2, 3, 3], [0, 0, 1], "b", alpha=0.5)
print(len(polys), [p.get_facecolor() for p in polys])
ax3 = plt.subplot(223)
only_y = ax3.plot([3, 1, 2])
print(only_y[0].get_xdata().tolist())
ax3.axis([0, 3, 0, 4])
print(ax3.get_xlim(), ax3.get_ylim())
ax4 = fig.add_subplot(224)
ax4.plot(x, "k--")
ax4.plot(x, x[::-1], "C3")
ax4.errorbar(x, x, 0.2, 0.1, "o")
print([l.get_linestyle() for l in ax4.lines][:2], ax4.lines[1].get_color())
try:
    ax4.plot(x, x, "q-")
except ValueError as e:
    print("ValueError", str(e)[:60])
try:
    ax4.plot(x, x, x)
except ValueError as e:
    print("ValueError", str(e)[:60])
print(ax.axis("off"), ax.axison)
print(ax2.axis("equal"), ax2.get_aspect())
print(ax3.axis("tight"))
print(ax4.axis("scaled"))
fig.savefig("positional.png")
