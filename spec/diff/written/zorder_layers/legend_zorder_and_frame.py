# Legend drawn above an annotation box and a figure-level line drawn with fig.add_artist.
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([0, 1], [0, 1], label="diag")
ax.text(0.05, 0.85, "box under legend", transform=ax.transAxes, bbox=dict(fc="lightgreen"), zorder=4)
leg = ax.legend(loc="upper left")
leg.set_zorder(5)
fig.add_artist(Line2D([0.1, 0.9], [0.05, 0.05], color="red", lw=3, transform=fig.transFigure))
print(leg.get_zorder(), len(fig.artists) if hasattr(fig, "artists") else None, len(fig.lines))
fig.savefig("legend_layer.png")
fig.savefig("legend_zorder_and_frame_small.png", dpi=40)
print(fig.get_size_inches(), len(fig.axes))
