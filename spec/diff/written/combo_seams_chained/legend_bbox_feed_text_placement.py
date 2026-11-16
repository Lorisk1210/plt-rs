# Measure a legend's bounding box after drawing and place a caption text right under it in axes coordinates.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
for k in range(3):
    ax.plot(np.arange(5), np.arange(5) * (k + 1), label=f"rate {k + 1}")
leg = ax.legend(loc="upper left")
fig.canvas.draw()
bb = leg.get_window_extent().transformed(ax.transAxes.inverted())
ax.text(bb.x0, bb.y0 - 0.02, "rates per hour", transform=ax.transAxes, va="top", fontsize=8, style="italic")
tb = ax.texts[0]
fig.canvas.draw()
print([round(v, 3) for v in bb.bounds], [round(v, 1) for v in tb.get_window_extent().bounds][:2])
fig.savefig("caption.png", dpi=50)
