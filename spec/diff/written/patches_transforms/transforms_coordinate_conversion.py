# Convert between data, axes, figure and display coordinates to place a marker by pixels.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4), dpi=100)
ax.plot([0, 10], [0, 100])
fig.canvas.draw()
display = ax.transData.transform((5, 50))
back = ax.transData.inverted().transform(display)
axes_frac = ax.transAxes.inverted().transform(display)
fig_frac = fig.transFigure.inverted().transform(display)
print(np.round(display, 2), np.round(back, 6), np.round(axes_frac, 4), np.round(fig_frac, 4))
ax.plot(*ax.transData.inverted().transform(display + [20, 20]), "r*", ms=15)
ax.annotate("20 px up-right", xy=(5, 50), xytext=(20, 20), textcoords="offset pixels")
fig.savefig("coords.png")
