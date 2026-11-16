# Pie placed off-centre with radius and center, frame=True and axis limits shown.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
ax.pie([3, 2, 1], center=(1, 1), radius=0.8, frame=True, labels=["a", "b", "c"])
ax.pie([1, 1], center=(3, 1.5), radius=0.5, frame=True, colors=["k", "0.8"])
ax.set_xlim(0, 4)
ax.set_ylim(0, 2.5)
ax.set_aspect("equal")
print(ax.axison, [round(v, 3) for v in ax.get_xlim()])
fig.savefig("frame_pie.png")
fig.savefig("frame_pie.svg")
fig.savefig("pie_radius_frame_80dpi.png", dpi=80)
print(fig.get_size_inches(), len(fig.axes))
plt.close(fig)
