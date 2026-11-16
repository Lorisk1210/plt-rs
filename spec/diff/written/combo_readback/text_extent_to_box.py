# Measure a title's window extent after drawing and draw a matching rectangle behind it in figure coordinates.
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch

fig, ax = plt.subplots(figsize=(6, 4))
ax.plot([0, 1, 2], [2, 0, 1])
title = ax.set_title("Quarterly overview", fontsize=16)
fig.canvas.draw()
bb = title.get_window_extent().transformed(fig.transFigure.inverted())
print([round(v, 4) for v in bb.bounds])
fig.add_artist(FancyBboxPatch((bb.x0, bb.y0), bb.width, bb.height, boxstyle="round,pad=0.01",
                              transform=fig.transFigure, fc="lightyellow", ec="k", zorder=-1))
lab = ax.xaxis.label
ax.set_xlabel("time")
fig.canvas.draw()
print([round(v, 1) for v in ax.xaxis.label.get_window_extent().bounds], lab is ax.xaxis.label)
fig.savefig("title_box.png", dpi=50)
