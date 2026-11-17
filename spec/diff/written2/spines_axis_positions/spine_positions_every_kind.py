# Spine set_position with outward, axes, data, zero and center; spine paths in display coordinates and tick label placement.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 3, figsize=(8, 5), dpi=50)
positions = [("outward", 10), ("axes", 0.3), ("data", 2), "zero", "center", ("outward", -5)]
for ax, pos in zip(axs.flat, positions):
    ax.set_xlim(-3, 5)
    ax.set_ylim(-1, 4)
    ax.spines["left"].set_position(pos)
    ax.spines["bottom"].set_position(pos)
    ax.spines[["top", "right"]].set_visible(False)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for ax, pos in zip(axs.flat, positions):
    left = ax.spines["left"]
    path = left.get_path().vertices
    disp = left.get_transform().transform(path)
    print(pos, left.get_position(), [round(v, 1) for v in disp[0]], [round(v, 1) for v in ax.yaxis.get_ticklabels()[1].get_window_extent(r).bounds][:2])
print(type(axs[0, 0].spines).__name__, sorted(axs[0, 0].spines.keys()))
try:
    axs[0, 0].spines["left"].set_position(("sideways", 1))
except ValueError as e:
    print("ValueError", str(e)[:60])
print(axs[0, 0].spines.left is axs[0, 0].spines["left"], type(axs[0, 0].spines[:]).__name__)
axs[0, 0].spines[:].set_linewidth(2)
print([s.get_linewidth() for s in axs[0, 0].spines.values()])
print(axs[0, 0].spines["left"].get_spine_transform() is not None, axs[0, 0].spines["left"].axis is axs[0, 0].yaxis)
fig.savefig("spine_positions.png")
