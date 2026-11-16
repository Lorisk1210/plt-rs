# Data drawn over the spines and ticks (clip_on=False, high zorder) for markers at the axis limits.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([0, 1, 2, 3], [0, 3, 1, 3], "o-", ms=12, clip_on=False, zorder=10)
ax.set_xlim(0, 3)
ax.set_ylim(0, 3)
ax.spines[["top", "right"]].set_visible(False)
print(ax.lines[0].get_clip_on(), ax.spines["left"].get_zorder())
fig.savefig("over_spines.png")
fig.savefig("over_spines_small.png", dpi=50)
fig.savefig("spines_and_data_order_small.png", dpi=40)
print(fig.get_size_inches(), len(fig.axes))
